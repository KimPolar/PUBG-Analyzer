from __future__ import annotations

import gzip
from pathlib import Path

import pytest

from pubg_analyzer.io import TelemetryLoadError, load_telemetry, load_telemetry_bytes

FIXTURE = Path(__file__).parent / "fixtures" / "minimal_telemetry.json"


def test_loads_plain_and_gzip_payloads() -> None:
    plain = FIXTURE.read_bytes()
    expected = load_telemetry(FIXTURE)
    compressed = gzip.compress(plain)

    assert load_telemetry_bytes(compressed) == expected
    assert expected[0]["_T"] == "LogMatchStart"


def test_rejects_non_array_json() -> None:
    with pytest.raises(TelemetryLoadError, match="root must be a JSON array"):
        load_telemetry_bytes(b'{"_T":"LogMatchStart"}')


def test_rejects_unapproved_remote_host_before_download() -> None:
    with pytest.raises(TelemetryLoadError, match="not allowed"):
        load_telemetry(
            "https://example.com/telemetry.json",
            allowed_hosts={"telemetry-cdn.pubg.com"},
        )
