from __future__ import annotations

from bisect import bisect_left
from collections import defaultdict
from collections.abc import Iterable
from dataclasses import dataclass
from math import exp, floor, hypot, tanh
from statistics import mean, median
from typing import Any

from .models import (
    CM_PER_METER,
    CellAccumulator,
    CircleState,
    CombatTotals,
    MovementSegment,
    TeamSnapshot,
    Vec3,
)
from .timeline import EventClock, PhaseTimeline


@dataclass(frozen=True, slots=True)
class _MemberObservation:
    elapsed_s: float
    position: Vec3
    in_vehicle: bool


def build_team_snapshots(
    events: list[dict[str, Any]],
    clock: EventClock,
    phases: PhaseTimeline,
    circles: list[CircleState],
    *,
    bucket_seconds: float,
    cell_size_m: float,
    match_end_s: float,
) -> list[TeamSnapshot]:
    grouped: dict[tuple[int, int], dict[str, _MemberObservation]] = defaultdict(dict)

    for index, event in enumerate(events):
        if event.get("_T") != "LogPlayerPosition":
            continue
        elapsed_s = clock.elapsed(event)
        character = event.get("character")
        if elapsed_s is None or elapsed_s < 0 or not isinstance(character, dict):
            continue
        team_id = _integer(character.get("teamId"))
        position = Vec3.from_mapping(character.get("location"))
        if team_id is None or position is None:
            continue

        player_key = character.get("accountId") or character.get("name") or f"event-{index}"
        bucket = int(floor(elapsed_s / bucket_seconds))
        vehicle = event.get("vehicle")
        in_vehicle = bool(character.get("isInVehicle")) or (
            isinstance(vehicle, dict) and vehicle.get("vehicleType") not in {None, "", "None"}
        )
        observation = _MemberObservation(elapsed_s, position, in_vehicle)
        previous = grouped[(bucket, team_id)].get(str(player_key))
        if previous is None or previous.elapsed_s <= elapsed_s:
            grouped[(bucket, team_id)][str(player_key)] = observation

    circle_by_phase = {circle.phase: circle for circle in circles}
    ordered_circle_phases = sorted(circle_by_phase)
    cell_size_cm = cell_size_m * CM_PER_METER
    snapshots: list[TeamSnapshot] = []

    for (bucket, team_id), member_map in sorted(grouped.items()):
        members = list(member_map.values())
        if not members:
            continue
        elapsed_s = float(median(member.elapsed_s for member in members))
        center = Vec3(
            float(median(member.position.x_cm for member in members)),
            float(median(member.position.y_cm for member in members)),
            float(median(member.position.z_cm for member in members)),
        )
        spread_m = max(
            (center.distance_2d_cm(member.position) / CM_PER_METER for member in members),
            default=0.0,
        )
        phase = phases.phase_at(elapsed_s)
        snapshot = TeamSnapshot(
            bucket=bucket,
            elapsed_s=elapsed_s,
            phase=phase,
            team_id=team_id,
            center=center,
            member_count=len(members),
            spread_m=spread_m,
            vehicle_member_count=sum(member.in_vehicle for member in members),
            cell_x=int(floor(center.x_cm / cell_size_cm)),
            cell_y=int(floor(center.y_cm / cell_size_cm)),
        )
        _add_zone_features(snapshot, circle_by_phase, ordered_circle_phases)
        snapshots.append(snapshot)

    _add_enemy_density(snapshots)
    _add_future_labels(snapshots, match_end_s=match_end_s)
    return snapshots


def build_movement_segments(
    snapshots: list[TeamSnapshot], *, bucket_seconds: float
) -> list[MovementSegment]:
    by_team: dict[int, list[TeamSnapshot]] = defaultdict(list)
    for snapshot in snapshots:
        by_team[snapshot.team_id].append(snapshot)

    segments: list[MovementSegment] = []
    maximum_gap_s = bucket_seconds * 3.5
    for team_id, team_snapshots in by_team.items():
        team_snapshots.sort(key=lambda item: item.elapsed_s)
        for start, end in zip(team_snapshots, team_snapshots[1:], strict=False):
            duration_s = end.elapsed_s - start.elapsed_s
            if duration_s <= 0 or duration_s > maximum_gap_s:
                continue
            distance_m = start.center.distance_2d_cm(end.center) / CM_PER_METER
            segments.append(
                MovementSegment(
                    team_id=team_id,
                    phase=end.phase,
                    started_at_s=start.elapsed_s,
                    ended_at_s=end.elapsed_s,
                    start_cell_id=start.cell_id,
                    end_cell_id=end.cell_id,
                    straight_distance_m=distance_m,
                    duration_s=duration_s,
                    speed_mps=distance_m / duration_s,
                    used_vehicle=(start.vehicle_member_count > 0 or end.vehicle_member_count > 0),
                )
            )
    return segments


def aggregate_phase_cells(
    events: list[dict[str, Any]],
    snapshots: list[TeamSnapshot],
    clock: EventClock,
    phases: PhaseTimeline,
    *,
    bucket_seconds: float,
    cell_size_m: float,
) -> list[dict[str, Any]]:
    accumulators: dict[tuple[int, int, int], CellAccumulator] = {}
    for snapshot in snapshots:
        key = (snapshot.phase, snapshot.cell_x, snapshot.cell_y)
        accumulator = accumulators.setdefault(
            key,
            CellAccumulator(
                phase=snapshot.phase,
                cell_x=snapshot.cell_x,
                cell_y=snapshot.cell_y,
            ),
        )
        accumulator.samples.append(snapshot)
        accumulator.unique_teams.add(snapshot.team_id)

    _add_hold_durations(accumulators, snapshots, bucket_seconds=bucket_seconds)
    combat = _collect_combat(events, clock, phases, cell_size_m=cell_size_m)
    for key, totals in combat.items():
        if key not in accumulators:
            phase, cell_x, cell_y = key
            accumulators[key] = CellAccumulator(phase=phase, cell_x=cell_x, cell_y=cell_y)
        accumulators[key].combat = totals

    elevation = _build_elevation_map(snapshots, cell_size_m=cell_size_m)
    cells = [
        _cell_to_dict(accumulator, elevation, cell_size_m=cell_size_m)
        for accumulator in accumulators.values()
        if accumulator.samples
    ]
    cells.sort(
        key=lambda cell: (cell["historical_value_score"], cell["occupancy_samples"]),
        reverse=True,
    )
    return cells


def _add_zone_features(
    snapshot: TeamSnapshot,
    circle_by_phase: dict[int, CircleState],
    ordered_circle_phases: list[int],
) -> None:
    current = circle_by_phase.get(snapshot.phase)
    if current is None:
        eligible = [phase for phase in ordered_circle_phases if phase <= snapshot.phase]
        current = circle_by_phase.get(eligible[-1]) if eligible else None

    if current is not None:
        distance = current.distance_to_center_m(snapshot.center)
        snapshot.distance_to_center_m = distance
        snapshot.distance_to_edge_m = current.distance_to_edge_m(snapshot.center)
        radius_m = current.radius_cm / CM_PER_METER
        snapshot.normalized_center_distance = distance / radius_m if radius_m else None
        snapshot.inside_current_zone = current.contains(snapshot.center)

    future = [circle_by_phase[phase] for phase in ordered_circle_phases if phase > snapshot.phase]
    if future:
        next_circle = future[0]
        snapshot.next_zone_contains = next_circle.contains(snapshot.center)
        snapshot.next_zone_distance_m = max(0.0, -next_circle.distance_to_edge_m(snapshot.center))
    if len(future) >= 2:
        snapshot.phase_plus_2_contains = future[1].contains(snapshot.center)
    if len(future) >= 3:
        snapshot.phase_plus_3_contains = future[2].contains(snapshot.center)


def _add_enemy_density(snapshots: list[TeamSnapshot]) -> None:
    by_bucket: dict[int, list[TeamSnapshot]] = defaultdict(list)
    for snapshot in snapshots:
        by_bucket[snapshot.bucket].append(snapshot)

    for bucket_snapshots in by_bucket.values():
        for index, snapshot in enumerate(bucket_snapshots):
            for other in bucket_snapshots[index + 1 :]:
                if snapshot.team_id == other.team_id:
                    continue
                distance_m = snapshot.center.distance_2d_cm(other.center) / CM_PER_METER
                if distance_m <= 500:
                    snapshot.enemy_teams_500m += 1
                    other.enemy_teams_500m += 1
                if distance_m <= 300:
                    snapshot.enemy_teams_300m += 1
                    other.enemy_teams_300m += 1
                if distance_m <= 100:
                    snapshot.enemy_teams_100m += 1
                    other.enemy_teams_100m += 1


def _add_future_labels(snapshots: list[TeamSnapshot], *, match_end_s: float) -> None:
    by_team: dict[int, list[TeamSnapshot]] = defaultdict(list)
    for snapshot in snapshots:
        by_team[snapshot.team_id].append(snapshot)

    horizons = (
        (30.0, "survives_30s", "future_displacement_30m"),
        (60.0, "survives_60s", "future_displacement_60m"),
        (120.0, "survives_120s", "future_displacement_120m"),
    )
    for team_snapshots in by_team.values():
        team_snapshots.sort(key=lambda item: item.elapsed_s)
        times = [item.elapsed_s for item in team_snapshots]
        for snapshot in team_snapshots:
            for horizon_s, survival_attr, displacement_attr in horizons:
                target_s = snapshot.elapsed_s + horizon_s
                if target_s > match_end_s:
                    setattr(snapshot, survival_attr, None)
                    setattr(snapshot, displacement_attr, None)
                    continue
                future_index = bisect_left(times, target_s)
                survived = future_index < len(team_snapshots)
                setattr(snapshot, survival_attr, survived)
                if survived:
                    future = team_snapshots[future_index]
                    displacement_m = snapshot.center.distance_2d_cm(future.center) / CM_PER_METER
                    setattr(snapshot, displacement_attr, displacement_m)


def _add_hold_durations(
    accumulators: dict[tuple[int, int, int], CellAccumulator],
    snapshots: list[TeamSnapshot],
    *,
    bucket_seconds: float,
) -> None:
    by_team: dict[int, list[TeamSnapshot]] = defaultdict(list)
    for snapshot in snapshots:
        by_team[snapshot.team_id].append(snapshot)

    for team_snapshots in by_team.values():
        team_snapshots.sort(key=lambda item: item.elapsed_s)
        run: list[TeamSnapshot] = []
        for snapshot in team_snapshots:
            if run:
                previous = run[-1]
                same_phase_cell = (
                    snapshot.phase == previous.phase
                    and snapshot.cell_x == previous.cell_x
                    and snapshot.cell_y == previous.cell_y
                )
                contiguous = snapshot.elapsed_s - previous.elapsed_s <= bucket_seconds * 2.5
                if not same_phase_cell or not contiguous:
                    _finish_hold_run(accumulators, run, bucket_seconds)
                    run = []
            run.append(snapshot)
        _finish_hold_run(accumulators, run, bucket_seconds)


def _finish_hold_run(
    accumulators: dict[tuple[int, int, int], CellAccumulator],
    run: list[TeamSnapshot],
    bucket_seconds: float,
) -> None:
    if not run:
        return
    first, last = run[0], run[-1]
    duration_s = max(bucket_seconds, last.elapsed_s - first.elapsed_s + bucket_seconds)
    key = (first.phase, first.cell_x, first.cell_y)
    accumulators[key].hold_durations_s.append(duration_s)


def _collect_combat(
    events: list[dict[str, Any]],
    clock: EventClock,
    phases: PhaseTimeline,
    *,
    cell_size_m: float,
) -> dict[tuple[int, int, int], CombatTotals]:
    totals: dict[tuple[int, int, int], CombatTotals] = defaultdict(CombatTotals)

    for event in events:
        event_type = event.get("_T")
        if event_type not in {"LogPlayerTakeDamage", "LogPlayerMakeGroggy", "LogPlayerKillV2"}:
            continue
        elapsed_s = clock.elapsed(event)
        if elapsed_s is None or elapsed_s < 0:
            continue
        phase = phases.phase_at(elapsed_s)

        if event_type == "LogPlayerTakeDamage":
            damage = _float(event.get("damage"), default=0.0)
            attacker_key = _character_cell_key(event.get("attacker"), phase, cell_size_m)
            victim_key = _character_cell_key(event.get("victim"), phase, cell_size_m)
            if attacker_key is not None:
                totals[attacker_key].damage_dealt += damage
            if victim_key is not None:
                totals[victim_key].damage_received += damage

        elif event_type == "LogPlayerMakeGroggy":
            attacker_key = _character_cell_key(event.get("attacker"), phase, cell_size_m)
            victim_key = _character_cell_key(event.get("victim"), phase, cell_size_m)
            if attacker_key is not None:
                totals[attacker_key].knocks_given += 1
            if victim_key is not None:
                totals[victim_key].knocks_received += 1

        elif event_type == "LogPlayerKillV2":
            killer = event.get("killer") or event.get("finisher")
            killer_key = _character_cell_key(killer, phase, cell_size_m)
            victim_key = _character_cell_key(event.get("victim"), phase, cell_size_m)
            if killer_key is not None:
                totals[killer_key].kills += 1
            if victim_key is not None:
                totals[victim_key].deaths += 1
    return totals


def _character_cell_key(
    character: Any, phase: int, cell_size_m: float
) -> tuple[int, int, int] | None:
    if not isinstance(character, dict):
        return None
    location = Vec3.from_mapping(character.get("location"))
    if location is None:
        return None
    cell_size_cm = cell_size_m * CM_PER_METER
    return (
        phase,
        int(floor(location.x_cm / cell_size_cm)),
        int(floor(location.y_cm / cell_size_cm)),
    )


def _build_elevation_map(
    snapshots: list[TeamSnapshot], *, cell_size_m: float
) -> dict[tuple[int, int], tuple[float, float | None, float]]:
    heights: dict[tuple[int, int], list[float]] = defaultdict(list)
    for snapshot in snapshots:
        heights[(snapshot.cell_x, snapshot.cell_y)].append(snapshot.center.z_cm / CM_PER_METER)

    ground = {cell: _percentile(values, 0.2) for cell, values in heights.items()}
    radius_cells = max(1, int(round(300.0 / cell_size_m)))
    result: dict[tuple[int, int], tuple[float, float | None, float]] = {}
    for (cell_x, cell_y), ground_z in ground.items():
        neighbors = [
            neighbor_z
            for (other_x, other_y), neighbor_z in ground.items()
            if (other_x, other_y) != (cell_x, cell_y)
            and hypot(other_x - cell_x, other_y - cell_y) <= radius_cells
        ]
        relative = ground_z - median(neighbors) if neighbors else None
        values = heights[(cell_x, cell_y)]
        z_iqr = _percentile(values, 0.75) - _percentile(values, 0.25)
        result[(cell_x, cell_y)] = (ground_z, relative, z_iqr)
    return result


def _cell_to_dict(
    accumulator: CellAccumulator,
    elevation: dict[tuple[int, int], tuple[float, float | None, float]],
    *,
    cell_size_m: float,
) -> dict[str, Any]:
    samples = accumulator.samples
    visits = len(accumulator.hold_durations_s)
    hold_mean = _mean_or_none(accumulator.hold_durations_s)
    survival_30 = _boolean_rate(sample.survives_30s for sample in samples)
    survival_60 = _boolean_rate(sample.survives_60s for sample in samples)
    survival_120 = _boolean_rate(sample.survives_120s for sample in samples)
    retention = _boolean_rate(sample.next_zone_contains for sample in samples)
    next_distance = _mean_optional(sample.next_zone_distance_m for sample in samples)
    displacement_60 = _mean_optional(sample.future_displacement_60m for sample in samples)
    enemy_300 = mean(sample.enemy_teams_300m for sample in samples)
    ground_z, relative_z, z_iqr = elevation.get(
        (accumulator.cell_x, accumulator.cell_y), (0.0, None, 0.0)
    )

    score, score_components, confidence = _historical_score(
        samples=samples,
        visits=visits,
        hold_mean=hold_mean,
        survival_120=survival_120,
        retention=retention,
        next_distance=next_distance,
        enemy_300=enemy_300,
        combat=accumulator.combat,
    )
    center_x_m = (accumulator.cell_x + 0.5) * cell_size_m
    center_y_m = (accumulator.cell_y + 0.5) * cell_size_m
    return {
        "phase_cell_id": f"p{accumulator.phase}:{accumulator.cell_x}:{accumulator.cell_y}",
        "cell_id": f"{accumulator.cell_x}:{accumulator.cell_y}",
        "phase": accumulator.phase,
        "cell_x": accumulator.cell_x,
        "cell_y": accumulator.cell_y,
        "center_x_m": round(center_x_m, 3),
        "center_y_m": round(center_y_m, 3),
        "occupancy_samples": len(samples),
        "unique_teams": len(accumulator.unique_teams),
        "visits": visits,
        "observed_hold_seconds": round(sum(accumulator.hold_durations_s), 3),
        "mean_hold_seconds": _rounded(hold_mean),
        "vehicle_sample_rate": round(
            sum(sample.vehicle_member_count > 0 for sample in samples) / len(samples), 4
        ),
        "mean_team_spread_m": round(mean(sample.spread_m for sample in samples), 3),
        "survival": {
            "30s": _rate_dict(survival_30),
            "60s": _rate_dict(survival_60),
            "120s": _rate_dict(survival_120),
        },
        "zone": {
            "observed_next_zone_retention": _rate_dict(retention),
            "mean_next_zone_entry_distance_m": _rounded(next_distance),
            "mean_normalized_center_distance": _rounded(
                _mean_optional(sample.normalized_center_distance for sample in samples)
            ),
        },
        "rotation": {
            "mean_future_displacement_60m": _rounded(displacement_60),
        },
        "contest": {
            "mean_enemy_teams_100m": round(mean(sample.enemy_teams_100m for sample in samples), 4),
            "mean_enemy_teams_300m": round(enemy_300, 4),
            "mean_enemy_teams_500m": round(mean(sample.enemy_teams_500m for sample in samples), 4),
        },
        "combat": {
            "damage_dealt": round(accumulator.combat.damage_dealt, 3),
            "damage_received": round(accumulator.combat.damage_received, 3),
            "knocks_given": accumulator.combat.knocks_given,
            "knocks_received": accumulator.combat.knocks_received,
            "kills": accumulator.combat.kills,
            "deaths": accumulator.combat.deaths,
        },
        "elevation_proxy": {
            "estimated_ground_z_m": round(ground_z, 3),
            "relative_elevation_300m": _rounded(relative_z),
            "z_iqr_m": round(z_iqr, 3),
        },
        "historical_value_score": round(score, 2),
        "score_confidence": round(confidence, 4),
        "score_components": {key: round(value, 4) for key, value in score_components.items()},
    }


def _historical_score(
    *,
    samples: list[TeamSnapshot],
    visits: int,
    hold_mean: float | None,
    survival_120: tuple[float, int] | None,
    retention: tuple[float, int] | None,
    next_distance: float | None,
    enemy_300: float,
    combat: CombatTotals,
) -> tuple[float, dict[str, float], float]:
    survival_component = _shrunk_rate(survival_120)
    retention_component = _shrunk_rate(retention)
    hold_component = min(1.0, (hold_mean or 0.0) / 120.0)
    rotation_component = exp(-(next_distance or 0.0) / 500.0) if next_distance is not None else 0.5
    net_damage_per_visit = (combat.damage_dealt - combat.damage_received) / max(1, visits)
    combat_component = 0.5 + 0.5 * tanh(net_damage_per_visit / 100.0)
    contest_component = 1.0 - min(1.0, enemy_300 / 3.0)

    components = {
        "survival_120s": survival_component,
        "next_zone_retention": retention_component,
        "hold": hold_component,
        "rotation_burden": rotation_component,
        "combat_balance": combat_component,
        "low_contest": contest_component,
    }
    raw = (
        0.25 * survival_component
        + 0.20 * retention_component
        + 0.15 * hold_component
        + 0.15 * rotation_component
        + 0.15 * combat_component
        + 0.10 * contest_component
    )
    confidence = 1.0 - exp(-max(1, min(visits, len(samples))) / 8.0)
    score = 50.0 + confidence * (raw * 100.0 - 50.0)
    return score, components, confidence


def _boolean_rate(values: Iterable[bool | None]) -> tuple[float, int] | None:
    observed = [value for value in values if value is not None]
    if not observed:
        return None
    return sum(observed) / len(observed), len(observed)


def _rate_dict(value: tuple[float, int] | None) -> dict[str, float | int] | None:
    if value is None:
        return None
    rate, observations = value
    return {"rate": round(rate, 4), "observations": observations}


def _shrunk_rate(value: tuple[float, int] | None) -> float:
    if value is None:
        return 0.5
    rate, observations = value
    successes = rate * observations
    return (successes + 2.0) / (observations + 4.0)


def _mean_optional(values: Iterable[float | None]) -> float | None:
    observed = [value for value in values if value is not None]
    return mean(observed) if observed else None


def _mean_or_none(values: list[float]) -> float | None:
    return mean(values) if values else None


def _percentile(values: list[float], fraction: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    if len(ordered) == 1:
        return ordered[0]
    index = fraction * (len(ordered) - 1)
    lower = int(floor(index))
    upper = min(lower + 1, len(ordered) - 1)
    weight = index - lower
    return ordered[lower] * (1.0 - weight) + ordered[upper] * weight


def _integer(value: Any) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        return value
    if isinstance(value, float) and value.is_integer():
        return int(value)
    return None


def _float(value: Any, *, default: float) -> float:
    if isinstance(value, bool):
        return default
    if isinstance(value, (int, float)):
        return float(value)
    return default


def _rounded(value: float | None) -> float | None:
    return None if value is None else round(value, 3)
