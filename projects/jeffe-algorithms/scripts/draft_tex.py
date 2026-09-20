"""Write a mechanical LaTeX draft of PDF pages straight from the text layer.

    python3 scripts/draft_tex.py 39 88 /tmp/draft.tex

Prose goes from build/extract/pNNN.json into the draft without being retyped,
so a reconstruction agent only has to fix structure, identifiers and math
around it. Lines the script cannot place are kept as comments for the agent
to resolve:

    %SF: ...        Roboto text (epigraphs, captions, pseudocode comments)
    %CODE: ...      Inconsolata lines (code, strings)
    %FOOTNOTE: ...  small text at the page bottom (footnote bodies)
    %PAGE N         page boundary

Inline marks become \\emph{..}, \\textbf{..}, $..$ (raw extracted math, to be
rewritten as proper LaTeX) and \\Str{..}.
"""
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from pageview import EXTRACT, lines_of  # noqa: E402

import json

LINE = re.compile(r'^\s*(?P<y>\d+) x\s*(?P<x>[\d.]+) s\s*(?P<s>[\d.]+) \| (?P<t>.*)$')


def merge_math(text):
    """Join math spans the extractor split: ⟦d⟧⟦[⟧⟦v⟧ is one formula."""
    previous = None
    while text != previous:
        previous = text
        text = re.sub(r'⟧( ?)⟦', lambda m: m.group(1), text)
    return text


def inline(text):
    text = merge_math(text)
    text = re.sub(r'_(?:\s*_)+', ' ', text)            # `_running_ _time_` -> one italic run
    text = re.sub(r'\*_(.+?)_\*', r'\\textbf{\\emph{\1}}', text)
    text = re.sub(r'(?<![\w\\])_(.+?)_(?!\w)', r'\\emph{\1}', text)
    text = re.sub(r'(?<![\w\\])\*(.+?)\*(?!\w)', r'\\textbf{\1}', text)
    text = re.sub(r'⟦(.+?)⟧', r'$\1$', text)
    text = re.sub(r'`(.+?)`', r'\\Str{\1}', text)
    return text


def only(text, kind):
    stripped = re.sub(r'\s+', '', text)
    if kind == 'sf':
        return bool(stripped) and re.fullmatch(r'(\{sf[BI]*:[^}]*\})+', stripped) is not None
    return bool(stripped) and re.fullmatch(r'(`[^`]*`)+', stripped) is not None


def unsf(text):
    return re.sub(r'\{sf[BI]*:([^}]*)\}', r'\1', text)


def draft(first, last):
    out, para, prev_y, left = [], [], None, None
    page_left, prev_indent = None, 0
    # A paragraph often runs across a page break and past the footnotes at
    # the page bottom, so page markers and footnote bodies wait until the
    # paragraph they interrupt has been written.
    pending = []

    def flush():
        if para:
            text = ' '.join(para)
            text = re.sub(r'(\w)- (?=[a-z])', r'\1', text)  # join words hyphenated at line ends
            out.append(inline(text))
            out.append('')
            para.clear()
        out.extend(pending)
        pending.clear()

    for number in range(first, last + 1):
        pending.append(f'%PAGE {number}')
        page = json.loads((EXTRACT / f'p{number:03d}.json').read_text())
        parsed = [m.groupdict() for m in (LINE.match(raw) for raw in lines_of(page)) if m]
        body = [float(d['x']) for d in parsed if float(d['s']) >= 9 and not d['t'].startswith('{sf')]
        prev_y, page_left, prev_indent = None, min(body, default=0.0), 0
        for d in parsed:
            y, x, size, text = float(d['y']), float(d['x']), float(d['s']), d['t']
            if left is None or x < left:
                left = x
            if size >= 12 and text.startswith('{sfB:'):
                flush()
                title = unsf(text).strip()
                number_match = re.match(r'([\d.]+)\s+(.*)', title)
                if size >= 20:
                    out += [f'\\chapter{{{title}}}', '']
                elif number_match:
                    num, name = number_match.groups()
                    level = 'section' if num.count('.') == 1 else 'subsection'
                    out += [f'\\{level}{{{name}}}', f'\\label{{sec:{num}}}', '']
                else:
                    out += [f'\\subsection*{{{title}}}', '']
            elif size < 9 and y > 500:
                pending.append('%FOOTNOTE: ' + inline(text))
                continue
            elif only(text, 'sf'):
                flush()
                out.append('%SF: ' + unsf(text))
            elif only(text, 'code'):
                flush()
                out.append('%CODE: ' + text.replace('`', ''))
            else:
                # Indentation tells paragraphs and list levels apart. The body
                # block starts at `left` (which shifts by ~18pt between verso
                # and recto pages, so compare within a page); a first line is
                # indented about one em; a list item's body sits further in and
                # its continuation lines line up with it, not with `left`.
                indent = x - page_left
                gap = prev_y is not None and y - prev_y > 20
                # Sub- and superscripts of inline math come out as their own
                # short lines at odd offsets; they belong to the line before.
                fragment = len(re.sub(r'[⟦⟧{}`_*]', '', text).strip()) < 12
                starts_paragraph = not fragment and (
                    gap or 7 < indent < 14 or (indent >= 14 and abs(indent - prev_indent) > 2))
                if starts_paragraph:
                    flush()
                    if indent >= 14:
                        out.append(f'%INDENT {indent:.0f}')
                if fragment:
                    prev_y = y
                    para.append(text)
                    continue
                para.append(text)
                prev_indent = indent
            prev_y = y
    flush()
    return '\n'.join(out) + '\n'


if __name__ == '__main__':
    first, last, target = int(sys.argv[1]), int(sys.argv[2]), Path(sys.argv[3])
    target.write_text(draft(first, last))
