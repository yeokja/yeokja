"""Compare the Korean build with the English reference build and fail on any
difference translation must not make.

- The two builds have the same pages.
- Every page has the same block structure in its article: the same number of
  headings (and the same heading ids, in order), definition lists, admonitions
  of each type, mermaid diagrams, code blocks, tables, rows, cells, lists, list
  items and block quotes.
- The Korean build has no broken local link or `#anchor` the English build
  does not have. A file is looked up by its exact spelling, so a link that only
  resolves on a case-insensitive filesystem is broken.
- The navigation is translated: labels without Hangul are listed, and more than
  MAX_LATIN_NAV of them fails.

usage: check_site.py <english-site> <korean-site>
"""
from collections import Counter
from functools import lru_cache
from html.parser import HTMLParser
import os
from pathlib import Path
import re
import sys
from urllib.parse import unquote, urlsplit

COUNTED = ('h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'dl', 'dt', 'dd', 'table', 'tr', 'th', 'td',
           'ul', 'ol', 'li', 'blockquote', 'pre')
VOID = {'area', 'base', 'br', 'col', 'embed', 'hr', 'img', 'input', 'link', 'meta', 'source', 'track', 'wbr'}
HANGUL = re.compile(r'[가-힣]')
MAX_LATIN_NAV = 0.25


class Page(HTMLParser):
    def __init__(self, file):
        super().__init__(convert_charrefs=True)
        self.ids, self.links = set(), []
        self.shape, self.heading_ids, self.nav = Counter(), [], []
        self.stack = []          # open elements as (tag, marker)
        self.label = None
        self.feed(file.read_text())

    def inside(self, marker):
        return any(m == marker for _, m in self.stack)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        classes = (attrs.get('class') or '').split()
        if attrs.get('id'):
            self.ids.add(attrs['id'])
        if tag == 'a' and attrs.get('name'):
            self.ids.add(attrs['name'])
        for key in ('href', 'src'):
            if attrs.get(key):
                self.links.append(attrs[key])
        marker = None
        if tag == 'article':
            marker = 'article'
        elif tag == 'nav' and 'md-nav--primary' in classes:
            marker = 'nav'
        elif tag == 'span' and 'md-ellipsis' in classes and self.inside('nav'):
            marker, self.label = 'label', ''
        if self.inside('article'):
            if tag in COUNTED:
                self.shape[tag] += 1
            if tag in COUNTED[:6]:
                self.heading_ids.append(attrs.get('id'))
            if tag == 'div' and 'admonition' in classes:
                self.shape['admonition:' + ' '.join(c for c in classes if c != 'admonition')] += 1
            if 'mermaid' in classes:
                self.shape['mermaid'] += 1
            if tag == 'div' and 'highlight' in classes:
                self.shape['highlight'] += 1
        if tag not in VOID:
            self.stack.append((tag, marker))

    def handle_endtag(self, tag):
        for index in range(len(self.stack) - 1, -1, -1):
            if self.stack[index][0] == tag:
                if self.stack[index][1] == 'label':
                    self.nav.append(' '.join(self.label.split()))
                    self.label = None
                del self.stack[index:]
                return

    def handle_data(self, data):
        if self.label is not None:
            self.label += data


@lru_cache(maxsize=None)
def names(directory):
    return frozenset(os.listdir(directory)) if os.path.isdir(directory) else frozenset()


def exists_exact(root, target):
    """Whether `target` exists under `root` spelled exactly this way."""
    at = str(root)
    for part in Path(target).relative_to(root).parts:
        if part not in names(at):
            return False
        at = os.path.join(at, part)
    return True


def load(root):
    root = Path(root).resolve()
    return root, {page.resolve(): Page(page) for page in sorted(root.rglob('*.html'))}


def broken(root, pages):
    missing = set()
    for file, page in pages.items():
        for link in page.links:
            url = urlsplit(link)
            if url.scheme or url.netloc or url.path.startswith('/'):
                continue
            target = Path(os.path.normpath(file.parent / unquote(url.path))) if url.path else file
            if not target.is_relative_to(root):
                continue
            if target.is_dir():
                target = target / 'index.html'
            name = str(file.relative_to(root))
            if not exists_exact(root, target):
                missing.add((name, link, 'file'))
            elif url.fragment and target in pages and unquote(url.fragment) not in pages[target].ids:
                missing.add((name, link, 'fragment'))
    return missing


def main(english, korean):
    en_root, en = load(english)
    ko_root, ko = load(korean)
    failures = []

    en_pages = {str(p.relative_to(en_root)): page for p, page in en.items()}
    ko_pages = {str(p.relative_to(ko_root)): page for p, page in ko.items()}
    for name in sorted(set(en_pages) ^ set(ko_pages)):
        failures.append(f'{name}: only in the {"English" if name in en_pages else "Korean"} build')

    totals = Counter()
    for name in sorted(set(en_pages) & set(ko_pages)):
        a, b = en_pages[name], ko_pages[name]
        totals.update(b.shape)
        if a.shape != b.shape:
            diff = {k: (a.shape[k], b.shape[k]) for k in sorted(set(a.shape) | set(b.shape)) if a.shape[k] != b.shape[k]}
            failures.append(f'{name}: structure differs (English, Korean): {diff}')
        if a.heading_ids != b.heading_ids:
            failures.append(f'{name}: heading ids differ')

    new_broken = broken(ko_root, ko) - broken(en_root, en)
    for item in sorted(new_broken)[:60]:
        failures.append(f'broken link: {item}')

    nav = ko_pages['index.html'].nav if 'index.html' in ko_pages else []
    latin = sorted({label for label in nav if not HANGUL.search(label)})
    if not nav:
        failures.append('index.html: no navigation labels found')
    elif len(latin) > MAX_LATIN_NAV * len(set(nav)):
        failures.append(f'navigation: {len(latin)} of {len(set(nav))} distinct labels have no Hangul')

    admonitions = sum(count for key, count in totals.items() if key.startswith('admonition:'))
    print(f'pages: {len(ko_pages)}  headings: {sum(totals[h] for h in COUNTED[:6])}  '
          f'definition lists: {totals["dl"]} ({totals["dt"]} terms)  admonitions: {admonitions}  '
          f'mermaid: {totals["mermaid"]}  code blocks: {totals["highlight"]}  tables: {totals["table"]}')
    print(f'links checked: {sum(len(p.links) for p in ko.values())}  new broken: {len(new_broken)}')
    print(f'navigation labels: {len(nav)} ({len(set(nav))} distinct), without Hangul: {len(latin)}')
    for label in latin:
        print(f'  nav without Hangul: {label}')
    for failure in failures:
        print(f'FAIL {failure}')
    print(f'failures: {len(failures)}')
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main(*sys.argv[1:3]))
