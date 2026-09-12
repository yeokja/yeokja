from pathlib import Path
import tempfile
import unittest
from build_print import Page, build_print, rewrite


class PrintTests(unittest.TestCase):
    def test_chapter_and_glossary_links_keep_their_targets(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'sub').mkdir()
            (root / 'toc.html').write_text('<a href="a.html">A</a><a href="sub/b.html">B</a>')
            (root / 'a.html').write_text('<main><h1 id="query">쿼리</h1><a href="sub/b.html#query">용어집</a><a href="sub/b.html">다음 장</a></main>')
            (root / 'sub/b.html').write_text('<main><h1 id="glossary">용어집</h1><span id="query">쿼리</span><a href="../a.html#query">쿼리 장</a><img src="../img/a.png"></main>')
            (root / 'print.html').write_text('<html><main>OLD</main><script src="book.js"></script></html>')
            build_print(root)
            result = (root / 'print.html').read_text()
            self.assertIn('href="#chapter-1-query">용어집</a>', result)
            self.assertIn('href="#chapter-0-query">쿼리 장</a>', result)
            self.assertIn('href="#chapter-1">다음 장</a>', result)
            self.assertEqual(result.count('id="chapter-0-query"'), 1)
            self.assertEqual(result.count('id="chapter-1-query"'), 1)
            self.assertIn('src="img/a.png"', result)
            self.assertIn('<script src="book.js"></script>', result)
            self.assertNotIn('OLD', result)

    def test_quoted_attribute_contents_and_data_attributes_are_untouched(self):
        page = Page("<main><img alt=\"Example src='photo.png'\" src=\"photo.png\"><div data-id=\"preserved\"></div></main>")
        result = rewrite(page, 'sub/a.html', {'sub/a.html': 'chapter-0'})
        self.assertIn("alt=\"Example src='photo.png'\"", result)
        self.assertIn('src="sub/photo.png"', result)
        self.assertIn('data-id="preserved"', result)


if __name__ == '__main__':
    unittest.main()
