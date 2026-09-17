from __future__ import annotations

from collections import defaultdict
from collections.abc import Iterable
from dataclasses import dataclass
from datetime import datetime
from statistics import median
from typing import Any

from .models import CircleState, Vec3


def _number(value: Any) -> float | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, (int, float)):
        return float(value)
    return None


def _timestamp(value: Any) -> float | None:
    if not isinstance(value, str):
        return None
    try:
        return datetime.fromisoformat(value.replace("Z", "+00:00")).timestamp()
    except ValueError:
        return None


@dataclass(frozen=True, slots=True)
class PhaseMarker:
    elapsed_s: float
    phase: int


class EventClock:
    """Converts event timestamps to one match-wide elapsed-time axis.

    PlayerPosition.elapsedTime is not globally aligned for every player in real telemetry.
    GameStatePeriodic provides the stable anchor, so timestamps are preferred after anchoring.
    """

    def __init__(self, origin_epoch_s: float | None) -> None:
        self.origin_epoch_s = origin_epoch_s

    @classmethod
    def from_events(cls, events: Iterable[dict[str, Any]]) -> EventClock:
        events = list(events)
        preferred: list[float] = []
        fallback: list[float] = []
        timestamps: list[float] = []

        for event in events:
            epoch = _timestamp(event.get("_D"))
            if epoch is not None:
                timestamps.append(epoch)
            explicit = _explicit_elapsed(event)
            if epoch is None or explicit is None:
                continue
            anchor = epoch - explicit
            fallback.append(anchor)
            if event.get("_T") == "LogGameStatePeriodic":
                preferred.append(anchor)

        anchors = preferred or fallback
        if anchors:
            return cls(float(median(anchors)))
        return cls(min(timestamps) if timestamps else None)

    def elapsed(self, event: dict[str, Any]) -> float | None:
        epoch = _timestamp(event.get("_D"))
        if epoch is not None and self.origin_epoch_s is not None:
            return epoch - self.origin_epoch_s
        return _explicit_elapsed(event)


class PhaseTimeline:
    def __init__(self, markers: Iterable[PhaseMarker]) -> None:
        ordered = sorted(markers, key=lambda marker: marker.elapsed_s)
        deduplicated: list[PhaseMarker] = []
        for marker in ordered:
            if deduplicated and deduplicated[-1].phase == marker.phase:
                continue
            deduplicated.append(marker)
        self.markers = deduplicated

    def phase_at(self, elapsed_s: float) -> int:
        phase = 0
        for marker in self.markers:
            if marker.elapsed_s > elapsed_s:
                break
            phase = marker.phase
        return phase

    @property
    def phases(self) -> list[int]:
        return sorted({marker.phase for marker in self.markers})


def build_phase_timeline(events: list[dict[str, Any]], clock: EventClock) -> PhaseTimeline:
    markers: list[PhaseMarker] = []
    for event in events:
        if event.get("_T") != "LogPhaseChange":
            continue
        phase = event.get("phase")
        elapsed_s = clock.elapsed(event)
        if isinstance(phase, int) and elapsed_s is not None:
            markers.append(PhaseMarker(elapsed_s=max(0.0, elapsed_s), phase=phase))

    if markers:
        return PhaseTimeline(markers)
    return _infer_phase_timeline(events, clock)


def build_circle_states(
    events: list[dict[str, Any]], clock: EventClock, phases: PhaseTimeline
) -> list[CircleState]:
    initial_samples: list[tuple[float, Vec3, float]] = []
    warning_samples: dict[int, list[tuple[float, Vec3, float]]] = defaultdict(list)

    for event in events:
        if event.get("_T") != "LogGameStatePeriodic":
            continue
        elapsed_s = clock.elapsed(event)
        game_state = event.get("gameState")
        if elapsed_s is None or not isinstance(game_state, dict):
            continue

        safety = Vec3.from_mapping(game_state.get("safetyZonePosition"))
        safety_radius = _number(game_state.get("safetyZoneRadius"))
        if safety is not None and safety_radius is not None and safety_radius > 0:
            initial_samples.append((elapsed_s, safety, safety_radius))

        phase = phases.phase_at(elapsed_s)
        warning = Vec3.from_mapping(game_state.get("poisonGasWarningPosition"))
        warning_radius = _number(game_state.get("poisonGasWarningRadius"))
        if phase > 0 and warning is not None and warning_radius is not None and warning_radius > 0:
            warning_samples[phase].append((elapsed_s, warning, warning_radius))

    circles: list[CircleState] = []
    if initial_samples:
        first_phase_time = phases.markers[0].elapsed_s if phases.markers else float("inf")
        before_phase = [sample for sample in initial_samples if sample[0] <= first_phase_time]
        sample_pool = before_phase or initial_samples[:1]
        circles.append(_representative_circle(0, sample_pool, "initial_safety_zone"))

    for phase, samples in sorted(warning_samples.items()):
        circles.append(_representative_circle(phase, samples, "poison_gas_warning_zone"))
    return circles


def _representative_circle(
    phase: int, samples: list[tuple[float, Vec3, float]], source: str
) -> CircleState:
    return CircleState(
        phase=phase,
        center=Vec3(
            float(median(sample[1].x_cm for sample in samples)),
            float(median(sample[1].y_cm for sample in samples)),
            float(median(sample[1].z_cm for sample in samples)),
        ),
        radius_cm=float(median(sample[2] for sample in samples)),
        first_seen_s=min(sample[0] for sample in samples),
        last_seen_s=max(sample[0] for sample in samples),
        sample_count=len(samples),
        source=source,
    )


def _explicit_elapsed(event: dict[str, Any]) -> float | None:
    direct = _number(event.get("elapsedTime"))
    if direct is not None:
        return direct
    game_state = event.get("gameState")
    if isinstance(game_state, dict):
        return _number(game_state.get("elapsedTime"))
    return None


def _infer_phase_timeline(events: list[dict[str, Any]], clock: EventClock) -> PhaseTimeline:
    markers: list[PhaseMarker] = []
    previous_radius: float | None = None
    phase = 0
    for event in events:
        if event.get("_T") != "LogGameStatePeriodic":
            continue
        elapsed_s = clock.elapsed(event)
        game_state = event.get("gameState")
        if elapsed_s is None or not isinstance(game_state, dict):
            continue
        radius = _number(game_state.get("poisonGasWarningRadius"))
        if radius is None or radius <= 0:
            continue
        threshold = max(100.0, (previous_radius or radius) * 0.005)
        if previous_radius is None or abs(radius - previous_radius) > threshold:
            phase += 1
            markers.append(PhaseMarker(max(0.0, elapsed_s), phase))
            previous_radius = radius
    return PhaseTimeline(markers)
