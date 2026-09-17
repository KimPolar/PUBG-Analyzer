from __future__ import annotations

from typing import Any

from fastapi import FastAPI, HTTPException
from fastapi.concurrency import run_in_threadpool
from pydantic import BaseModel, ConfigDict, Field

from .analyzer import AnalysisConfig, TelemetryAnalyzer
from .io import TelemetryLoadError, load_telemetry

OFFICIAL_TELEMETRY_HOSTS = {"telemetry-cdn.pubg.com"}


class AnalyzeOptions(BaseModel):
    model_config = ConfigDict(extra="forbid")

    cell_size_m: float = Field(default=100.0, ge=10.0, le=1000.0)
    bucket_seconds: float = Field(default=10.0, ge=1.0, le=60.0)
    include_snapshots: bool = False
    include_movement_segments: bool = False


class AnalyzeUrlRequest(AnalyzeOptions):
    url: str


class AnalyzeEventsRequest(AnalyzeOptions):
    events: list[dict[str, Any]]


app = FastAPI(
    title="PUBG Position Analyzer",
    version="0.1.0",
    description="Phase-aware feature extraction from PUBG telemetry.",
)


@app.get("/health")
def health() -> dict[str, str]:
    return {"status": "ok"}


@app.get("/v1/capabilities")
def capabilities() -> dict[str, Any]:
    return {
        "schema_version": "0.1.0",
        "official_url_hosts": sorted(OFFICIAL_TELEMETRY_HOSTS),
        "features": [
            "phase target circles",
            "team position snapshots",
            "zone-relative position",
            "enemy density",
            "future survival labels",
            "movement segments",
            "phase-aware cell aggregates",
            "elevation proxy",
        ],
    }


@app.post("/v1/analyze/url")
async def analyze_url(request: AnalyzeUrlRequest) -> dict[str, Any]:
    try:
        events = await run_in_threadpool(
            load_telemetry,
            request.url,
            allowed_hosts=OFFICIAL_TELEMETRY_HOSTS,
        )
        return await run_in_threadpool(_analyze, events, request, request.url)
    except TelemetryLoadError as exc:
        raise HTTPException(status_code=400, detail=str(exc)) from exc
    except ValueError as exc:
        raise HTTPException(status_code=422, detail=str(exc)) from exc


@app.post("/v1/analyze/events")
async def analyze_events(request: AnalyzeEventsRequest) -> dict[str, Any]:
    try:
        return await run_in_threadpool(_analyze, request.events, request, "request-body")
    except ValueError as exc:
        raise HTTPException(status_code=422, detail=str(exc)) from exc


def _analyze(events: list[dict[str, Any]], options: AnalyzeOptions, source: str) -> dict[str, Any]:
    analyzer = TelemetryAnalyzer(
        AnalysisConfig(
            cell_size_m=options.cell_size_m,
            bucket_seconds=options.bucket_seconds,
            include_snapshots=options.include_snapshots,
            include_movement_segments=options.include_movement_segments,
        )
    )
    return analyzer.analyze(events, source=source)
