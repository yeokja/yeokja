"""Render original and reconstructed pages with cp-algorithms' Markdown
extensions and fail if the mkdocs parser changed their structure.

usage: check_parser_corpus.py <upstream/src> <roundtrip-out>
"""
from html.parser import HTMLParser
from pathlib import Path
import re
import sys

import markdown
from mkdocs.utils import meta

EXTENSIONS = ['toc', 'tables', 'pymdownx.arithmatex', 'pymdownx.highlight', 'admonition',
              'pymdownx.details', 'pymdownx.superfences', 'pymdownx.tabbed', 'attr_list', 'meta']
CONFIG = {'pymdownx.arithmatex': {'generic': True}, 'pymdownx.tabbed': {'alternate_style': True},
          'toc': {'permalink': True}}


def render(text):
    body, _ = meta.get_data(text)
    return markdown.Markdown(extensions=EXTENSIONS, extension_configs=CONFIG).convert(body)


class Shape(HTMLParser):
    """Tag/class sequence plus the verbatim contents of math and code."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.tags, self.raw, self.stack = [], [], []

    def handle_starttag(self, tag, attrs):
        cls = dict(attrs).get('class') or ''
        self.tags.append((tag, cls))
        verbatim = 'arithmatex' in cls or tag == 'code'
        self.stack.append((tag, verbatim))
        if verbatim and sum(v for _, v in self.stack) == 1:
            self.raw.append('')

    def handle_endtag(self, tag):
        self.tags.append(('/' + tag, ''))
        if self.stack and self.stack[-1][0] == tag:
            self.stack.pop()

    def handle_data(self, data):
        if any(v for _, v in self.stack):
            self.raw[-1] += re.sub(r'\s+', '', data)


def shape(html):
    s = Shape()
    s.feed(html)
    return s.tags, s.raw


def main(src, out):
    failures = []
    for original in sorted(Path(src).rglob('*.md')):
        rel = original.relative_to(src)
        expected = shape(render(original.read_text()))
        for mode in ('identity', 'fake'):
            path = Path(out) / mode / rel
            if not path.exists():
                continue
            if shape(render(path.read_text())) != expected:
                failures.append(f'{mode}: {rel}')
    for failure in failures:
        print(failure)
    print(f'structure differences: {len(failures)}')
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main(*sys.argv[1:3]))
