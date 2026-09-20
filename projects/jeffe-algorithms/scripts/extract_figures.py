"""Cut every embedded vector figure (form XObject) and raster image out of
the Algorithms PDF as standalone files for the reconstructed source.

The book's figures are OmniGraffle PDFs placed with \\includegraphics, so each
is a form XObject drawn by a `Do` operator; its page box is the XObject's
BBox transformed by the current transformation matrix at that `Do`.

Each vector figure is written as a one-page PDF whose only content is that
form XObject (not a clipped copy of the whole page), so the file carries no
hidden page text and only the fonts the figure itself uses.
"""
import json
from pathlib import Path
import re
import sys

import pymupdf

TOKEN = re.compile(rb"/[^\s/\[\]()<>{}]+|\((?:\\.|[^\\)])*\)|<[0-9A-Fa-f\s]*>|\[|\]|[^\s/\[\]()<>]+")
SKIP_PAGES = {79}  # Figure 1.25 (printed p. 61): portrait included with the artist's permission only


def _array(doc, xref, key, default):
    kind, value = doc.xref_get_key(xref, key)
    return [float(v) for v in value.strip('[]').split()] if kind == 'array' else default


def _draws(doc, page):
    """(name, xref, ctm, pdf_rect) for every form XObject drawn on the page.

    ctm is the transformation in effect at the `Do` (the renderer applies the
    form's own /Matrix on top of it); pdf_rect is the form's BBox mapped by
    /Matrix and then ctm, in PDF (bottom-left origin) coordinates.
    PyMuPDF's get_xobjects() bbox already includes /Matrix, so the raw keys
    are read here instead.
    """
    forms = {}
    for xref, name, invoker, _ in page.get_xobjects():
        if invoker == 0:
            matrix = pymupdf.Matrix(*_array(doc, xref, 'Matrix', [1, 0, 0, 1, 0, 0]))
            forms[name] = (xref, pymupdf.Rect(_array(doc, xref, 'BBox', [0, 0, 0, 0])) * matrix)
    stream = b''.join(doc.xref_stream(c) or b'' for c in page.get_contents())
    ctm, stack, operands, found = pymupdf.Matrix(1, 0, 0, 1, 0, 0), [], [], []
    for token in TOKEN.findall(stream):
        if token == b'q':
            stack.append(ctm)
        elif token == b'Q':
            ctm = stack.pop() if stack else ctm
        elif token == b'cm':
            a, b, c, d, e, f = map(float, operands[-6:])
            ctm = pymupdf.Matrix(a, b, c, d, e, f) * ctm
        elif token == b'Do':
            name = operands[-1].decode()[1:]
            if name in forms:
                xref, box = forms[name]
                found.append((name, xref, ctm, box * ctm))
        if token in (b'q', b'Q', b'cm', b'Do') or (re.match(rb"^[A-Za-z'\"*]+$", token) and not token.startswith(b'/')):
            operands = []
        else:
            operands.append(token)
    return found


def placements(doc, page):
    """(XObject name, box in page coordinates with top-left origin)."""
    height = page.mediabox.height
    return [(name, pymupdf.Rect(r.x0, height - r.y1, r.x1, height - r.y0))
            for name, _, _, r in _draws(doc, page)]


def page_images(page):
    """(xref, smask xref, box) for raster images drawn directly on the page.

    Images inside form XObjects are part of a vector cut already. Boxes come
    from get_image_info(); get_image_bbox() crashes in PyMuPDF 1.27 builds
    on Python 3.14 (SWIG director error).
    """
    direct = {image[0]: image[1] for image in page.get_images(full=True) if image[-1] == 0}
    found = []
    for info in page.get_image_info(xrefs=True):
        rect = pymupdf.Rect(info['bbox'])
        if info['xref'] in direct and not rect.is_empty and rect.is_valid:
            found.append((info['xref'], direct[info['xref']], rect))
    return found


def cut_form(doc, index, name, ctm, pdf_rect, path):
    """Write a PDF whose single page draws only form XObject `name`."""
    out = pymupdf.open()
    out.insert_pdf(doc, from_page=index, to_page=index, links=False, annots=False)
    page = out[0]
    xref = next(x for x, n, invoker, _ in page.get_xobjects() if n == name and invoker == 0)
    m = pymupdf.Matrix(ctm) * pymupdf.Matrix(1, 0, 0, 1, -pdf_rect.x0, -pdf_rect.y0)
    contents = page.get_contents()
    out.update_stream(contents[0], f'q {m.a:g} {m.b:g} {m.c:g} {m.d:g} {m.e:g} {m.f:g} cm /{name} Do Q'.encode())
    out.xref_set_key(page.xref, 'Contents', f'{contents[0]} 0 R')
    out.xref_set_key(page.xref, 'Resources', f'<</XObject<</{name} {xref} 0 R>>>>')
    out.xref_set_key(page.xref, 'MediaBox', f'[0 0 {pdf_rect.width:g} {pdf_rect.height:g}]')
    for key in ('CropBox', 'TrimBox', 'BleedBox', 'ArtBox', 'Annots'):
        out.xref_set_key(page.xref, key, 'null')
    out.save(path, garbage=4, deflate=True)


def main(pdf_path, figures_dir, boxes_path):
    doc = pymupdf.open(pdf_path)
    figures_dir, manifest, boxes = Path(figures_dir), {}, {}
    figures_dir.mkdir(parents=True, exist_ok=True)
    for index, page in enumerate(doc):
        number = index + 1
        height = page.mediabox.height
        page_boxes = []
        for name, _, ctm, r in _draws(doc, page):
            box = [round(v, 1) for v in (r.x0, height - r.y1, r.x1, height - r.y0)]
            page_boxes.append(box)
            if number in SKIP_PAGES:
                continue
            key = f'p{number:03d}-{name}'
            cut_form(doc, index, name, ctm, r, figures_dir / f'{key}.pdf')
            manifest[key] = {'page': number, 'bbox': box, 'kind': 'vector'}
        for n, (xref, smask, rect) in enumerate(page_images(page)):
            box = [round(v, 1) for v in rect]
            page_boxes.append(box)
            if number in SKIP_PAGES:
                continue
            key = f'p{number:03d}-img{n}'
            if smask:
                pix = pymupdf.Pixmap(pymupdf.Pixmap(doc, xref), pymupdf.Pixmap(doc, smask))
                pix.save(figures_dir / f'{key}.png')
            else:
                info = doc.extract_image(xref)
                (figures_dir / f'{key}.{info["ext"]}').write_bytes(info['image'])
            manifest[key] = {'page': number, 'bbox': box, 'kind': 'raster'}
        if page_boxes:
            boxes[str(number)] = page_boxes
    (figures_dir / 'manifest.json').write_text(json.dumps(manifest, indent=1))
    Path(boxes_path).parent.mkdir(parents=True, exist_ok=True)
    Path(boxes_path).write_text(json.dumps(boxes, indent=1))


if __name__ == '__main__':
    main(*sys.argv[1:4])
