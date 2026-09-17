from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from .analyzer import AnalysisConfig, TelemetryAnalyzer
from .io import TelemetryLoadError


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        prog="pubg-analyzer",
        description="Analyze PUBG telemetry into phase-aware position features.",
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    analyze = subparsers.add_parser("analyze", help="Analyze a local file or telemetry URL")
    analyze.add_argument("source", help="JSON/gzip file path or HTTP(S) telemetry URL")
    analyze.add_argument("-o", "--output", type=Path, help="Output JSON path; stdout if omitted")
    analyze.add_argument("--cell-size-m", type=float, default=100.0)
    analyze.add_argument("--bucket-seconds", type=float, default=10.0)
    analyze.add_argument("--include-snapshots", action="store_true")
    analyze.add_argument("--include-movement-segments", action="store_true")
    analyze.add_argument("--compact", action="store_true", help="Write compact JSON")

    serve = subparsers.add_parser("serve", help="Run the FastAPI service")
    serve.add_argument("--host", default="127.0.0.1")
    serve.add_argument("--port", type=int, default=8000)
    serve.add_argument("--reload", action="store_true")
    return parser


def main(argv: list[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    if args.command == "serve":
        return _serve(args.host, args.port, args.reload)

    try:
        analyzer = TelemetryAnalyzer(
            AnalysisConfig(
                cell_size_m=args.cell_size_m,
                bucket_seconds=args.bucket_seconds,
                include_snapshots=args.include_snapshots,
                include_movement_segments=args.include_movement_segments,
            )
        )
        result = analyzer.analyze_source(args.source)
    except (TelemetryLoadError, ValueError) as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    indent = None if args.compact else 2
    rendered = json.dumps(result, ensure_ascii=False, indent=indent)
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered + "\n", encoding="utf-8")
        print(f"wrote {args.output}", file=sys.stderr)
    else:
        print(rendered)
    return 0


def _serve(host: str, port: int, reload: bool) -> int:
    try:
        import uvicorn
    except ImportError:
        print("error: install project dependencies before starting the API", file=sys.stderr)
        return 2
    uvicorn.run("pubg_analyzer.api:app", host=host, port=port, reload=reload)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
