from __future__ import annotations

import json
from pathlib import Path

from pubg_analyzer.cli import main

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_telemetry.json"


def test_cli_writes_analysis_file(tmp_path: Path) -> None:
    output = tmp_path / "analysis.json"

    exit_code = main(["analyze", str(FIXTURE), "--output", str(output), "--compact"])

    assert exit_code == 0
    result = json.loads(output.read_text(encoding="utf-8"))
    assert result["schema_version"] == "0.1.0"
