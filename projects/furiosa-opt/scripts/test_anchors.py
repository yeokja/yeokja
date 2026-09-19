import unittest
from preserve_anchors import preserve_anchors


class AnchorTests(unittest.TestCase):
    def test_original_and_korean_fragments_both_resolve(self):
        original = '<main><h1 id="hir-debugging">HIR Debugging</h1><h2 id="details">Details</h2></main>'
        translated = '<main><h1 id="hir-디버깅"><a href="#hir-디버깅">HIR 디버깅</a></h1><h2 id="세부">세부</h2></main>'
        result = preserve_anchors(original, translated)
        self.assertIn('id="hir-debugging"', result)
        self.assertIn('id="hir-디버깅"', result)
        self.assertIn('href="#hir-디버깅"', result)
        self.assertIn('id="details"', result)
        self.assertEqual(preserve_anchors(original, result), result)

    def test_heading_changes_fail_instead_of_misplacing_anchors(self):
        with self.assertRaises(ValueError):
            preserve_anchors('<main><h1 id="a">A</h1></main>', '<main><h2 id="가">가</h2></main>')

    def test_code_and_navigation_are_not_headings(self):
        source = '<h1 id="menu">Menu</h1><main><pre>&lt;h1&gt;</pre><h1 id="rust">Rust</h1></main>'
        self.assertEqual(preserve_anchors(source, source), source)



if __name__ == '__main__':
    unittest.main()
