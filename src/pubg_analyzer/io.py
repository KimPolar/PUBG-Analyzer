from __future__ import annotations

import gzip
import io
import json
from collections.abc import Iterable
from pathlib import Path
from typing import Any
from urllib.error import HTTPError, URLError
from urllib.parse import urlparse
from urllib.request import HTTPRedirectHandler, Request, build_opener

DEFAULT_MAX_BYTES = 128 * 1024 * 1024


class TelemetryLoadError(ValueError):
    """Raised when a telemetry source cannot be loaded or validated."""


class _CheckedRedirectHandler(HTTPRedirectHandler):
    def __init__(self, allowed_hosts: set[str] | None) -> None:
        self.allowed_hosts = allowed_hosts

    def redirect_request(self, req, fp, code, msg, headers, newurl):  # noqa: ANN001
        _validate_url(newurl, self.allowed_hosts)
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def load_telemetry(
    source: str | Path,
    *,
    max_bytes: int = DEFAULT_MAX_BYTES,
    allowed_hosts: Iterable[str] | None = None,
    timeout_s: float = 30.0,
) -> list[dict[str, Any]]:
    source_text = str(source)
    parsed = urlparse(source_text)
    hosts = {host.lower() for host in allowed_hosts} if allowed_hosts is not None else None

    if parsed.scheme in {"http", "https"}:
        raw = _download(source_text, max_bytes=max_bytes, allowed_hosts=hosts, timeout_s=timeout_s)
    else:
        path = Path(source).expanduser()
        if not path.is_file():
            raise TelemetryLoadError(f"Telemetry file does not exist: {path}")
        if path.stat().st_size > max_bytes:
            raise TelemetryLoadError(f"Telemetry file exceeds {max_bytes} bytes")
        raw = path.read_bytes()

    return load_telemetry_bytes(raw, max_bytes=max_bytes)


def load_telemetry_bytes(raw: bytes, *, max_bytes: int = DEFAULT_MAX_BYTES) -> list[dict[str, Any]]:
    if len(raw) > max_bytes:
        raise TelemetryLoadError(f"Compressed telemetry payload exceeds {max_bytes} bytes")

    if raw.startswith(b"\x1f\x8b"):
        try:
            with gzip.GzipFile(fileobj=io.BytesIO(raw)) as stream:
                raw = stream.read(max_bytes + 1)
        except OSError as exc:
            raise TelemetryLoadError("Invalid gzip telemetry payload") from exc

    if len(raw) > max_bytes:
        raise TelemetryLoadError(f"Decompressed telemetry payload exceeds {max_bytes} bytes")

    try:
        payload = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as exc:
        raise TelemetryLoadError("Telemetry payload is not valid UTF-8 JSON") from exc

    if not isinstance(payload, list):
        raise TelemetryLoadError("PUBG telemetry root must be a JSON array")
    if any(not isinstance(event, dict) for event in payload):
        raise TelemetryLoadError("Every telemetry event must be a JSON object")
    return payload


def _download(
    url: str,
    *,
    max_bytes: int,
    allowed_hosts: set[str] | None,
    timeout_s: float,
) -> bytes:
    _validate_url(url, allowed_hosts)
    opener = build_opener(_CheckedRedirectHandler(allowed_hosts))
    request = Request(url, headers={"User-Agent": "PUBG-Position-Analyzer/0.1"})
    try:
        with opener.open(request, timeout=timeout_s) as response:
            _validate_url(response.geturl(), allowed_hosts)
            declared_size = response.headers.get("Content-Length")
            if declared_size and int(declared_size) > max_bytes:
                raise TelemetryLoadError(f"Telemetry download exceeds {max_bytes} bytes")
            raw = response.read(max_bytes + 1)
    except TelemetryLoadError:
        raise
    except (HTTPError, URLError, TimeoutError, ValueError) as exc:
        raise TelemetryLoadError(f"Could not download telemetry: {exc}") from exc

    if len(raw) > max_bytes:
        raise TelemetryLoadError(f"Telemetry download exceeds {max_bytes} bytes")
    return raw


def _validate_url(url: str, allowed_hosts: set[str] | None) -> None:
    parsed = urlparse(url)
    if parsed.scheme not in {"http", "https"} or not parsed.hostname:
        raise TelemetryLoadError("Telemetry URL must use HTTP or HTTPS")
    host = parsed.hostname.lower().rstrip(".")
    if allowed_hosts is not None and host not in allowed_hosts:
        expected = ", ".join(sorted(allowed_hosts))
        raise TelemetryLoadError(f"Telemetry host '{host}' is not allowed; expected: {expected}")
