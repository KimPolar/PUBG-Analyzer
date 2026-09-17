from __future__ import annotations

from pathlib import Path

from pubg_analyzer import AnalysisConfig, TelemetryAnalyzer
from pubg_analyzer.io import load_telemetry

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_telemetry.json"


def test_analyzer_builds_phase_aware_position_features() -> None:
    events = load_telemetry(FIXTURE)
    analyzer = TelemetryAnalyzer(
        AnalysisConfig(include_snapshots=True, include_movement_segments=True)
    )

    result = analyzer.analyze(events, source=str(FIXTURE))

    assert result["match"]["map_name"] == "Baltic_Main"
    assert result["summary"]["observed_teams"] == 2
    assert result["summary"]["observed_players"] == 3
    assert [circle["phase"] for circle in result["circles"]] == [0, 1, 2]
    assert [marker["phase"] for marker in result["phase_timeline"]] == [1, 2]
    assert result["summary"]["team_snapshot_count"] > 0
    assert result["movement_summary"]["segment_count"] > 0

    snapshots = result["team_snapshots"]
    team_one_phase_one = next(
        snapshot for snapshot in snapshots if snapshot["team_id"] == 1 and snapshot["phase"] == 1
    )
    assert team_one_phase_one["zone"]["inside_current"] is True
    assert team_one_phase_one["nearby_enemy_teams"]["500m"] == 1

    cells = result["position_cells"]
    assert cells
    assert all("phase_cell_id" in cell for cell in cells)
    assert sum(cell["combat"]["damage_dealt"] for cell in cells) == 25
    assert sum(cell["combat"]["knocks_given"] for cell in cells) == 1
    assert sum(cell["combat"]["kills"] for cell in cells) == 1


def test_default_result_omits_large_row_collections() -> None:
    result = TelemetryAnalyzer().analyze(load_telemetry(FIXTURE))

    assert "team_snapshots" not in result
    assert "movement_segments" not in result
    assert "position_cells" in result
