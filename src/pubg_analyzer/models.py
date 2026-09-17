from __future__ import annotations

from dataclasses import dataclass, field
from math import hypot
from typing import Any

CM_PER_METER = 100.0


def cm_to_m(value: float | None) -> float | None:
    if value is None:
        return None
    return round(value / CM_PER_METER, 3)


@dataclass(frozen=True, slots=True)
class Vec3:
    x_cm: float
    y_cm: float
    z_cm: float

    @classmethod
    def from_mapping(cls, value: Any) -> Vec3 | None:
        if not isinstance(value, dict):
            return None
        try:
            return cls(float(value["x"]), float(value["y"]), float(value.get("z", 0.0)))
        except (KeyError, TypeError, ValueError):
            return None

    def distance_2d_cm(self, other: Vec3) -> float:
        return hypot(self.x_cm - other.x_cm, self.y_cm - other.y_cm)

    def to_dict(self) -> dict[str, float]:
        return {
            "x_m": round(self.x_cm / CM_PER_METER, 3),
            "y_m": round(self.y_cm / CM_PER_METER, 3),
            "z_m": round(self.z_cm / CM_PER_METER, 3),
        }


@dataclass(frozen=True, slots=True)
class CircleState:
    phase: int
    center: Vec3
    radius_cm: float
    first_seen_s: float
    last_seen_s: float
    sample_count: int
    source: str

    def contains(self, point: Vec3) -> bool:
        return self.center.distance_2d_cm(point) <= self.radius_cm

    def distance_to_center_m(self, point: Vec3) -> float:
        return self.center.distance_2d_cm(point) / CM_PER_METER

    def distance_to_edge_m(self, point: Vec3) -> float:
        return (self.radius_cm - self.center.distance_2d_cm(point)) / CM_PER_METER

    def to_dict(self) -> dict[str, Any]:
        return {
            "phase": self.phase,
            "center": self.center.to_dict(),
            "radius_m": round(self.radius_cm / CM_PER_METER, 3),
            "first_seen_s": round(self.first_seen_s, 3),
            "last_seen_s": round(self.last_seen_s, 3),
            "sample_count": self.sample_count,
            "source": self.source,
        }


@dataclass(slots=True)
class TeamSnapshot:
    bucket: int
    elapsed_s: float
    phase: int
    team_id: int
    center: Vec3
    member_count: int
    spread_m: float
    vehicle_member_count: int
    cell_x: int
    cell_y: int
    distance_to_center_m: float | None = None
    distance_to_edge_m: float | None = None
    normalized_center_distance: float | None = None
    inside_current_zone: bool | None = None
    next_zone_contains: bool | None = None
    next_zone_distance_m: float | None = None
    phase_plus_2_contains: bool | None = None
    phase_plus_3_contains: bool | None = None
    enemy_teams_100m: int = 0
    enemy_teams_300m: int = 0
    enemy_teams_500m: int = 0
    survives_30s: bool | None = None
    survives_60s: bool | None = None
    survives_120s: bool | None = None
    future_displacement_30m: float | None = None
    future_displacement_60m: float | None = None
    future_displacement_120m: float | None = None

    @property
    def cell_id(self) -> str:
        return f"{self.cell_x}:{self.cell_y}"

    def to_dict(self) -> dict[str, Any]:
        return {
            "elapsed_s": round(self.elapsed_s, 3),
            "phase": self.phase,
            "team_id": self.team_id,
            "center": self.center.to_dict(),
            "member_count": self.member_count,
            "spread_m": round(self.spread_m, 3),
            "vehicle_member_count": self.vehicle_member_count,
            "cell_id": self.cell_id,
            "cell_x": self.cell_x,
            "cell_y": self.cell_y,
            "zone": {
                "distance_to_center_m": _round_optional(self.distance_to_center_m),
                "distance_to_edge_m": _round_optional(self.distance_to_edge_m),
                "normalized_center_distance": _round_optional(self.normalized_center_distance),
                "inside_current": self.inside_current_zone,
                "next_contains": self.next_zone_contains,
                "next_distance_m": _round_optional(self.next_zone_distance_m),
                "phase_plus_2_contains": self.phase_plus_2_contains,
                "phase_plus_3_contains": self.phase_plus_3_contains,
            },
            "nearby_enemy_teams": {
                "100m": self.enemy_teams_100m,
                "300m": self.enemy_teams_300m,
                "500m": self.enemy_teams_500m,
            },
            "future": {
                "survives_30s": self.survives_30s,
                "survives_60s": self.survives_60s,
                "survives_120s": self.survives_120s,
                "displacement_30m": _round_optional(self.future_displacement_30m),
                "displacement_60m": _round_optional(self.future_displacement_60m),
                "displacement_120m": _round_optional(self.future_displacement_120m),
            },
        }


@dataclass(frozen=True, slots=True)
class MovementSegment:
    team_id: int
    phase: int
    started_at_s: float
    ended_at_s: float
    start_cell_id: str
    end_cell_id: str
    straight_distance_m: float
    duration_s: float
    speed_mps: float
    used_vehicle: bool

    def to_dict(self) -> dict[str, Any]:
        return {
            "team_id": self.team_id,
            "phase": self.phase,
            "started_at_s": round(self.started_at_s, 3),
            "ended_at_s": round(self.ended_at_s, 3),
            "start_cell_id": self.start_cell_id,
            "end_cell_id": self.end_cell_id,
            "straight_distance_m": round(self.straight_distance_m, 3),
            "duration_s": round(self.duration_s, 3),
            "speed_mps": round(self.speed_mps, 3),
            "used_vehicle": self.used_vehicle,
        }


@dataclass(slots=True)
class CombatTotals:
    damage_dealt: float = 0.0
    damage_received: float = 0.0
    knocks_given: int = 0
    knocks_received: int = 0
    kills: int = 0
    deaths: int = 0


@dataclass(slots=True)
class CellAccumulator:
    phase: int
    cell_x: int
    cell_y: int
    samples: list[TeamSnapshot] = field(default_factory=list)
    unique_teams: set[int] = field(default_factory=set)
    hold_durations_s: list[float] = field(default_factory=list)
    combat: CombatTotals = field(default_factory=CombatTotals)


def _round_optional(value: float | None, digits: int = 3) -> float | None:
    return None if value is None else round(value, digits)
