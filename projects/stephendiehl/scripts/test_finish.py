from pathlib import Path
import tempfile
import unittest
from finish_html import add_footer, finish


class FinishTests(unittest.TestCase):
    def test_footer_carries_the_required_attribution(self):
        result = add_footer('<main><h1>제목</h1></main>', depth=1)
        self.assertIn('<footer class="translation-credit">', result)
        self.assertLess(result.index('<footer'), result.index('</main>'))
        self.assertIn('Stephen Diehl, <a href="https://www.stephendiehl.com">stephendiehl.com</a>', result)
        self.assertIn('https://creativecommons.org/licenses/by-sa/4.0/', result)
        self.assertIn('href="../license.html"', result)
        self.assertIn('런던의 소프트웨어 엔지니어이자 작가', result)
        self.assertIn('어떤 조직과의 연관도', result)
        self.assertIn('https://github.com/yeokja/yeokja', result)
        self.assertIn('Anthropic 사의 <code>claude-opus-5-5</code> 모델을 활용하여', result)
        self.assertIn('학습을 모두 비허용한 상태로 작업하였습니다', result)
        self.assertIn('비공식 번역', result)

    def test_page_without_main_is_unchanged(self):
        self.assertEqual(add_footer('<html>redirect</html>', depth=0), '<html>redirect</html>')

    def test_finish_uses_each_page_depth(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'posts').mkdir()
            (root / 'index.html').write_text('<main></main>')
            (root / 'posts' / 'b.html').write_text('<main></main>')
            finish(root)
            self.assertIn('href="license.html"', (root / 'index.html').read_text())
            self.assertIn('href="../license.html"', (root / 'posts' / 'b.html').read_text())


if __name__ == '__main__':
    unittest.main()
