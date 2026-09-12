import unittest
from preserve_anchors import preserve_anchors

class AnchorTests(unittest.TestCase):
    def test_keeps_english_and_korean_fragments(self):
        source = '<div class="lesson-main"><h2 id="multiple-views">Multiple views</h2><div><h3 id="details">Details</h3></div></div>'
        target = '<div class="lesson-main"><h2 id="여러-뷰">여러 뷰</h2><div><h3 id="세부">세부</h3></div></div>'
        result = preserve_anchors(source, target)
        for anchor in ('multiple-views', 'details', '여러-뷰', '세부'):
            self.assertIn(f'id="{anchor}"', result)
        self.assertEqual(result, preserve_anchors(source, result))

    def test_rejects_mismatched_heading_structure(self):
        with self.assertRaises(ValueError):
            preserve_anchors('<div class="lesson-main"><h2 id="a">A</h2></div>', '<div class="lesson-main"><h3 id="가">가</h3></div>')

    def test_ignores_sidebar_headings(self):
        source = '<div class="lesson-main"><h2 id="a">A</h2></div><h2 id="nav">Nav</h2>'
        target = '<div class="lesson-main"><h2 id="가">가</h2></div><h3 id="nav">탐색</h3>'
        self.assertIn('id="a"', preserve_anchors(source, target))

if __name__ == '__main__':
    unittest.main()
