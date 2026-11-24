"""Tests for prepare.py and en_anchors.py.

Run inside the project devShell (docutils comes from the upstream venv):

    python3 -m unittest discover -s scripts -p 'test_*.py'
"""

import shutil
import tempfile
import unittest
from pathlib import Path

from en_anchors import reference_section_ids
from prepare import BEGIN_MARKER, MANAGED_BLOCK, localize_numref_labels, prepare

UPSTREAM = Path(__file__).resolve().parents[1] / "upstream"
FILES = ["conf.py", "index.rst", "_templates/layout.html", "_templates/genindex.html"]


class LinkTree:
    """A tree whose files are symlinks into upstream, like `yeokja assemble` builds."""

    def __enter__(self) -> Path:
        self.directory = tempfile.TemporaryDirectory()
        root = Path(self.directory.name)
        for name in FILES:
            (root / name).parent.mkdir(parents=True, exist_ok=True)
            (root / name).symlink_to(UPSTREAM / name)
        return root

    def __exit__(self, *args) -> None:
        self.directory.cleanup()


class PrepareTest(unittest.TestCase):
    def test_patches_pinned_upstream_without_writing_through_links(self) -> None:
        before = {name: (UPSTREAM / name).read_text(encoding="utf-8") for name in FILES}
        with LinkTree() as root:
            prepare(root)
            for name in FILES:
                self.assertFalse((root / name).is_symlink(), name)
            conf = (root / "conf.py").read_text(encoding="utf-8")
            self.assertTrue(conf.endswith(MANAGED_BLOCK + "\n"))
            index = (root / "index.rst").read_text(encoding="utf-8")
            self.assertIn("   :caption: 목차\n", index)
            self.assertIn("   약어 목록 <genindex>\n", index)
            for template in ("_templates/layout.html", "_templates/genindex.html"):
                self.assertNotIn("Acronym Glossary", (root / template).read_text(encoding="utf-8"))
            self.assertNotIn("This glossary", (root / "_templates/genindex.html").read_text(encoding="utf-8"))
        for name in FILES:
            self.assertEqual((UPSTREAM / name).read_text(encoding="utf-8"), before[name], name)

    def test_second_run_is_a_no_op(self) -> None:
        with LinkTree() as root:
            prepare(root)
            first = {name: (root / name).read_text(encoding="utf-8") for name in FILES}
            prepare(root)
            for name in FILES:
                self.assertEqual((root / name).read_text(encoding="utf-8"), first[name], name)
            self.assertEqual((root / "conf.py").read_text(encoding="utf-8").count(BEGIN_MARKER), 1)

    def test_managed_conf_executes_and_overrides(self) -> None:
        with LinkTree() as root:
            prepare(root)
            namespace = {"__file__": str(root / "conf.py")}
            code = (root / "conf.py").read_text(encoding="utf-8").replace(
                "version = get_version()", "version = 'Version 7.1-dev'"
            )
            import os

            os.environ.setdefault("YEOKJA_ROOT", str(UPSTREAM.parent))
            exec(compile(code, "conf.py", "exec"), namespace)
        self.assertEqual(namespace["language"], "ko")
        self.assertEqual(namespace["numfig_format"], {"figure": "그림 %s.", "table": "표 %s."})
        self.assertEqual(namespace["version"], "버전 7.1-dev")
        self.assertNotIn("sphinxcontrib.spelling", namespace["extensions"])
        self.assertNotIn("sphinx_multiversion", namespace["extensions"])
        self.assertIn("en_anchors", namespace["extensions"])
        self.assertIn("chapters.rst", namespace["exclude_patterns"])
        self.assertIn("announcement", namespace["html_theme_options"])
        self.assertNotIn("gtag", namespace["setup"].__code__.co_names)

    def test_upstream_drift_fails_loudly(self) -> None:
        with LinkTree() as root:
            conf = root / "conf.py"
            text = conf.read_text(encoding="utf-8").replace("language = 'en'", "language = 'fr'")
            conf.unlink()
            conf.write_text(text, encoding="utf-8")
            with self.assertRaises(ValueError):
                prepare(root)

    def test_index_drift_fails_loudly(self) -> None:
        with LinkTree() as root:
            index = root / "index.rst"
            text = index.read_text(encoding="utf-8").replace("Acronym Glossary <genindex>", "Glossary <genindex>")
            index.unlink()
            index.write_text(text, encoding="utf-8")
            with self.assertRaises(ValueError):
                prepare(root)


class NumrefLabelTest(unittest.TestCase):
    def test_labels_become_korean_and_keep_number_and_target(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "part").mkdir()
            target = root / "part" / "a.rst"
            target.symlink_to(UPSTREAM / "technology" / "switch.rst")
            source = (UPSTREAM / "technology" / "switch.rst").read_text(encoding="utf-8")
            (root / "b.rst").write_text(
                "보기 :numref:`Figures %s <fig-osi>`\\ 와 :numref:`Table %s <tab-ops>`, "
                ":numref:`%s <fig-iphdr>`, :numref:`그림 %s <fig-x>`.\n",
                encoding="utf-8",
            )
            self.assertEqual(localize_numref_labels(root), 2)
            self.assertFalse(target.is_symlink())
            self.assertEqual((UPSTREAM / "technology" / "switch.rst").read_text(encoding="utf-8"), source)
            self.assertIn(":numref:`그림\n%s <fig-pipeline>`", target.read_text(encoding="utf-8"))
            self.assertEqual(
                (root / "b.rst").read_text(encoding="utf-8"),
                "보기 :numref:`그림 %s <fig-osi>`\\ 와 :numref:`표 %s <tab-ops>`, "
                ":numref:`%s <fig-iphdr>`, :numref:`그림 %s <fig-x>`.\n",
            )
            self.assertEqual(localize_numref_labels(root), 0)


class ReferenceSectionIdsTest(unittest.TestCase):
    def test_ids_follow_includes_and_substitutions(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            shutil.copy(UPSTREAM / "chapters.rst", root / "chapters.rst")
            (root / "part").mkdir()
            (root / "part" / "one.rst").write_text("Sliding Window\n--------------\n\nText.\n", encoding="utf-8")
            (root / "doc.rst").write_text(
                ".. include:: chapters.rst\n\n"
                "Chapter |Intro|:  Introduction\n==============================\n\n"
                "Prose with :numref:`Figure %s <fig-x>`.\n\n"
                ".. include:: part/one.rst\n",
                encoding="utf-8",
            )
            self.assertEqual(
                reference_section_ids(root / "doc.rst"),
                # docutils names a section before substitutions resolve, as the upstream site shows.
                ["chapter-intro-introduction", "sliding-window"],
            )


if __name__ == "__main__":
    unittest.main()
