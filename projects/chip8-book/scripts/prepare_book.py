"""Turn the pandoc manuscript into a multi-page mdBook source tree, in place.

The upstream book is written for pandoc, which concatenates every chapter into
one document. Two of its idioms break once mdBook gives each chapter its own
page, so this script rewrites them in the copy that is about to be built (the
original sources and ko/ are never touched):

- A line holding only ``\\newpage`` is a raw LaTeX page break for the PDF.
  mdBook would print it as text, so the line is removed.
- ``](#id)`` links to a ``{#id}`` heading attribute in another chapter would
  point at the current page. They become ``](file.md#id)``. The heading IDs are
  read from the tree itself, so the same mapping applies to the English and the
  Korean build. A link to an ID that no heading defines is an error.
"""
from pathlib import Path
import re
import sys

# yeokja's Markdown serializer escapes the backslash (``\\newpage``); CommonMark
# renders both spellings as the same text, so both are page breaks.
NEWPAGE = re.compile(r'^[ \t]*\\{1,2}newpage[ \t]*\n?', re.M)
HEADING_ID = re.compile(r'^#{1,6}[ \t].*\{#([A-Za-z][\w:.-]*)\}[ \t]*$', re.M)
LOCAL_LINK = re.compile(r'\]\(#([A-Za-z][\w:.-]*)\)')
FENCE = re.compile(r'^(```|~~~)')


def split_code(text):
    """Yield (is_code, chunk) so rewrites skip fenced code blocks."""
    chunk, code, fence = [], False, None
    for line in text.splitlines(keepends=True):
        match = FENCE.match(line)
        if match and (not code or line.startswith(fence)):
            if not code:
                if chunk:
                    yield False, ''.join(chunk)
                chunk, code, fence = [line], True, match.group(1)
                continue
            chunk.append(line)
            yield True, ''.join(chunk)
            chunk, code, fence = [], False, None
            continue
        chunk.append(line)
    if chunk:
        yield code, ''.join(chunk)


def heading_ids(pages):
    owners = {}
    for name, text in pages.items():
        for is_code, chunk in split_code(text):
            if is_code:
                continue
            for anchor in HEADING_ID.findall(chunk):
                if anchor in owners:
                    raise ValueError(f'heading ID #{anchor} defined in {owners[anchor]} and {name}')
                owners[anchor] = name
    return owners


def prepare(name, text, owners):
    out = []
    for is_code, chunk in split_code(text):
        if not is_code:
            chunk = NEWPAGE.sub('', chunk)

            def link(match):
                anchor = match.group(1)
                if anchor not in owners:
                    raise ValueError(f'{name}: link to undefined heading ID #{anchor}')
                if owners[anchor] == name:
                    return match.group(0)
                return f']({owners[anchor]}#{anchor})'

            chunk = LOCAL_LINK.sub(link, chunk)
        out.append(chunk)
    return ''.join(out).lstrip('\n')


def main(root):
    root = Path(root)
    pages = {p.name: p.read_text() for p in sorted(root.glob('*.md')) if p.name != 'SUMMARY.md'}
    if not pages:
        raise ValueError(f'no chapters in {root}')
    owners = heading_ids(pages)
    breaks = 0
    for name, text in pages.items():
        result = prepare(name, text, owners)
        breaks += len(NEWPAGE.findall(''.join(c for code, c in split_code(text) if not code)))
        prose = ''.join(c for code, c in split_code(result) if not code)
        if re.search(r'\\newpage', prose):
            raise ValueError(f'{name}: \\newpage left inside a line')
        (root / name).write_text(result)
    rewritten = sum(len(re.findall(r'\]\([\w.-]+\.md#', (root / n).read_text())) for n in pages)
    print(f'Prepared {len(pages)} chapters: removed {breaks} \\newpage lines, '
          f'{rewritten} cross-chapter heading links, {len(owners)} heading IDs')


if __name__ == '__main__':
    main(sys.argv[1])
