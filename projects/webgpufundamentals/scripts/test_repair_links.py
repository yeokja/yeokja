import unittest
from repair_links import repair_lesson_html

class LinkRepairTests(unittest.TestCase):
    def test_corrects_verified_typos_and_empty_anchors(self):
        text = '<a href="webgpu-lighitng-spot.html">조명</a><a href="../a-blending"></a><a href="../../webgpu-compute-shaders.html#a-race-conditions">경쟁</a>'
        result = repair_lesson_html(text, 'webgpu-compute-shaders-histogram.html', 'ko')
        self.assertIn('href="webgpu-lighting-spot.html"', result)
        self.assertIn('id="a-blending"', result)
        self.assertIn('href="webgpu-compute-shaders.html#a-race-conditions"', result)

    def test_marks_absent_upstream_articles_without_fabricating_pages(self):
        result = repair_lesson_html('<a href="webgpu-skinning.html">스키닝</a>', 'webgpu-picking.html', 'ko')
        self.assertIn('스키닝', result)
        self.assertIn('원문 준비 중', result)
        self.assertNotIn('<a ', result)
        self.assertIn('data-upstream-href="webgpu-skinning.html"', result)

    def test_does_not_rewrite_external_links(self):
        text = '<a href="https://example.com/webgpu-skinning.html">external</a>'
        self.assertEqual(text, repair_lesson_html(text, 'webgpu-picking.html', 'ko'))

if __name__ == '__main__':
    unittest.main()
