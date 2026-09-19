from pathlib import Path
import tempfile
import unittest
from finish_html import add_footer, finish


class FinishTests(unittest.TestCase):
    def test_footer_goes_inside_main_with_root_relative_license_links(self):
        result = add_footer('<main><h1>제목</h1></main>', depth=2)
        self.assertIn('<footer class="translation-credit">', result)
        self.assertLess(result.index('<footer'), result.index('</main>'))
        self.assertIn('href="../../LICENSE"', result)
        self.assertIn('href="../../NOTICE"', result)
        self.assertIn('https://github.com/yeokja/yeokja', result)
        self.assertIn('Anthropic 사의 <code>claude-sonnet-5</code> 모델을 활용하여', result)
        self.assertIn('학습을 모두 비허용한 상태로 작업하였습니다', result)
        self.assertIn('비공식 번역', result)

    def test_page_without_main_is_unchanged(self):
        self.assertEqual(add_footer('<html>redirect</html>', depth=0), '<html>redirect</html>')

    def test_finish_uses_each_page_depth(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'a').mkdir()
            (root / 'index.html').write_text('<main></main>')
            (root / 'a' / 'b.html').write_text('<main></main>')
            finish(root)
            self.assertIn('href="LICENSE"', (root / 'index.html').read_text())
            self.assertIn('href="../LICENSE"', (root / 'a' / 'b.html').read_text())


if __name__ == '__main__':
    unittest.main()
