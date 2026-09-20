"""Render original and rebuilt chapter pages side by side for visual review.

    python3 scripts/sidebyside.py 02          # after scripts/verify_source.py 02

Writes build/sbs/<key>-NN.png: the original PDF page on the left, page NN of
build/verify/<key>/main.pdf on the right. The rebuilt chapter repaginates, so
a pair shows roughly the same content; use it to check math, pseudocode,
displays and layout, not line breaks.
"""
import json
from pathlib import Path
import sys

import pymupdf

ROOT = Path(__file__).resolve().parent.parent


def main(key, dpi=80):
    first, last = json.loads((ROOT / 'build/extract/chapters.json').read_text())[key]
    orig = pymupdf.open(ROOT / 'upstream/Algorithms-JeffE.pdf')
    new = pymupdf.open(ROOT / 'build/verify' / key / 'main.pdf')
    out_dir = ROOT / 'build/sbs'
    out_dir.mkdir(parents=True, exist_ok=True)
    width, height = orig[0].rect.width, orig[0].rect.height
    for k in range(max(last - first + 1, new.page_count)):
        sheet = pymupdf.open()
        page = sheet.new_page(width=2 * width + 10, height=height)
        if first + k <= last:
            page.show_pdf_page(pymupdf.Rect(0, 0, width, height), orig, first + k - 1)
        if k < new.page_count:
            page.show_pdf_page(pymupdf.Rect(width + 10, 0, 2 * width + 10, height), new, k)
        page.draw_line((width + 5, 0), (width + 5, height), color=(1, 0, 0))
        path = out_dir / f'{key}-{k + 1:02d}.png'
        page.get_pixmap(dpi=dpi).save(path)
        print(path)


if __name__ == '__main__':
    main(sys.argv[1], *(int(a) for a in sys.argv[2:3]))
