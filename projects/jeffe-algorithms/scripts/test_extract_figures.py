from pathlib import Path
import tempfile
import unittest
import pymupdf
from extract_figures import _draws, cut_form, placements


def page_with_figure():
    src = pymupdf.open()
    figure = src.new_page(width=100, height=50)
    figure.draw_rect(pymupdf.Rect(0, 0, 100, 50))
    figure.insert_text((10, 30), 'label')
    doc = pymupdf.open()
    page = doc.new_page(width=612, height=792)
    page.insert_text((50, 100), 'surrounding prose')
    page.show_pdf_page(pymupdf.Rect(200, 300, 300, 350), src, 0)
    return doc, page


class PlacementTests(unittest.TestCase):
    def test_form_xobject_placement_in_page_coordinates(self):
        doc, page = page_with_figure()
        found = placements(doc, page)
        self.assertEqual(len(found), 1)
        name, rect = found[0]
        self.assertAlmostEqual(rect.x0, 200, delta=1)
        self.assertAlmostEqual(rect.y0, 300, delta=1)
        self.assertAlmostEqual(rect.x1, 300, delta=1)
        self.assertAlmostEqual(rect.y1, 350, delta=1)

    def test_cut_contains_only_the_figure(self):
        doc, page = page_with_figure()
        (name, _, ctm, pdf_rect), = _draws(doc, page)
        with tempfile.TemporaryDirectory() as d:
            path = Path(d) / 'fig.pdf'
            cut_form(doc, 0, name, ctm, pdf_rect, path)
            cut = pymupdf.open(path)
            self.assertEqual(len(cut), 1)
            self.assertAlmostEqual(cut[0].rect.width, 100, delta=1)
            self.assertAlmostEqual(cut[0].rect.height, 50, delta=1)
            words = [w[4] for w in cut[0].get_text('words')]
            self.assertEqual(words, ['label'])
            self.assertLess(cut[0].get_text('words')[0][0], 20)  # drawn at the figure's own origin


if __name__ == '__main__':
    unittest.main()
