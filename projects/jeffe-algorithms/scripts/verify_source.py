"""Verify a reconstructed chapter against the original PDF's text layer.

Compiles the English source for one chapter, extracts its text with the same
span classification as the original, and compares prose word sequences.
Blocks that only moved (floats and footnotes land on other pages when the
rebuilt page breaks differ) count as matched but are listed for review.
Structure (figure captions, footnotes, exercises, outline, pseudocode
procedures) is compared between the two PDFs. The math symbol stream is
compared for review only.

Writes build/verify/<key>.txt and exits 1 when the chapter fails.
"""
from collections import Counter
from dataclasses import dataclass, field
import difflib
import json
from pathlib import Path
import re
import subprocess
import sys
import unicodedata

from extract import classify, normalize_glyphs

ROOT = Path(__file__).resolve().parent.parent
KEEP = {'text', 'heading', 'code'}
MAX_RATIO = 0.005
MIN_MOVE = 4  # tokens; shorter coincidental runs are not treated as moved blocks
NUMBER = re.compile(r'^[\dA-Z]+(\.\d+)*[.,;:)]*$')
TOKEN = re.compile(r"\?\?|\d+(?:\.\d+)+|\w+(?:'\w+)*|[^\w\s]")
QUOTES = str.maketrans({'’': "'", '‘': "'", '“': '"', '”': '"'})
PAGE_REF = re.compile(r'\b(pages?)\s+(?:\d+|[xivlc]+\b)(?:\s*[–-]\s*(?:\d+|[xivlc]+))?\b')
MATH_MAP = str.maketrans({'〈': '⟨', '〉': '⟩', '〈': '⟨', '〉': '⟩',
                          '−': '-', '∶': ':', '\\': '∖', '⋅': '·', '⋯': '···'})
# MathDesign's extension font (large operators, big delimiters): the text
# layer shows ASCII codes; keep the operators, drop the delimiters (big
# delimiters are dropped from the rebuilt side too).
EX_GLYPHS = {'X': '∑', 'P': '∑', 'W': '⋁', 'V': '⋀', 'S': '⋃', 'T': '⋂',
             'p': '√', 'q': '√', 'r': '√', 's': '√'}
BIG_MATH_SIZE = 12.5  # rebuilt math spans larger than this are big operators/delimiters
FILES = {'preface': 'frontmatter/preface', 'image-credits': 'backmatter/credits', 'colophon': 'backmatter/colophon',
         **{f'{n:02d}': f'chapters/{n:02d}-{name}' for n, name in enumerate(
             ['intro', 'recursion', 'backtracking', 'dynprog', 'greedy', 'graphs', 'dfs', 'mst',
              'sssp', 'apsp', 'maxflow', 'maxflowapps', 'nphard'])}}


ACCENT_BEFORE = {'\u0304', '\u0307'}   # macron, dot above: pdfTeX's \accent emits them before the base
DOT_BELOW = '\u0323'                   # \d{x} is an \ooalign, so its dot extracts as an ordinary '.'


def refold_accents(text):
    r"""Write combining accents the way the original's text layer stores them.

    The original is pdfTeX with T1 XCharter, which has no precomposed ā/ṅ/ṛ,
    so it builds them with \accent (accent glyph first) and \d (a period
    after the base). LuaLaTeX emits the precomposed character instead.
    """
    out = []
    for ch in unicodedata.normalize('NFD', text):
        if not unicodedata.combining(ch):
            out.append(ch)
        elif ch == DOT_BELOW:
            out.append('.')
        elif ch in ACCENT_BEFORE and out:
            out.insert(len(out) - 1, ch)
        else:
            out.append(ch)
    return unicodedata.normalize('NFC', ''.join(out))


def words(spans):
    text = ' '.join(s['text'] for s in spans if s['cls'] in KEEP)
    text = unicodedata.normalize('NFKC', normalize_glyphs(text)).translate(QUOTES)
    text = re.sub(r'(\w)-\s+(\w)', r'\1\2', text)   # hyphenation at line ends
    text = re.sub(r'(?<=\w)-(?=\w)', '', text)       # so depth-first == depth-/first
    text = PAGE_REF.sub(r'\1 #', text)
    return [part for token in TOKEN.findall(text) for part in TOKEN.findall(refold_accents(token))]


def math_symbols(spans):
    parts = []
    for s in spans:
        if s['cls'] != 'math':
            continue
        text, font = s['text'], s.get('font', '').split('+')[-1]
        if font.startswith('MathDesign') and '-Ex' in font:
            text = ''.join(EX_GLYPHS.get(c, '') for c in text)
        elif s.get('size', 0) > BIG_MATH_SIZE:
            text = re.sub(r'[()\[\]{}|⟨⟩‖]', '', text)
        parts.append(text)
    text = re.sub(r'⟨DIFF:(.)⟩', r'\1', ''.join(parts).replace('⟨ARC⟩', '→'))
    text = text.replace('̸=', '≠').replace('≠', '≠').replace('=⇒', '⟹')
    text = unicodedata.normalize('NFKC', text).translate(MATH_MAP)
    return [c for c in text if not c.isspace() and c.isprintable()
            and not '' <= c <= '' and not '⎛' <= c <= '⎭']


def residual_difference(hunks):
    """Tokens missing from / added to the rebuilt text once order is ignored.

    Residual hunks whose tokens all reappear elsewhere are reorderings around
    footnotes and floats; what is left here needs a closer look.
    """
    a = Counter(t for h in hunks for t in h[0])
    b = Counter(t for h in hunks for t in h[1])
    return sorted((a - b).elements()), sorted((b - a).elements())


@dataclass
class Report:
    mismatch_ratio: float
    hunks: list = field(default_factory=list)   # [(original tokens, rebuilt tokens)]
    moved: list = field(default_factory=list)   # [(tokens, original index, rebuilt index)]


def compare(original, rebuilt, min_move=MIN_MOVE):
    matcher = difflib.SequenceMatcher(a=original, b=rebuilt, autojunk=False)
    ops = []
    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        if tag == 'equal':
            continue
        a, b = original[i1:i2], rebuilt[j1:j2]
        if tag == 'replace' and all(t == '??' for t in b) and all(NUMBER.match(t) for t in a):
            continue
        joined_a, joined_b = ''.join(a), ''.join(b)
        if tag == 'replace' and joined_a == joined_b:  # spacing only ("offthe" after an ff ligature)
            continue
        if tag == 'replace' and joined_a == joined_a.upper() != joined_a.lower() and joined_a == joined_b.upper():
            continue  # small caps the original extracts as capitals ("NONE" vs "None")
        ops.append((i1, i2, j1, j2))
    a_left = [i for i1, i2, _, _ in ops for i in range(i1, i2)]
    b_left = [j for _, _, j1, j2 in ops for j in range(j1, j2)]
    moved = []
    while a_left and b_left:
        mover = difflib.SequenceMatcher(a=[original[i] for i in a_left], b=[rebuilt[j] for j in b_left],
                                        autojunk=False)
        blocks = [blk for blk in mover.get_matching_blocks() if blk.size >= min_move]
        if not blocks:
            break
        taken_a, taken_b = set(), set()
        for x, y, n in blocks:
            moved.append((original[a_left[x]:a_left[x] + n], a_left[x], b_left[y]))
            taken_a.update(a_left[x:x + n])
            taken_b.update(b_left[y:y + n])
        a_left = [i for i in a_left if i not in taken_a]
        b_left = [j for j in b_left if j not in taken_b]
    a_set, b_set = set(a_left), set(b_left)
    hunks = []
    for i1, i2, j1, j2 in ops:
        a = [original[i] for i in range(i1, i2) if i in a_set]
        b = [rebuilt[j] for j in range(j1, j2) if j in b_set]
        if a or b:
            hunks.append((a, b))
    return Report(len(a_left) / max(len(original), 1), hunks, sorted(moved, key=lambda m: m[1]))


def figure_destinations(doc, key):
    """`figure.N.M` hyperref destinations of one chapter.

    More reliable than reading `Figure N.M.` off the caption: a caption that
    sits inside the figure's box is classified as figure text, and a stretched
    caption line splits the bold label across spans.
    """
    if not key.isdigit():
        return []
    prefix = f'{int(key)}.'
    return sorted(name.split('figure.')[1] for name in doc.resolve_names()
                  if name.startswith('figure.') and name.split('figure.')[1].startswith(prefix))


def footnote_names(links):
    seen = []
    for link in links:
        name = link.get('name') or ''
        if name.startswith('Hfootnote') and name not in seen:
            seen.append(name)
    return seen


def exercise_numbers(spans):
    numbers, started = [], False
    for s in spans:
        if s['cls'] == 'heading' and s['text'].strip() == 'Exercises':
            started = True
        elif started and s['cls'] == 'text' and s['bbox'][0] < 100:
            m = re.match(r'^\s*(\d+)\.(?!\d)', s['text'])
            if m:
                numbers.append(int(m.group(1)))
    return numbers


def pseudocode_index(lines):
    text = ' '.join(lines)
    return {name: [int(p) for p in re.findall(r'\d+', pages)]
            for name, pages in re.findall(r'([A-Za-z][^\s,]*),\s*(\d+(?:,\s*\d+)*)', text)}


def normalize_title(title):
    # The original's bookmarks carry ASCII quotes where the page prints
    # typographic ones, so fold them as the word comparison does.
    title = re.sub(r'⟨DIFF:.⟩|[♥♦♣♠]', '', unicodedata.normalize('NFKC', title)).translate(QUOTES)
    return ' '.join(title.split())


# ---- PDF side -------------------------------------------------------------

def load_original(key):
    chapters = json.loads((ROOT / 'build/extract/chapters.json').read_text())
    first, last = chapters[key]
    pages = [json.loads((ROOT / f'build/extract/p{p:03d}.json').read_text()) for p in range(first, last + 1)]
    import pymupdf
    toc = [(level, normalize_title(title)) for level, title, page in
           pymupdf.open(ROOT / 'upstream/Algorithms-JeffE.pdf').get_toc() if level >= 2 and first <= page <= last]
    lo, hi = chapters['index-of-pseudocode']
    index_spans = [s['text'] for p in range(lo, hi + 1)
                   for s in json.loads((ROOT / f'build/extract/p{p:03d}.json').read_text())['spans']
                   if s['cls'] == 'text']
    printed = [int(page['printed']) for page in pages if page['printed'].isdigit()]
    procs = sorted(name for name, at in pseudocode_index(index_spans).items()
                   if printed and any(printed[0] <= p <= printed[-1] for p in at))
    figures = figure_destinations(pymupdf.open(ROOT / 'upstream/Algorithms-JeffE.pdf'), key)
    return pages, toc, procs, figures


def load_rebuilt(key):
    import pymupdf
    from extract import raster_boxes
    from extract_figures import placements
    out = ROOT / 'build/verify' / key
    for sub in ('frontmatter', 'chapters', 'backmatter'):  # \include writes <sub>/<file>.aux there
        (out / sub).mkdir(parents=True, exist_ok=True)
    # A clean \include build can need more than latexmk's default 5 passes.
    run = subprocess.run(['latexmk', '-lualatex', '-interaction=nonstopmode', '-halt-on-error',
                          '-e', '$max_repeat=10', f'-outdir={out}',
                          f'-usepretex=\\def\\IncludeOnly{{{FILES[key]}}}\\def\\SkipTOC{{1}}', 'main.tex'],
                         cwd=ROOT / 'source', capture_output=True, text=True)
    if run.returncode != 0:
        sys.exit(f'LaTeX build failed; see {out / "main.log"}\n' + run.stdout[-2000:])
    doc = pymupdf.open(out / 'main.pdf')
    pages = []
    for page in doc:
        boxes = raster_boxes(page) + [tuple(rect) for _, rect in placements(doc, page)]
        spans = []
        for block in page.get_text('dict')['blocks']:
            for line in block.get('lines', []):
                for span in line['spans']:
                    if span['text'].strip():
                        spans.append({'text': span['text'], 'font': span['font'], 'bbox': list(span['bbox']),
                                      'size': span['size'],
                                      'cls': classify(span['font'], span['bbox'], page.rect.height, boxes)})
        links = [{'name': link.get('nameddest') or link.get('name')} for link in page.get_links()]
        pages.append({'page': page.number + 1, 'spans': spans, 'links': links})
    toc = [(level, normalize_title(title)) for level, title, _ in doc.get_toc() if level >= 2]
    return pages, toc, figure_destinations(doc, key)


def source_procs(key):
    text = (ROOT / 'source' / f'{FILES[key]}.tex').read_text()
    return sorted(set(re.findall(r'\\Proc\{([^}]*)\}', text)))


def page_of(pages):
    """Token index -> PDF page number (approximate at hyphenated page breaks)."""
    owner = []
    for page in pages:
        owner += [page['page']] * len(words(page['spans']))
    return owner


def main(key):
    orig_pages, orig_toc, index_procs, orig_figures = load_original(key)
    new_pages, new_toc, new_figures = load_rebuilt(key)
    orig_spans = [s for p in orig_pages for s in p['spans']]
    new_spans = [s for p in new_pages for s in p['spans']]
    original, rebuilt = words(orig_spans), words(new_spans)
    report = compare(original, rebuilt)
    owner = page_of(orig_pages)

    checks = {
        'figures': (orig_figures, new_figures),
        'footnotes': (len(footnote_names([l for p in orig_pages for l in p['links']])),
                      len(footnote_names([l for p in new_pages for l in p['links']]))),
        'exercises': (exercise_numbers(orig_spans), exercise_numbers(new_spans)),
        'sections': (orig_toc, new_toc),
    }
    missing_procs = sorted(set(index_procs) - set(source_procs(key)))
    failed = [name for name, (a, b) in checks.items() if a != b] + (['procs'] if missing_procs else [])
    math = compare(math_symbols(orig_spans), math_symbols(new_spans), min_move=8)

    lines = [f'{key}: mismatch {report.mismatch_ratio:.4%} (limit {MAX_RATIO:.2%}); '
             f'original {len(original)} tokens, rebuilt {len(rebuilt)} tokens; '
             f'{len(report.hunks)} hunks, {len(report.moved)} moved blocks',
             f'structure: {"ok" if not failed else "MISMATCH " + ", ".join(failed)}']
    for name, (a, b) in checks.items():
        lines.append(f'  {name}: {"ok" if a == b else "DIFF"} original={a} rebuilt={b}'
                     if name != 'sections' else f'  sections: {"ok" if a == b else "DIFF"} ({len(a)} vs {len(b)})')
        if name == 'sections' and a != b:
            lines += [f'    {x!r:60} | {y!r}' for x, y in zip(a + [None] * (len(b) - len(a)), b + [None] * (len(a) - len(b)))]
    lines.append(f'  procs: {"ok" if not missing_procs else "MISSING " + str(missing_procs)} '
                 f'(index {len(index_procs)}, source {len(source_procs(key))})')
    lines.append(f'math symbols (review only): mismatch {math.mismatch_ratio:.2%}, {len(math.hunks)} hunks')
    missing, extra = residual_difference(report.hunks)
    lines.append(f'residual after ignoring order: missing={missing} extra={extra}')
    lines.append('hunks:')
    at = 0
    for a, b in report.hunks:
        # locate the hunk in the original for review
        while at < len(original) and a and original[at:at + len(a)] != a:
            at += 1
        page = owner[at] if a and at < len(owner) else '?'
        lines.append(f'- p{page}: {" ".join(a)!r} -> {" ".join(b)!r}')
    lines.append('moved (matched, review order):')
    lines += [f'- p{owner[i] if i < len(owner) else "?"}: {" ".join(tokens[:12])}{" …" if len(tokens) > 12 else ""}'
              f' ({len(tokens)} tokens)' for tokens, i, _ in report.moved]
    lines.append('math hunks:')
    lines += [f'- {"".join(a)!r} -> {"".join(b)!r}' for a, b in math.hunks]
    out = ROOT / 'build/verify' / f'{key}.txt'
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text('\n'.join(lines) + '\n')
    print('\n'.join(lines[:60]))
    return 0 if report.mismatch_ratio <= MAX_RATIO and not failed else 1


if __name__ == '__main__':
    sys.exit(main(sys.argv[1]))
