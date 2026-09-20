"""Extract per-page reconstruction material from the Algorithms PDF.

For every PDF page: a 150dpi PNG, and a JSON with each text span's font,
size, bbox and class (text/math/code/heading/figure/header_footer), the
page's internal link targets, and the bounding boxes of embedded figures.
Also writes chapters.json (chapter key -> [first, last] PDF page) from the
outline. Figure boxes come from figures.json, written by extract_figures.py.
"""
import json
from pathlib import Path
import re
import sys

MARGIN = 45  # points; running heads and folios live in the top/bottom bands
LIGATURES = {'ﬁ': 'fi', 'ﬂ': 'fl', 'ﬀ': 'ff', 'ﬃ': 'ffi', 'ﬄ': 'ffl'}
# The book's Dingbats font draws the exercise annotations of the Preface's
# "About the Exercises": red heart, blue diamond, green club, black spade.
DIFFICULTY = {'n': '♥', 'o': '♦', 'p': '♣', 'm': '♠'}
MATH_FONTS = ('MathDesign', 'CharterBT', 'stmary', 'Dingbats', 'CMSY', 'CMEX', 'MSAM', 'MSBM')


def classify(font, bbox, page_height, figure_boxes):
    x0, y0, x1, y1 = bbox
    if y1 < MARGIN or y0 > page_height - MARGIN:
        return 'header_footer'
    cx, cy = (x0 + x1) / 2, (y0 + y1) / 2
    if any(fx0 <= cx <= fx1 and fy0 <= cy <= fy1 for fx0, fy0, fx1, fy1 in figure_boxes):
        return 'figure'
    name = font.split('+')[-1]
    if name.startswith(MATH_FONTS) or 'Math' in name:
        return 'math'
    if name.startswith('Inconsolata'):
        return 'code'
    if name.startswith('Roboto'):
        return 'heading'
    return 'text'


def normalize_glyphs(text, font=''):
    name = font.split('+')[-1]
    if name.startswith('stmary'):
        text = text.replace('\x01', '⟨ARC⟩')
    if name.startswith('Dingbats'):
        text = ''.join(f'⟨DIFF:{DIFFICULTY[c]}⟩' if c in DIFFICULTY else c for c in text)
    for bad, good in LIGATURES.items():
        text = text.replace(bad, good)
    return text


def chapter_ranges(toc, page_count):
    """Top-level outline entries -> keys ('preface', '00'..'12', 'index', ...).

    The outline titles carry no numbers, so the chapters are the entries
    between 'Table of Contents' and the first index, numbered from 0.
    """
    tops = [(title, page) for level, title, page in toc if level == 1]
    ranges, number, in_chapters = {}, 0, False
    for i, (title, first) in enumerate(tops):
        last = (tops[i + 1][1] - 1) if i + 1 < len(tops) else page_count
        if title.startswith('Index'):
            in_chapters = False
        if in_chapters:
            key, number = f'{number:02d}', number + 1
        else:
            key = re.sub(r'\W+', '-', title.lower()).strip('-')
        if title == 'Table of Contents':
            in_chapters = True
        ranges[key] = [first, last]
    return ranges


def raster_boxes(page):
    """Raster images drawn on the page; vector figure boxes come from
    figures.json (extract_figures.py traces the content stream).
    get_image_info() instead of get_image_bbox(), which crashes in PyMuPDF
    1.27 builds on Python 3.14."""
    return [tuple(info['bbox']) for info in page.get_image_info()
            if info['bbox'][2] > info['bbox'][0] and info['bbox'][3] > info['bbox'][1]]


def main(pdf_path, out, figures_path=None):
    import pymupdf
    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    doc = pymupdf.open(pdf_path)
    (out / 'chapters.json').write_text(json.dumps(chapter_ranges(doc.get_toc(), doc.page_count), indent=1))
    figures = json.loads(Path(figures_path).read_text()) if figures_path else {}
    for index, page in enumerate(doc):
        number = index + 1
        boxes = [tuple(b) for b in figures.get(str(number), [])]
        boxes += [b for b in raster_boxes(page) if list(b) not in [list(x) for x in boxes]]
        spans = []
        for block in page.get_text('dict')['blocks']:
            for line in block.get('lines', []):
                for span in line['spans']:
                    if not span['text'].strip():
                        continue
                    spans.append({'text': normalize_glyphs(span['text'], span['font']), 'font': span['font'],
                                  'size': round(span['size'], 1), 'bbox': [round(v, 1) for v in span['bbox']],
                                  'cls': classify(span['font'], span['bbox'], page.rect.height, boxes)})
        links = [{'bbox': [round(v, 1) for v in link['from']], 'to_page': link.get('page', -1) + 1,
                  'name': link.get('nameddest') or link.get('name') or link.get('uri')}
                 for link in page.get_links()]
        (out / f'p{number:03d}.json').write_text(json.dumps(
            {'page': number, 'printed': page.get_label(), 'spans': spans, 'links': links,
             'figures': [[round(v, 1) for v in b] for b in boxes]}, ensure_ascii=False, indent=0))
        page.get_pixmap(dpi=150).save(out / f'p{number:03d}.png')


if __name__ == '__main__':
    main(*sys.argv[1:4])
