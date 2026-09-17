import json
import tempfile
import unittest
from pathlib import Path

from pubg_trainer import train


class TrainerTests(unittest.TestCase):
    def test_skips_small_targets_and_writes_manifest(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "rows.jsonl"
            input_path.write_text(
                "\n".join(
                    json.dumps(
                        {
                            "matchId": f"match-{index}",
                            "mapName": "Baltic_Main",
                            "phase": 2,
                            "nextZoneContains": index % 2 == 0,
                        }
                    )
                    for index in range(10)
                ),
                encoding="utf-8",
            )
            output = root / "models"
            report = train(input_path, output)
            self.assertEqual(report["inputRows"], 10)
            self.assertTrue(all(item["status"] == "skipped" for item in report["models"]))
            self.assertTrue((output / "manifest.json").exists())

    def test_trains_balanced_target_with_match_group_holdout(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            input_path = root / "rows.jsonl"
            rows = []
            for index in range(100):
                rows.append(
                    {
                        "matchId": f"match-{index // 10}",
                        "mapName": "Baltic_Main" if index < 50 else "Desert_Main",
                        "phase": 2 + index % 4,
                        "relativeX": (index % 10 - 5) / 10,
                        "relativeY": ((index * 3) % 10 - 5) / 10,
                        "memberCount": 4,
                        "enemyTeams300m": index % 3,
                        "nextZoneContains": index % 2 == 0,
                    }
                )
            input_path.write_text(
                "\n".join(json.dumps(row) for row in rows), encoding="utf-8"
            )
            output = root / "models"
            report = train(input_path, output)
            next_zone = next(
                item for item in report["models"] if item["model"] == "next_zone_contains"
            )
            self.assertEqual(next_zone["status"], "trained")
            self.assertEqual(next_zone["rows"], 100)
            self.assertTrue((output / "next_zone_contains.joblib").exists())


if __name__ == "__main__":
    unittest.main()
