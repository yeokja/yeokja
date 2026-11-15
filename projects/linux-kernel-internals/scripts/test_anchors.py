import importlib.util
from pathlib import Path
import unittest

try:
    import markdown
except ImportError:
    # Python-Markdown은 nix devShell에만 있습니다. 없는 환경(PR 워크플로의 기본 python3)에서는 건너뜁니다.
    raise unittest.SkipTest('markdown is not installed')

spec = importlib.util.spec_from_file_location(
    'anchors', Path(__file__).parent.parent / 'overlay' / 'hooks' / 'anchors.py')
anchors = importlib.util.module_from_spec(spec)
spec.loader.exec_module(anchors)


def render(text, page, record, ids=None):
    anchors._state.update(page=page, ids=ids or {}, recorded={})
    md = markdown.Markdown(extensions=['attr_list', 'admonition', anchors.AnchorsExtension(record), 'toc'])
    return md.convert(text)


class AnchorTests(unittest.TestCase):
    def test_english_build_records_ids_in_document_order(self):
        render('# Dirty Pipe\n\n## The fix\n\n## The fix\n', 'a.md', record=True)
        self.assertEqual(anchors._state['recorded'], {'a.md': ['dirty-pipe', 'the-fix', 'the-fix_1']})

    def test_korean_headings_get_english_ids(self):
        html = render('# 더티 파이프\n\n## 수정\n', 'a.md', record=False, ids={'a.md': ['dirty-pipe', 'the-fix']})
        self.assertIn('<h1 id="dirty-pipe">', html)
        self.assertIn('<h2 id="the-fix">', html)

    def test_heading_count_mismatch_fails(self):
        with self.assertRaises(ValueError):
            render('# 하나\n', 'a.md', record=False, ids={'a.md': ['one', 'two']})

    def test_page_missing_from_the_english_build_fails(self):
        with self.assertRaises(ValueError):
            render('# 하나\n', 'b.md', record=False, ids={'a.md': ['one']})


if __name__ == '__main__':
    unittest.main()
