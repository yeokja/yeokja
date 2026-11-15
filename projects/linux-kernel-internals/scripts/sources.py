"""Derive the two translation sources that are not upstream files as they are.

nav/nav.md
    The labels of upstream mkdocs.yml's `nav:` as a nested Markdown list, one
    label per item, in nav order. Entries without a label (`- page.md`) have
    nothing to translate and are left out. prepare_site.py puts the translated
    labels back into mkdocs.yml in the same order.

normalized/docs/<page>.md
    Pages that write an admonition header without the space (`!!!note`).
    Python-Markdown accepts that spelling, the mkdocs parser does not: it reads
    header and body as one paragraph. The copy here has the space (`!!! note`)
    and nothing else changed, and yeokja.toml translates it in place of the
    upstream page.

usage: sources.py extract [--check]

`extract` rewrites both; `extract --check` fails when what is committed differs
from what the pinned upstream gives, or when yeokja.toml does not exclude
exactly the normalized pages from the upstream source rule.
"""
import json
from pathlib import Path
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parent.parent
LABEL = r'''"(?:[^"\\]|\\.)*"|'(?:[^']|'')*'|[^\s:"'][^:]*?'''
ENTRY = re.compile(rf'^(?P<indent> +)- (?P<label>{LABEL}):(?P<target> \S+\.md)?$')
PAGE = re.compile(r'^(?P<indent> +)- (?P<path>\S+\.md)$')
# Characters that would make a label something other than plain text in the
# nav list. Upstream uses none of them; a new one fails here, not in the nav.
MARKUP = re.compile(r'[\\`*\[\]<>]|(?<![A-Za-z0-9])_|_(?![A-Za-z0-9])')
HEADER = re.compile(r'^(\s*)!!!(?=\w)')
FENCE = re.compile(r'^\s*(```|~~~)')


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


def normalize(text):
    """Give `!!!type` headers their space; None when the page has none."""
    out, fenced, changed = [], False, False
    for line in text.split('\n'):
        if FENCE.match(line):
            fenced = not fenced
        elif not fenced and HEADER.match(line):
            line = HEADER.sub(r'\1!!! ', line)
            changed = True
        out.append(line)
    return '\n'.join(out) if changed else None


def derive(upstream):
    """Every derived file as {path relative to the project: content}."""
    files = {'nav/nav.md': nav_markdown((upstream / 'mkdocs.yml').read_text())}
    docs = upstream / 'docs'
    for page in sorted(docs.rglob('*.md')):
        fixed = normalize(page.read_text())
        if fixed is not None:
            files[f'normalized/docs/{page.relative_to(docs).as_posix()}'] = fixed
    return files


def committed(root):
    found = {}
    for directory in ('nav', 'normalized'):
        for item in sorted((root / directory).rglob('*')):
            if item.is_file():
                found[item.relative_to(root).as_posix()] = item.read_text()
    return found


def excluded_pages(root):
    config = tomllib.loads((root / 'yeokja.toml').read_text())
    for source in config['sources']:
        if source['path'] == 'upstream/docs':
            return set(source.get('exclude', []))
    raise ValueError('yeokja.toml has no upstream/docs source')


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
    normalized = {name.removeprefix('normalized/docs/') for name in files if name.startswith('normalized/')}
    excluded = excluded_pages(root)
    for page in sorted(normalized - excluded):
        problems.append(f'{page}: normalized but not excluded from the upstream/docs source in yeokja.toml')
    for page in sorted(page for page in excluded - normalized if (root / 'normalized/docs' / page).exists()):
        problems.append(f'{page}: excluded for normalization but upstream no longer needs it')
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
