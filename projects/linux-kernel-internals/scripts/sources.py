"""Derive the translation source that is not an upstream file as it is.

nav/nav.md
    The labels of upstream mkdocs.yml's `nav:` as a nested Markdown list, one
    label per item, in nav order. Entries without a label (`- page.md`) have
    nothing to translate and are left out. prepare_site.py puts the translated
    labels back into mkdocs.yml in the same order.

usage: sources.py extract [--check]

`extract` rewrites it; `extract --check` fails when what is committed differs
from what the pinned upstream gives.
"""
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parent.parent
LABEL = r'''"(?:[^"\\]|\\.)*"|'(?:[^']|'')*'|[^\s:"'][^:]*?'''
ENTRY = re.compile(rf'^(?P<indent> +)- (?P<label>{LABEL}):(?P<target> \S+\.md)?$')
PAGE = re.compile(r'^(?P<indent> +)- (?P<path>\S+\.md)$')
# Characters that would make a label something other than plain text in the
# nav list. Upstream uses none of them; a new one fails here, not in the nav.
MARKUP = re.compile(r'[\\`*\[\]<>]|(?<![A-Za-z0-9])_|_(?![A-Za-z0-9])')


def nav_lines(config_text):
    """The lines of the `nav:` block, as (line_number, line)."""
    lines = config_text.split('\n')
    try:
        start = lines.index('nav:')
    except ValueError:
        raise ValueError('mkdocs.yml has no top-level nav:') from None
    block = []
    for number in range(start + 1, len(lines)):
        line = lines[number]
        if line and not line.startswith(' '):
            break
        if line.strip():
            block.append((number, line))
    return block


def unquote(label):
    if label.startswith('"'):
        return json.loads(label)
    if label.startswith("'"):
        return label[1:-1].replace("''", "'")
    return label


def parse_nav(config_text):
    """Every nav line as (line_number, depth, label or None, match)."""
    entries = []
    for number, line in nav_lines(config_text):
        match = ENTRY.match(line)
        if match:
            label = unquote(match['label'])
        else:
            match = PAGE.match(line)
            if not match:
                raise ValueError(f'mkdocs.yml:{number + 1}: unrecognized nav line {line!r}')
            label = None
        indent = len(match['indent'])
        if indent % 2:
            raise ValueError(f'mkdocs.yml:{number + 1}: odd nav indentation')
        entries.append((number, indent // 2 - 1, label, match))
    return entries


def nav_markdown(config_text):
    out = []
    for number, depth, label, _ in parse_nav(config_text):
        if label is None:
            continue
        if MARKUP.search(label):
            raise ValueError(f'mkdocs.yml:{number + 1}: nav label {label!r} has Markdown markup')
        out.append(f'{"    " * depth}- {label}\n')
    return ''.join(out)


def derive(upstream):
    """Every derived file as {path relative to the project: content}."""
    return {'nav/nav.md': nav_markdown((upstream / 'mkdocs.yml').read_text())}


def committed(root):
    return {item.relative_to(root).as_posix(): item.read_text()
            for item in sorted((root / 'nav').rglob('*')) if item.is_file()}


def check(root, files):
    problems = []
    have = committed(root)
    for name in sorted(set(files) | set(have)):
        if name not in have:
            problems.append(f'{name}: missing (run sources.py extract)')
        elif name not in files:
            problems.append(f'{name}: no longer derived from upstream (run sources.py extract)')
        elif have[name] != files[name]:
            problems.append(f'{name}: differs from upstream (run sources.py extract)')
    return problems


def main(argv):
    if not argv or argv[0] != 'extract' or argv[1:] not in ([], ['--check']):
        print(__doc__)
        return 2
    files = derive(ROOT / 'upstream')
    if argv[1:]:
        problems = check(ROOT, files)
        for problem in problems:
            print(problem)
        print(f'derived sources: {len(files)} file(s), {len(problems)} problem(s)')
        return 1 if problems else 0
    for name in committed(ROOT):
        if name not in files:
            (ROOT / name).unlink()
    for name, content in files.items():
        target = ROOT / name
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content)
    print(f'derived sources: wrote {len(files)} file(s)')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1:]))
