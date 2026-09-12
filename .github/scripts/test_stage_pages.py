from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path


REPOSITORY_ROOT = Path(__file__).resolve().parents[2]
STAGE_SCRIPT = REPOSITORY_ROOT / ".github" / "scripts" / "stage-pages.sh"


class StagePagesTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary_directory = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary_directory.name)
        self.artifacts = self.root / "artifacts"
        self.site = self.root / "site"
        self.landing = self.root / "landing"
        self.fingerprints = self.root / "fingerprints"
        self.artifacts.mkdir()
        self.site.mkdir()
        self.landing.mkdir()
        self.fingerprints.mkdir()
        self.write(self.site / "index.html", "published root")
        self.write(self.landing / "index.html", "new root")
        self.write(self.landing / "favicon.svg", "new favicon")

    def tearDown(self) -> None:
        self.temporary_directory.cleanup()

    @staticmethod
    def write(path: Path, contents: str) -> None:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(contents, encoding="utf-8")

    def add_site_artifact(
        self, artifact: str, contents: str, source_directory: str = "site"
    ) -> None:
        self.write(
            self.artifacts / artifact / source_directory / "index.html", contents
        )

    def add_required_artifacts(self) -> None:
        self.add_site_artifact("dist-devguide", "devguide")
        self.write(
            self.artifacts
            / "dist-chisel-book-pdf"
            / "Digital-Design-with-Chisel-ko.pdf",
            "chisel pdf",
        )

    def add_fingerprint(self, artifact: str, value: str) -> None:
        self.write(self.fingerprints / artifact, value + "\n")

    def run_stage(
        self, with_fingerprints: bool = False
    ) -> subprocess.CompletedProcess[str]:
        self.assertTrue(STAGE_SCRIPT.is_file(), f"missing script: {STAGE_SCRIPT}")
        command = [
            "bash",
            str(STAGE_SCRIPT),
            str(self.artifacts),
            str(self.site),
            str(self.landing),
        ]
        if with_fingerprints:
            command.append(str(self.fingerprints))
        return subprocess.run(command, check=False, capture_output=True, text=True)

    def test_missing_published_baseline_index_fails(self) -> None:
        (self.site / "index.html").unlink()
        self.add_required_artifacts()

        result = self.run_stage()

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("published Pages baseline is missing index.html", result.stderr)

    def test_missing_devguide_in_staged_tree_fails(self) -> None:
        result = self.run_stage()

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            "required devguide site is missing from the staged tree", result.stderr
        )

    def test_missing_chisel_book_in_staged_tree_fails(self) -> None:
        self.add_site_artifact("dist-devguide", "devguide")

        result = self.run_stage()

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            "required chisel-book PDF is missing from the staged tree", result.stderr
        )

    def test_required_files_preserved_from_published_tree_suffice(self) -> None:
        # plan 잡이 빌드를 건너뛰면 산출물은 없지만 보존된 트리에 이미 있습니다.
        self.write(self.site / "devguide" / "index.html", "published devguide")
        self.write(
            self.site / "chisel-book" / "Digital-Design-with-Chisel-ko.pdf",
            "published pdf",
        )

        result = self.run_stage()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.site / "devguide" / "index.html").read_text(encoding="utf-8"),
            "published devguide",
        )

    def test_fingerprints_recorded_only_for_overlaid_artifacts(self) -> None:
        self.add_required_artifacts()
        self.add_site_artifact("dist-mil", "new mil")
        self.write(
            self.artifacts / "dist-napkin-pdf" / "Napkin-ko.pdf", "new pdf"
        )
        self.add_site_artifact("dist-pypy", "new pypy")
        self.add_site_artifact("dist-pypy", "new rpython", "rpython-site")
        for artifact in (
            "dist-devguide",
            "dist-chisel-book-pdf",
            "dist-mil",
            "dist-napkin-pdf",
            "dist-pypy",
            "dist-tpil",
        ):
            self.add_fingerprint(artifact, f"fp-{artifact}")
        recorded = self.site / "build-fingerprints"
        self.write(recorded / "dist-tpil", "old-tpil\n")
        self.write(recorded / "dist-mil", "old-mil\n")

        result = self.run_stage(with_fingerprints=True)

        self.assertEqual(result.returncode, 0, result.stderr)
        for artifact in (
            "dist-devguide",
            "dist-chisel-book-pdf",
            "dist-mil",
            "dist-napkin-pdf",
            "dist-pypy",
        ):
            self.assertEqual(
                (recorded / artifact).read_text(encoding="utf-8"),
                f"fp-{artifact}\n",
                artifact,
            )
        # tpil had no artifact: its previously recorded fingerprint stays.
        self.assertEqual(
            (recorded / "dist-tpil").read_text(encoding="utf-8"), "old-tpil\n"
        )
        self.assertFalse((recorded / "dist-peps").exists())

    def test_overlay_without_fingerprint_warns_and_records_nothing(self) -> None:
        self.add_required_artifacts()
        self.add_fingerprint("dist-devguide", "fp-devguide")

        result = self.run_stage(with_fingerprints=True)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("no fingerprint for dist-chisel-book-pdf", result.stderr)
        recorded = self.site / "build-fingerprints"
        self.assertTrue((recorded / "dist-devguide").is_file())
        self.assertFalse((recorded / "dist-chisel-book-pdf").exists())

    def test_without_fingerprints_dir_nothing_is_recorded(self) -> None:
        self.add_required_artifacts()

        result = self.run_stage()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.site / "build-fingerprints").exists())

    def test_missing_legacy_artifact_preserves_published_tree(self) -> None:
        self.write(self.site / "mil" / "old.html", "published")
        self.add_required_artifacts()

        result = self.run_stage()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.site / "mil" / "old.html").read_text(encoding="utf-8"),
            "published",
        )

    def test_successful_artifact_replaces_old_subtree(self) -> None:
        self.write(self.site / "mil" / "old.html", "old")
        self.add_site_artifact("dist-mil", "new")
        self.add_required_artifacts()

        result = self.run_stage()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.site / "mil" / "old.html").exists())
        self.assertEqual(
            (self.site / "mil" / "index.html").read_text(encoding="utf-8"),
            "new",
        )

    def test_rust_forge_artifact_replaces_old_subtree(self) -> None:
        self.write(self.site / "rust-forge" / "old.html", "old")
        self.add_site_artifact("dist-rust-forge", "new")
        self.add_required_artifacts()

        result = self.run_stage()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.site / "rust-forge" / "old.html").exists())
        self.assertEqual(
            (self.site / "rust-forge" / "index.html").read_text(encoding="utf-8"),
            "new",
        )

    def test_zero_to_nix_artifact_replaces_old_subtree(self) -> None:
        self.write(self.site / "zero-to-nix" / "old.html", "old")
        self.add_site_artifact("dist-zero-to-nix", "new")
        self.add_required_artifacts()

        result = self.run_stage()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.site / "zero-to-nix" / "old.html").exists())
        self.assertEqual(
            (self.site / "zero-to-nix" / "index.html").read_text(encoding="utf-8"),
            "new",
        )

    def test_nix_dev_artifact_replaces_old_subtree(self) -> None:
        self.write(self.site / "nix-dev" / "old.html", "old")
        self.add_site_artifact("dist-nix-dev", "new")
        self.add_required_artifacts()

        result = self.run_stage()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.site / "nix-dev" / "old.html").exists())
        self.assertEqual(
            (self.site / "nix-dev" / "index.html").read_text(encoding="utf-8"),
            "new",
        )

    def test_raytracing_artifact_publishes_books_assets_and_fingerprint(self) -> None:
        self.write(self.site / "raytracing" / "old.html", "old")
        self.add_site_artifact("dist-raytracing", "Korean books")
        for relative_path in (
            "books/RayTracingInOneWeekend.html",
            "images/cover.jpg",
            "style/markdeep.min.js",
        ):
            self.write(
                self.artifacts / "dist-raytracing" / "site" / relative_path,
                relative_path,
            )
        self.add_fingerprint("dist-raytracing", "new-raytracing")
        self.add_required_artifacts()

        result = self.run_stage(with_fingerprints=True)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.site / "raytracing" / "old.html").exists())
        self.assertEqual(
            (self.site / "raytracing" / "index.html").read_text(encoding="utf-8"),
            "Korean books",
        )
        for relative_path in (
            "books/RayTracingInOneWeekend.html",
            "images/cover.jpg",
            "style/markdeep.min.js",
        ):
            self.assertEqual(
                (self.site / "raytracing" / relative_path).read_text(encoding="utf-8"),
                relative_path,
            )
        self.assertEqual(
            (self.site / "build-fingerprints" / "dist-raytracing").read_text(
                encoding="utf-8"
            ),
            "new-raytracing\n",
        )

    def test_missing_raytracing_artifact_preserves_published_books(self) -> None:
        self.write(self.site / "raytracing" / "index.html", "published books")
        self.write(
            self.site / "build-fingerprints" / "dist-raytracing", "published-fp\n"
        )
        self.add_fingerprint("dist-raytracing", "unpublished-fp")
        self.add_required_artifacts()

        result = self.run_stage(with_fingerprints=True)

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (self.site / "raytracing" / "index.html").read_text(encoding="utf-8"),
            "published books",
        )
        self.assertEqual(
            (self.site / "build-fingerprints" / "dist-raytracing").read_text(
                encoding="utf-8"
            ),
            "published-fp\n",
        )

    def test_pypy_artifact_replaces_pypy_and_rpython_together(self) -> None:
        self.write(self.site / "pypy" / "old.html", "old pypy")
        self.write(self.site / "rpython" / "old.html", "old rpython")
        self.add_site_artifact("dist-pypy", "new pypy")
        self.add_site_artifact("dist-pypy", "new rpython", "rpython-site")
        self.add_required_artifacts()

        result = self.run_stage()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.site / "pypy" / "old.html").exists())
        self.assertFalse((self.site / "rpython" / "old.html").exists())
        self.assertEqual(
            (self.site / "pypy" / "index.html").read_text(encoding="utf-8"),
            "new pypy",
        )
        self.assertEqual(
            (self.site / "rpython" / "index.html").read_text(encoding="utf-8"),
            "new rpython",
        )

    def test_napkin_artifacts_replace_html_and_refresh_downloads(self) -> None:
        self.write(self.site / "napkin" / "old.html", "old")
        self.add_site_artifact("dist-napkin-html", "new html")
        self.write(
            self.artifacts / "dist-napkin-pdf" / "Napkin-ko.pdf", "new pdf"
        )
        self.write(
            self.artifacts / "dist-napkin-epub" / "Napkin-ko.epub", "new epub"
        )
        self.add_required_artifacts()

        result = self.run_stage()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.site / "napkin" / "old.html").exists())
        self.assertEqual(
            (self.site / "napkin" / "index.html").read_text(encoding="utf-8"),
            "new html",
        )
        self.assertEqual(
            (self.site / "napkin" / "Napkin-ko.pdf").read_text(encoding="utf-8"),
            "new pdf",
        )
        self.assertEqual(
            (self.site / "napkin" / "Napkin-ko.epub").read_text(encoding="utf-8"),
            "new epub",
        )

    def test_chisel_book_artifact_refreshes_pdf_download(self) -> None:
        self.add_required_artifacts()
        self.write(
            self.site / "chisel-book" / "Digital-Design-with-Chisel-ko.pdf",
            "old pdf",
        )
        self.write(
            self.artifacts
            / "dist-chisel-book-pdf"
            / "Digital-Design-with-Chisel-ko.pdf",
            "new pdf",
        )

        result = self.run_stage()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(
            (
                self.site
                / "chisel-book"
                / "Digital-Design-with-Chisel-ko.pdf"
            ).read_text(encoding="utf-8"),
            "new pdf",
        )

    def test_required_devguide_and_landing_files_are_refreshed(self) -> None:
        self.write(self.site / "devguide" / "old.html", "old devguide")
        self.write(self.site / "favicon.svg", "old favicon")
        self.add_required_artifacts()

        result = self.run_stage()

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertFalse((self.site / "devguide" / "old.html").exists())
        self.assertEqual(
            (self.site / "devguide" / "index.html").read_text(encoding="utf-8"),
            "devguide",
        )
        self.assertEqual(
            (self.site / "index.html").read_text(encoding="utf-8"), "new root"
        )
        self.assertEqual(
            (self.site / "favicon.svg").read_text(encoding="utf-8"),
            "new favicon",
        )


if __name__ == "__main__":
    unittest.main()
