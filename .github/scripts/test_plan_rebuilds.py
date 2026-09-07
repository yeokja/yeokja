from __future__ import annotations

import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
PLAN_SCRIPT = REPOSITORY_ROOT / ".github" / "scripts" / "plan-rebuilds.sh"


def run_git(args: list[str], cwd: Path) -> None:
    subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True)


class PlanRebuildsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        run_git(["init", "-q"], self.root)
        run_git(["config", "user.email", "test@example.com"], self.root)
        run_git(["config", "user.name", "Test"], self.root)

        for name in ("alpha", "beta"):
            project = self.root / "projects" / name
            (project / "state").mkdir(parents=True)
            self.write(project / ".gitignore", "ko/\n")
            self.write(project / "yeokja.toml", f"[project]\nname = \"{name}\"\n")
            (project / "ko").mkdir()
            self.write(project / "ko" / "book.md", f"{name} 번역")
        run_git(["add", "-A"], self.root)
        run_git(["commit", "-q", "-m", "initial"], self.root)

        self.entries = [
            {
                "project": "alpha",
                "target": "html",
                "artifact": "dist-alpha",
                "artifact_path": "projects/alpha/dist",
            },
            {
                "project": "beta",
                "target": "pdf",
                "artifact": "dist-beta-pdf",
                "artifact_path": "projects/beta/output/pdf",
                "rebuild_daily": True,
            },
        ]
        self.projects_json = self.root / "projects.json"
        self.projects_json.write_text(json.dumps(self.entries), encoding="utf-8")
        self.recorded = self.root / "recorded"
        self.out = self.root / "out"

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    @staticmethod
    def write(path: Path, contents: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents, encoding="utf-8")

    def run_plan(self, *extra: str) -> subprocess.CompletedProcess[str]:
        self.assertTrue(PLAN_SCRIPT.is_file(), f"missing script: {PLAN_SCRIPT}")
        env = dict(os.environ, YEOKJA_FINGERPRINT_DATE="2026-01-01")
        return subprocess.run(
            [
                "bash",
                str(PLAN_SCRIPT),
                str(self.projects_json),
                str(self.recorded),
                str(self.out),
                *extra,
            ],
            cwd=self.root,
            env=env,
            check=False,
            capture_output=True,
            text=True,
        )

    def selected_artifacts(self, result: subprocess.CompletedProcess[str]) -> list[str]:
        self.assertEqual(result.returncode, 0, result.stderr)
        matrix = json.loads(result.stdout)
        self.assertEqual(len(result.stdout.strip().splitlines()), 1, "one-line JSON")
        return [entry["artifact"] for entry in matrix]

    def test_without_records_every_entry_is_rebuilt(self) -> None:
        result = self.run_plan()

        self.assertEqual(
            self.selected_artifacts(result), ["dist-alpha", "dist-beta-pdf"]
        )
        self.assertTrue((self.out / "dist-alpha").is_file())
        self.assertTrue((self.out / "dist-beta-pdf").is_file())
        self.assertIn("no recorded fingerprint", result.stderr)

    def test_matching_records_skip_entries_and_keep_matrix_fields(self) -> None:
        first = self.run_plan()
        self.selected_artifacts(first)
        self.recorded.mkdir()
        (self.recorded / "dist-alpha").write_text(
            (self.out / "dist-alpha").read_text(encoding="utf-8"), encoding="utf-8"
        )

        result = self.run_plan()

        self.assertEqual(self.selected_artifacts(result), ["dist-beta-pdf"])
        self.assertEqual(json.loads(result.stdout), [self.entries[1]])
        self.assertIn("dist-alpha: up to date", result.stderr)

    def test_changed_input_rebuilds_entry(self) -> None:
        first = self.run_plan()
        self.selected_artifacts(first)
        self.recorded.mkdir()
        for artifact in ("dist-alpha", "dist-beta-pdf"):
            (self.recorded / artifact).write_text(
                (self.out / artifact).read_text(encoding="utf-8"), encoding="utf-8"
            )
        self.write(self.root / "projects" / "alpha" / "ko" / "book.md", "고친 번역")

        result = self.run_plan()

        self.assertEqual(self.selected_artifacts(result), ["dist-alpha"])
        self.assertIn("dist-alpha: rebuild (fingerprint", result.stderr)

    def test_daily_entry_is_rebuilt_when_the_date_changes(self) -> None:
        first = self.run_plan()
        self.selected_artifacts(first)
        self.recorded.mkdir()
        for artifact in ("dist-alpha", "dist-beta-pdf"):
            (self.recorded / artifact).write_text(
                (self.out / artifact).read_text(encoding="utf-8"), encoding="utf-8"
            )

        env = dict(os.environ, YEOKJA_FINGERPRINT_DATE="2026-01-02")
        result = subprocess.run(
            [
                "bash",
                str(PLAN_SCRIPT),
                str(self.projects_json),
                str(self.recorded),
                str(self.out),
            ],
            cwd=self.root,
            env=env,
            check=False,
            capture_output=True,
            text=True,
        )

        self.assertEqual(self.selected_artifacts(result), ["dist-beta-pdf"])

    def test_force_rebuilds_everything(self) -> None:
        first = self.run_plan()
        self.selected_artifacts(first)
        self.recorded.mkdir()
        for artifact in ("dist-alpha", "dist-beta-pdf"):
            (self.recorded / artifact).write_text(
                (self.out / artifact).read_text(encoding="utf-8"), encoding="utf-8"
            )

        result = self.run_plan("--force")

        self.assertEqual(
            self.selected_artifacts(result), ["dist-alpha", "dist-beta-pdf"]
        )
        self.assertIn("forced", result.stderr)

    def test_nothing_to_rebuild_yields_empty_array(self) -> None:
        first = self.run_plan()
        self.selected_artifacts(first)
        self.recorded.mkdir()
        for artifact in ("dist-alpha", "dist-beta-pdf"):
            (self.recorded / artifact).write_text(
                (self.out / artifact).read_text(encoding="utf-8"), encoding="utf-8"
            )

        result = self.run_plan()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout.strip(), "[]")


if __name__ == "__main__":
    unittest.main()
