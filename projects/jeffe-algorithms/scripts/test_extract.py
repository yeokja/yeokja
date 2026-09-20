import unittest
from extract import classify, normalize_glyphs, chapter_ranges

PAGE_HEIGHT = 691.9  # the book's page is 481.68 x 691.92 pt


class ClassifyTests(unittest.TestCase):
    def test_fonts(self):
        box = (100, 300, 200, 310)
        self.assertEqual(classify('ABCDEF+XCharter-Roman', box, PAGE_HEIGHT, []), 'text')
        self.assertEqual(classify('ABCDEF+MathDesign-CH-Regular-Sy', box, PAGE_HEIGHT, []), 'math')
        self.assertEqual(classify('ABCDEF+Inconsolatazi4-Regular', box, PAGE_HEIGHT, []), 'code')
        self.assertEqual(classify('ABCDEF+Roboto-Bold', box, PAGE_HEIGHT, []), 'heading')

    def test_math_alphabet_and_symbol_fonts(self):
        # MathDesign draws math letters and digits with CharterBT; directed-edge
        # arrows come from stmaryrd; difficulty marks (hearts etc.) from Dingbats.
        box = (100, 300, 200, 310)
        for font in ('CharterBT-Italic', 'CharterBT-Roman', 'stmary10', 'Dingbats', 'XCharter-Math'):
            self.assertEqual(classify(font, box, PAGE_HEIGHT, []), 'math', font)

    def test_position_and_figure_win_over_font(self):
        self.assertEqual(classify('Roboto-Light', (22.6, 31.9, 92.0, 40.8), PAGE_HEIGHT, []), 'header_footer')
        self.assertEqual(classify('Roboto-Light', (450.2, 656.6, 459.2, 665.6), PAGE_HEIGHT, []), 'header_footer')
        self.assertEqual(classify('XCharter-Roman', (110, 310, 150, 320), PAGE_HEIGHT, [(100, 300, 300, 400)]), 'figure')

    def test_chapter_epigraph_near_top_is_not_a_running_head(self):
        self.assertEqual(classify('Roboto-LightItalic', (72.0, 49.7, 356.5, 57.8), PAGE_HEIGHT, []), 'heading')


class GlyphTests(unittest.TestCase):
    def test_known_glyph_problems(self):
        self.assertEqual(normalize_glyphs('u\x01v', 'stmary10'), 'u⟨ARC⟩v')
        self.assertEqual(normalize_glyphs('ﬁnd ﬂow'), 'find flow')

    def test_arc_marker_only_for_stmaryrd(self):
        # MathDesign's extension font uses \x01 for a large delimiter.
        self.assertEqual(normalize_glyphs('\x01', 'MathDesign-CH-Regular-Ex'), '\x01')

    def test_difficulty_marks(self):
        self.assertEqual(normalize_glyphs('n', 'Dingbats'), '⟨DIFF:♥⟩')
        self.assertEqual(normalize_glyphs('po', 'Dingbats'), '⟨DIFF:♣⟩⟨DIFF:♦⟩')
        self.assertEqual(normalize_glyphs('m', 'Dingbats'), '⟨DIFF:♠⟩')


class ChapterTests(unittest.TestCase):
    def test_ranges_from_outline(self):
        # The outline has bare titles; chapters are the entries between the
        # table of contents and the first index, numbered from 0.
        toc = [[1, 'Preface', 5], [2, 'About This Book', 5], [1, 'Table of Contents', 13],
               [1, 'Introduction', 19], [2, 'What is an algorithm?', 19],
               [1, 'Recursion', 39], [1, 'Index', 447], [1, 'Index of People', 461],
               [1, 'Image Credits', 469], [1, 'Colophon', 471]]
        self.assertEqual(chapter_ranges(toc, 472), {
            'preface': [5, 12], 'table-of-contents': [13, 18], '00': [19, 38], '01': [39, 446],
            'index': [447, 460], 'index-of-people': [461, 468], 'image-credits': [469, 470],
            'colophon': [471, 472]})


if __name__ == '__main__':
    unittest.main()
