"""Print extracted pages as marked-up lines for reconstruction.

    python3 scripts/pageview.py 89 90 91

Groups each page's spans (build/extract/pNNN.json) into lines and marks the
font class so words can be copied exactly while the page image gives the
structure: ⟦math⟧  _italic_  *bold*  *_bold italic_*  {sf: Roboto}  `code`.
Each line starts with its y, x and size, so indentation (new paragraph,
list item, pseudocode level) and superscript footnote marks are visible.
Figure-internal spans and running heads are only counted.
"""
import json
from pathlib import Path
import sys

EXTRACT = Path(__file__).resolve().parent.parent / 'build/extract'


def mark(span):
    text, font = span['text'], span['font'].split('+')[-1]
    if span['cls'] == 'math':
        return '⟦' + text + '⟧'
    if span['cls'] == 'code':
        return '`' + text + '`'
    if span['cls'] == 'heading':
        style = ('sfB' if 'Bold' in font else 'sf') + ('I' if 'Italic' in font else '')
        return '{' + style + ':' + text + '}'
    if 'BoldItalic' in font:
        return '*_' + text + '_*'
    if 'Bold' in font:
        return '*' + text + '*'
    if 'Italic' in font or 'Slanted' in font:
        return '_' + text + '_'
    return text


def lines_of(page):
    spans = [s for s in page['spans'] if s['cls'] not in ('figure', 'header_footer')]
    lines = []
    for span in spans:
        y = (span['bbox'][1] + span['bbox'][3]) / 2
        for line in lines:
            if abs(line['y'] - y) < 3.5 and not (span['size'] < 8 and line['size'] >= 8):
                line['spans'].append(span)
                break
        else:
            lines.append({'y': y, 'size': span['size'], 'spans': [span]})
    for line in sorted(lines, key=lambda l: l['y']):
        line['spans'].sort(key=lambda s: s['bbox'][0])
        parts, prev = [], None
        for span in line['spans']:
            if prev is not None and span['bbox'][0] - prev > 1.2:
                parts.append(' ')
            parts.append(mark(span))
            prev = span['bbox'][2]
        size = max(s['size'] for s in line['spans'])
        yield f'{line["y"]:5.0f} x{line["spans"][0]["bbox"][0]:5.1f} s{size:4.1f} | ' + ''.join(parts)


def main(numbers):
    for n in numbers:
        page = json.loads((EXTRACT / f'p{n:03d}.json').read_text())
        print(f'=================== PDF page {n} (printed {page["printed"]}) figure boxes={len(page["figures"])}')
        for line in lines_of(page):
            print(line)
        hidden = sum(1 for s in page['spans'] if s['cls'] == 'figure')
        if hidden:
            print(f'   [{hidden} figure-internal spans omitted]')
        links = sorted({link['name'] for link in page['links'] if link['name']})
        if links:
            print('   links:', ', '.join(links))


if __name__ == '__main__':
    main([int(a) for a in sys.argv[1:]])
