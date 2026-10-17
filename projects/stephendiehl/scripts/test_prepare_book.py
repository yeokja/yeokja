import json
from pathlib import Path
import tempfile
import unittest

import prepare_book

POST = '---\ndate: "2013-07-07"\ntags: ["haskell", "category theory"]\nurl: "https://www.stephendiehl.com/posts/monads/"\n---\n\n# Monads Made Difficult\n\nBody $x$.\n\n## Categories\n'


class RenderTests(unittest.TestCase):
    def test_front_matter_becomes_a_visible_meta_line(self):
        title, date, page = prepare_book.render_post(POST, "ko")
        self.assertEqual((title, date), ("Monads Made Difficult", "2013-07-07"))
        self.assertTrue(page.startswith("# Monads Made Difficult\n\n<p class=\"post-meta\">"))
        self.assertIn('<time datetime="2013-07-07">2013년 7월 7일</time>', page)
        self.assertIn("태그: haskell, category theory", page)
        self.assertIn('<a href="https://www.stephendiehl.com/posts/monads/">원문</a>', page)
        self.assertIn("한국어 번역(원문에서 변경됨)", page)
        self.assertIn("\n\nBody $x$.\n\n## Categories\n", page)
        self.assertNotIn("---", page)

    def test_english_meta_line_has_the_same_shape(self):
        _, _, page = prepare_book.render_post(POST, "en")
        self.assertIn("July 7, 2013", page)
        self.assertIn("Tags: haskell, category theory", page)
        self.assertIn(">Original</a>", page)
        self.assertNotIn("한국어", page)
        self.assertEqual(page.count("\n#"), prepare_book.render_post(POST, "ko")[2].count("\n#"))

    def test_post_without_tags_omits_the_label(self):
        text = '---\ndate: "2020-01-02"\ntags: []\nurl: "u"\n---\n\n# T\n\nx\n'
        _, _, page = prepare_book.render_post(text, "ko")
        self.assertNotIn("태그", page)
        self.assertIn("2020년 1월 2일", page)

    def test_missing_h1_fails(self):
        with self.assertRaises(ValueError):
            prepare_book.render_post("---\ndate: \"2020-01-01\"\n---\n\nno heading\n", "ko")


class SummaryTests(unittest.TestCase):
    def test_summary_groups_by_year_newest_first_with_prefix_and_suffix(self):
        entries = [("b", "B [x]", "2026-10-11"), ("a", "A", "2026-01-01"), ("c", "C", "2013-07-07")]
        text = prepare_book.summary(entries, "ko")
        self.assertEqual(text.splitlines(), [
            "[소개](index.md)", "",
            "# 2026", "",
            "- [B \\[x\\]](posts/b.md)",
            "- [A](posts/a.md)", "",
            "# 2013", "",
            "- [C](posts/c.md)", "",
            "---", "",
            "[라이선스와 고지](license.md)",
        ])


class PrepareTests(unittest.TestCase):
    def test_prepare_lays_out_both_books_alike(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory) / "src"
            (root / "posts").mkdir(parents=True)
            (root / "posts" / "monads.md").write_text(POST)
            (root / "index.md").write_text("# Stephen Diehl\n\nHi.\n")
            (root / "license.md").write_text("# License\n\nCC BY-SA.\n")
            (root / "manifest.json").write_text(json.dumps({"posts": [{"slug": "monads", "date": "2013-07-07"}]}))
            for lang in ("en", "ko"):
                out = Path(directory) / lang
                self.assertEqual(prepare_book.prepare(root, out, lang), 1)
                self.assertIn('[preprocessor.katex]', (out / "book.toml").read_text())
                self.assertIn('additional-css = ["custom.css"]', (out / "book.toml").read_text())
                self.assertNotIn("site-url", (out / "book.toml").read_text())
                self.assertTrue((out / "custom.css").exists())
                self.assertIn("(posts/monads.md)", (out / "src" / "SUMMARY.md").read_text())
                self.assertIn("# Monads Made Difficult", (out / "src" / "posts" / "monads.md").read_text())
            ko_license = (Path(directory) / "ko" / "src" / "license.md").read_text()
            self.assertTrue(ko_license.startswith("# 라이선스와 고지"))
            self.assertIn("정중한 부탁이 아니라 엄격한 규칙", ko_license)
            self.assertTrue(ko_license.endswith("# License\n\nCC BY-SA.\n"))
            ko_index = (Path(directory) / "ko" / "src" / "index.md").read_text()
            self.assertTrue(ko_index.startswith("# Stephen Diehl\n\nHi.\n\n## 이 사이트에 대하여"))
            self.assertIn("claude-opus-5-5", ko_index)
            en_license = (Path(directory) / "en" / "src" / "license.md").read_text()
            self.assertEqual(en_license, "# License\n\nCC BY-SA.\n")


if __name__ == "__main__":
    unittest.main()
