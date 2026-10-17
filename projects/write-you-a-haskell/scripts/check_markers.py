"""Fail when a translated chapter lost or gained build-sensitive markup.

The book renders `$`/`$$` math with mdbook-katex, splices Rust sources with
`{{#include path:anchor}}`, and resolves `[@key]` citations with mdbook-bib. A
translation that drops a dollar sign, rewrites an include path or translates a
citation key renders wrong without any mdBook warning, so compare the counts
file by file against the original before the build runs.
"""
from collections import Counter
from pathlib import Path
import re
import sys

INCLUDE = re.compile(r'\{\{#include [^}]+\}\}')
CITATION = re.compile(r'\[@[A-Za-z0-9_-]+\]')
FENCE = re.compile(r'^(`{3,}|~{3,})', re.M)


def markers(text):
    return {
        'dollar': text.count('$'),
        'include': Counter(INCLUDE.findall(text)),
        'citation': Counter(CITATION.findall(text)),
        'fence': len(FENCE.findall(text)),
    }


def compare(original, translated):
    """Return the differences between two chapters as `kind: original -> translated` lines."""
    before, after = markers(original), markers(translated)
    problems = []
    for kind in before:
        if before[kind] != after[kind]:
            if isinstance(before[kind], Counter):
                lost = before[kind] - after[kind]
                gained = after[kind] - before[kind]
                problems.append(f'{kind}: lost {sorted(lost.elements())} gained {sorted(gained.elements())}')
            else:
                problems.append(f'{kind}: {before[kind]} -> {after[kind]}')
    return problems


def check(original_root, translated_root):
    original_root, translated_root = Path(original_root), Path(translated_root)
    failures = 0
    for source in sorted(original_root.rglob('*.md')):
        relative = source.relative_to(original_root)
        target = translated_root / relative
        if not target.is_file():
            print(f'{relative}: missing translation')
            failures += 1
            continue
        for problem in compare(source.read_text(), target.read_text()):
            print(f'{relative}: {problem}')
            failures += 1
    return failures


if __name__ == '__main__':
    count = check(sys.argv[1], sys.argv[2])
    print(f'{count} marker mismatch(es)')
    sys.exit(bool(count))
