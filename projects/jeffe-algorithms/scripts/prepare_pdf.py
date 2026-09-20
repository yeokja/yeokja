"""Prepare the assembled tree for the Korean LuaLaTeX build.

The Korean edition is switched on by latexmk (-usepretex='\\def\\KoreanEdition{1}'),
and the Korean copyright page lives in the untranslated base tree. This
checks that the tree has what the build needs and that no translated chapter
flips the edition switch itself.

It also turns the ASCII double quotes that translations sometimes use
("단어") into curly pairs (“단어”): with Ligatures=TeX fonts an ASCII quote
prints as a closing quote on both sides. Tree files are symlinks into ko/ and
source/, so a changed file is replaced, never edited through the link.
"""
from pathlib import Path
import sys

TRANSLATED = ('chapters/*.tex', 'frontmatter/preface.tex', 'backmatter/*.tex')


def check(tree):
    tree = Path(tree)
    problems = [f'missing {p}' for p in ('main.tex', 'preamble.tex', 'frontmatter/translation-notice.tex')
                if not (tree / p).exists()]
    for tex in sorted((tree / 'chapters').glob('*.tex')):
        if '\\koreantrue' in tex.read_text():
            problems.append(f'{tex.name}: edition switch inside a chapter')
    return problems


def smart_quotes(line):
    """Pair ASCII double quotes outside math into “ ”; leave an odd one alone."""
    quotes, math, at = [], False, 0
    while at < len(line):
        ch = line[at]
        if ch == '\\':
            at += 2
            continue
        if ch == '$':
            math = not math
        elif ch == '"' and not math:
            quotes.append(at)
        at += 1
    chars = list(line)
    for opening, closing in zip(quotes[0::2], quotes[1::2]):
        chars[opening], chars[closing] = '“', '”'
    return ''.join(chars)


def fix_quotes(tree):
    tree, changed = Path(tree), []
    for pattern in TRANSLATED:
        for tex in sorted(tree.glob(pattern)):
            text = tex.read_text()
            fixed = ''.join(smart_quotes(line) for line in text.splitlines(keepends=True))
            if fixed != text:
                tex.unlink()
                tex.write_text(fixed)
                changed.append(tex.name)
    return changed


if __name__ == '__main__':
    problems = check(sys.argv[1])
    for p in problems:
        print(p, file=sys.stderr)
    if problems:
        sys.exit(1)
    for name in fix_quotes(sys.argv[1]):
        print(f'curly quotes: {name}')
