from pathlib import Path
import tempfile
import unittest

from extract_heading_ids import extract


class ExtractTests(unittest.TestCase):
    def test_only_markdown_headings_with_permalinks(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / 'graph').mkdir()
            (root / 'graph' / 'dijkstra.html').write_text(
                '<h1>Template title</h1><article class="md-content__inner">'
                '<h1 id="dijkstra">Dijkstra<a class="headerlink" href="#dijkstra">¶</a></h1>'
                '<div><h3 id="raw">raw html</h3></div>'
                '<h2 id="algorithm">Algorithm<a class="headerlink" href="#algorithm">¶</a></h2>'
                '</article>')
            self.assertEqual(extract(root), {'graph/dijkstra.md': ['dijkstra', 'algorithm']})


if __name__ == '__main__':
    unittest.main()
