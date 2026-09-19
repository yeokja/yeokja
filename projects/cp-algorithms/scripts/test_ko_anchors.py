import importlib.util
from pathlib import Path
import unittest

import markdown

spec = importlib.util.spec_from_file_location(
    'ko_anchors', Path(__file__).parent.parent / 'overlay' / 'hooks' / 'ko_anchors.py')
ko_anchors = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ko_anchors)


def render(text, page, ids):
    ko_anchors._state.update(page=page, ids=ids)
    md = markdown.Markdown(extensions=['attr_list', ko_anchors.EnglishIdsExtension(), 'toc'],
                           extension_configs={'toc': {'permalink': True}})
    return md.convert(text)


class AnchorTests(unittest.TestCase):
    def test_korean_headings_get_english_ids_and_permalinks(self):
        html = render('# 다익스트라\n\n## 알고리즘\n', 'graph/dijkstra.md',
                      {'graph/dijkstra.md': ['dijkstra', 'algorithm']})
        self.assertIn('id="dijkstra"', html)
        self.assertIn('href="#algorithm"', html)

    def test_heading_count_mismatch_fails(self):
        with self.assertRaises(ValueError):
            render('# 하나\n', 'a.md', {'a.md': ['one', 'two']})

    def test_generated_page_is_skipped(self):
        self.assertIn('<h1', render('# 태그\n', 'tags.md', {'tags.md': ['tags', 'x']}))


if __name__ == '__main__':
    unittest.main()
