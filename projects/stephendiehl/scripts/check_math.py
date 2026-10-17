"""Check that the Korean build renders the same math as the English build.

mdbook-katex turns every ``$...$``/``$$...$$`` into a ``<span class="katex">``
at build time. A translation that drops, adds or breaks a math span changes
that count on its page, and math that leaked into a code span shows up as an
escaped ``&lt;span class="katex`` inside ``<code>``. Usage::

    check_math.py <english-book> <korean-book>
"""
from pathlib import Path
import re
import sys

KATEX = re.compile(r'<span class="katex(?:-display)?"')
LEAKED = re.compile(r'&lt;span class="katex')


def katex_counts(root):
    root = Path(root)
    return {str(p.relative_to(root)): len(KATEX.findall(p.read_text())) for p in root.rglob('*.html')}


def leaked_pages(root):
    root = Path(root)
    return sorted(str(p.relative_to(root)) for p in root.rglob('*.html') if LEAKED.search(p.read_text()))


def compare(original, translated):
    """Pages whose KaTeX span count differs, as (page, original, translated)."""
    source, target = katex_counts(original), katex_counts(translated)
    return sorted((page, source[page], target.get(page)) for page in source if source[page] != target.get(page))


if __name__ == '__main__':
    original, translated = sys.argv[1], sys.argv[2]
    mismatched = compare(original, translated)
    leaked = leaked_pages(translated)
    total = sum(katex_counts(translated).values())
    print(f'KaTeX spans in the Korean build: {total}; pages with a different count from the English build: {len(mismatched)}')
    for page, before, after in mismatched[:60]:
        print(f'  {page}: {before} -> {after}')
    if leaked:
        print(f'math rendered inside code on {len(leaked)} page(s): {leaked[:20]}')
    sys.exit(bool(mismatched or leaked))
