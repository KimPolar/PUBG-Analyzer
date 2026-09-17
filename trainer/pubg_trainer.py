"""Offline model trainer bundled as a PUBG Analyzer Tauri sidecar."""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from datetime import UTC, datetime
from pathlib import Path
from typing import Any, Iterable

import joblib
import numpy as np
from sklearn.feature_extraction import DictVectorizer
from sklearn.linear_model import LogisticRegression
from sklearn.metrics import accuracy_score, brier_score_loss, log_loss, roc_auc_score
from sklearn.model_selection import GroupShuffleSplit
from sklearn.pipeline import Pipeline
from sklearn.preprocessing import StandardScaler

SCHEMA_VERSION = "0.2.0"
MIN_LABELED_ROWS = 50
MIN_CLASS_ROWS = 5

CATEGORICAL_FEATURES = ["mapName"]
NUMERIC_FEATURES = [
    "phase",
    "relativeX",
    "relativeY",
    "normalizedCenterDistance",
    "distanceToEdgeM",
    "memberCount",
    "spreadM",
    "vehicleMemberRate",
    "enemyTeams100m",
    "enemyTeams300m",
    "enemyTeams500m",
    "relativeElevation300m",
]
FEATURES = CATEGORICAL_FEATURES + NUMERIC_FEATURES
TARGETS = {
    "next_zone_contains": "nextZoneContains",
    "survives_60s": "survives60s",
    "survives_120s": "survives120s",
}


def read_json_lines(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    with path.open("r", encoding="utf-8") as source:
        for line_number, line in enumerate(source, start=1):
            if not line.strip():
                continue
            try:
                value = json.loads(line)
            except json.JSONDecodeError as error:
                raise ValueError(f"invalid JSON on line {line_number}: {error}") from error
            if not isinstance(value, dict):
                raise ValueError(f"line {line_number} must contain a JSON object")
            rows.append(value)
    return rows


def feature_matrix(rows: Iterable[dict[str, Any]]) -> list[dict[str, Any]]:
    matrix = []
    for row in rows:
        item: dict[str, Any] = {"mapName": row.get("mapName") or "Unknown"}
        for name in NUMERIC_FEATURES:
            value = row.get(name)
            missing = not isinstance(value, (int, float)) or isinstance(value, bool)
            item[name] = 0.0 if missing else float(value)
            item[f"{name}__missing"] = float(missing)
        matrix.append(item)
    return matrix


def build_pipeline() -> Pipeline:
    classifier = LogisticRegression(
        class_weight="balanced",
        max_iter=1_500,
        solver="liblinear",
        random_state=42,
    )
    return Pipeline(
        [
            ("features", DictVectorizer(sparse=True)),
            ("scale", StandardScaler(with_mean=False)),
            ("classifier", classifier),
        ]
    )


def evaluate_group_holdout(
    rows: list[dict[str, Any]], labels: np.ndarray
) -> dict[str, float | int | str | None]:
    groups = np.asarray([str(row.get("matchId", "unknown")) for row in rows])
    unique_groups = np.unique(groups)
    if len(unique_groups) < 5:
        return {
            "strategy": "not_enough_matches",
            "testRows": 0,
            "accuracy": None,
            "brierScore": None,
            "logLoss": None,
            "rocAuc": None,
        }

    splitter = GroupShuffleSplit(n_splits=1, test_size=0.2, random_state=42)
    train_index, test_index = next(splitter.split(rows, labels, groups))
    train_labels = labels[train_index]
    test_labels = labels[test_index]
    if len(np.unique(train_labels)) < 2 or len(np.unique(test_labels)) < 2:
        return {
            "strategy": "group_split_has_one_class",
            "testRows": int(len(test_index)),
            "accuracy": None,
            "brierScore": None,
            "logLoss": None,
            "rocAuc": None,
        }

    matrix = feature_matrix(rows)
    model = build_pipeline()
    model.fit([matrix[index] for index in train_index], train_labels)
    probabilities = model.predict_proba([matrix[index] for index in test_index])[:, 1]
    predictions = (probabilities >= 0.5).astype(int)
    return {
        "strategy": "match_group_holdout",
        "testRows": int(len(test_index)),
        "accuracy": round(float(accuracy_score(test_labels, predictions)), 6),
        "brierScore": round(float(brier_score_loss(test_labels, probabilities)), 6),
        "logLoss": round(float(log_loss(test_labels, probabilities)), 6),
        "rocAuc": round(float(roc_auc_score(test_labels, probabilities)), 6),
    }


def train(input_path: Path, output_directory: Path) -> dict[str, Any]:
    rows = read_json_lines(input_path)
    if not rows:
        raise ValueError("training input contains no rows")
    output_directory.mkdir(parents=True, exist_ok=True)
    trained_at = datetime.now(UTC).isoformat()
    reports: list[dict[str, Any]] = []

    for model_name, target_key in TARGETS.items():
        labeled = [row for row in rows if isinstance(row.get(target_key), bool)]
        labels = np.asarray([int(row[target_key]) for row in labeled], dtype=np.int8)
        class_counts = {
            "false": int(np.sum(labels == 0)),
            "true": int(np.sum(labels == 1)),
        }
        if len(labeled) < MIN_LABELED_ROWS or min(class_counts.values(), default=0) < MIN_CLASS_ROWS:
            reports.append(
                {
                    "model": model_name,
                    "status": "skipped",
                    "reason": "not_enough_labeled_rows_or_class_balance",
                    "rows": len(labeled),
                    "classCounts": class_counts,
                }
            )
            continue

        evaluation = evaluate_group_holdout(labeled, labels)
        pipeline = build_pipeline()
        pipeline.fit(feature_matrix(labeled), labels)
        model_path = output_directory / f"{model_name}.joblib"
        artifact = {
            "schemaVersion": SCHEMA_VERSION,
            "trainedAt": trained_at,
            "target": target_key,
            "features": FEATURES,
            "pipeline": pipeline,
            "rowCount": len(labeled),
            "classCounts": class_counts,
            "evaluation": evaluation,
        }
        joblib.dump(artifact, model_path, compress=3)
        reports.append(
            {
                "model": model_name,
                "status": "trained",
                "rows": len(labeled),
                "classCounts": class_counts,
                "evaluation": evaluation,
                "file": model_path.name,
                "sha256": sha256_file(model_path),
            }
        )

    report = {
        "schemaVersion": SCHEMA_VERSION,
        "trainedAt": trained_at,
        "inputRows": len(rows),
        "models": reports,
    }
    manifest_path = output_directory / "manifest.json"
    manifest_path.write_text(
        json.dumps(report, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    return report


def predict(model_directory: Path, input_path: Path, output_path: Path) -> dict[str, Any]:
    rows = read_json_lines(input_path)
    model_files = sorted(model_directory.glob("*.joblib"))
    if not model_files:
        raise ValueError("model directory contains no trained models")
    loaded = [(path.stem, joblib.load(path)) for path in model_files]
    matrix = feature_matrix(rows)
    with output_path.open("w", encoding="utf-8") as output:
        for index, row in enumerate(rows):
            result = {
                "matchId": row.get("matchId"),
                "phase": row.get("phase"),
                "predictions": {},
            }
            for model_name, artifact in loaded:
                probability = artifact["pipeline"].predict_proba([matrix[index]])[0, 1]
                result["predictions"][model_name] = round(float(probability), 8)
            output.write(json.dumps(result, ensure_ascii=False) + "\n")
    return {"schemaVersion": SCHEMA_VERSION, "rows": len(rows), "models": len(loaded)}


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for block in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(prog="pubg-trainer")
    commands = parser.add_subparsers(dest="command", required=True)

    train_parser = commands.add_parser("train")
    train_parser.add_argument("--input", type=Path, required=True)
    train_parser.add_argument("--output-dir", type=Path, required=True)

    predict_parser = commands.add_parser("predict")
    predict_parser.add_argument("--model-dir", type=Path, required=True)
    predict_parser.add_argument("--input", type=Path, required=True)
    predict_parser.add_argument("--output", type=Path, required=True)
    return parser


def main(argv: list[str] | None = None) -> int:
    arguments = build_parser().parse_args(argv)
    try:
        if arguments.command == "train":
            report = train(arguments.input, arguments.output_dir)
        else:
            report = predict(arguments.model_dir, arguments.input, arguments.output)
    except Exception as error:  # sidecar boundary: return one actionable error to Rust
        print(str(error), file=sys.stderr)
        return 1
    print(json.dumps(report, ensure_ascii=False, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
