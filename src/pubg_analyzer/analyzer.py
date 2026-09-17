from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
from statistics import mean
from typing import Any

from .features import aggregate_phase_cells, build_movement_segments, build_team_snapshots
from .io import load_telemetry
from .timeline import EventClock, build_circle_states, build_phase_timeline


@dataclass(frozen=True, slots=True)
class AnalysisConfig:
    cell_size_m: float = 100.0
    bucket_seconds: float = 10.0
    include_snapshots: bool = False
    include_movement_segments: bool = False

    def __post_init__(self) -> None:
        if not 10.0 <= self.cell_size_m <= 1000.0:
            raise ValueError("cell_size_m must be between 10 and 1000")
        if not 1.0 <= self.bucket_seconds <= 60.0:
            raise ValueError("bucket_seconds must be between 1 and 60")


class TelemetryAnalyzer:
    def __init__(self, config: AnalysisConfig | None = None) -> None:
        self.config = config or AnalysisConfig()

    def analyze_source(
        self,
        source: str,
        *,
        allowed_hosts: set[str] | None = None,
    ) -> dict[str, Any]:
        events = load_telemetry(source, allowed_hosts=allowed_hosts)
        return self.analyze(events, source=source)

    def analyze(
        self,
        events: list[dict[str, Any]],
        *,
        source: str | None = None,
    ) -> dict[str, Any]:
        if not events:
            raise ValueError("Telemetry contains no events")

        clock = EventClock.from_events(events)
        phases = build_phase_timeline(events, clock)
        circles = build_circle_states(events, clock, phases)
        match_end_s = _match_end(events, clock)
        snapshots = build_team_snapshots(
            events,
            clock,
            phases,
            circles,
            bucket_seconds=self.config.bucket_seconds,
            cell_size_m=self.config.cell_size_m,
            match_end_s=match_end_s,
        )
        movements = build_movement_segments(
            snapshots,
            bucket_seconds=self.config.bucket_seconds,
        )
        cells = aggregate_phase_cells(
            events,
            snapshots,
            clock,
            phases,
            bucket_seconds=self.config.bucket_seconds,
            cell_size_m=self.config.cell_size_m,
        )

        result: dict[str, Any] = {
            "schema_version": "0.1.0",
            "source": source,
            "units": {
                "telemetry_coordinates": "centimeters",
                "output_distance": "meters",
                "elapsed_time": "seconds",
            },
            "config": {
                "cell_size_m": self.config.cell_size_m,
                "bucket_seconds": self.config.bucket_seconds,
            },
            "match": _match_metadata(events),
            "summary": _summary(events, snapshots, circles, movements, cells, match_end_s),
            "event_counts": dict(
                sorted(Counter(event.get("_T", "unknown") for event in events).items())
            ),
            "phase_timeline": [
                {"phase": marker.phase, "elapsed_s": round(marker.elapsed_s, 3)}
                for marker in phases.markers
            ],
            "circles": [circle.to_dict() for circle in circles],
            "position_cells": cells,
            "movement_summary": _movement_summary(movements),
            "methodology": {
                "circle_source": (
                    "Phase target circles use poisonGasWarningPosition/radius. "
                    "Phase 0 uses the initial safety zone."
                ),
                "elevation_proxy": (
                    "The 20th percentile of observed team-center z per spatial cell; "
                    "it is not authoritative terrain height."
                ),
                "score_scope": (
                    "historical_value_score is a smoothed descriptive score for this input "
                    "dataset, not a causal estimate or a live next-position recommendation."
                ),
            },
        }
        if self.config.include_snapshots:
            result["team_snapshots"] = [snapshot.to_dict() for snapshot in snapshots]
        if self.config.include_movement_segments:
            result["movement_segments"] = [segment.to_dict() for segment in movements]
        return result


def _match_end(events: list[dict[str, Any]], clock: EventClock) -> float:
    end_times = [
        clock.elapsed(event)
        for event in events
        if event.get("_T") == "LogMatchEnd" and clock.elapsed(event) is not None
    ]
    if end_times:
        return max(0.0, max(end_times))
    all_times = [clock.elapsed(event) for event in events]
    observed = [value for value in all_times if value is not None and value >= 0]
    return max(observed, default=0.0)


def _match_metadata(events: list[dict[str, Any]]) -> dict[str, Any]:
    start = next((event for event in events if event.get("_T") == "LogMatchStart"), {})
    options = start.get("blueZoneCustomOptions")
    return {
        "map_name": start.get("mapName"),
        "team_size": start.get("teamSize"),
        "weather_id": start.get("weatherId"),
        "is_custom_game": start.get("isCustomGame"),
        "is_event_mode": start.get("isEventMode"),
        "blue_zone_custom_options": options if isinstance(options, list) else [],
    }


def _summary(
    events: list[dict[str, Any]],
    snapshots,
    circles,
    movements,
    cells,
    match_end_s: float,
) -> dict[str, Any]:
    team_ids = {snapshot.team_id for snapshot in snapshots}
    players: set[str] = set()
    for event in events:
        character = event.get("character")
        if isinstance(character, dict):
            player_id = character.get("accountId") or character.get("name")
            if player_id:
                players.add(str(player_id))
    return {
        "event_count": len(events),
        "duration_s": round(match_end_s, 3),
        "observed_players": len(players),
        "observed_teams": len(team_ids),
        "phases_with_circle": len(circles),
        "team_snapshot_count": len(snapshots),
        "movement_segment_count": len(movements),
        "phase_cell_count": len(cells),
    }


def _movement_summary(movements) -> dict[str, Any]:
    if not movements:
        return {
            "segment_count": 0,
            "total_distance_m": 0.0,
            "mean_speed_mps": None,
            "vehicle_segment_rate": None,
        }
    return {
        "segment_count": len(movements),
        "total_distance_m": round(sum(segment.straight_distance_m for segment in movements), 3),
        "mean_speed_mps": round(mean(segment.speed_mps for segment in movements), 3),
        "vehicle_segment_rate": round(
            sum(segment.used_vehicle for segment in movements) / len(movements), 4
        ),
    }
