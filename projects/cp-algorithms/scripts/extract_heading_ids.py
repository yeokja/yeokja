"""Map each page of an English MkDocs build to its Markdown heading ids.

Only headings that carry a toc permalink (`a.headerlink`) come from Markdown;
template headings and raw-HTML headings are skipped, matching what the
ko_anchors treeprocessor sees.

usage: extract_heading_ids.py <site> <out.json>
"""
from html.parser import HTMLParser
import json
from pathlib import Path
import sys


class Headings(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.ids, self.current = [], None

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag in ('h1', 'h2', 'h3', 'h4', 'h5', 'h6'):
            self.current = attrs.get('id')
        elif tag == 'a' and self.current and 'headerlink' in (attrs.get('class') or ''):
            self.ids.append(self.current)
            self.current = None

    def handle_endtag(self, tag):
        if tag in ('h1', 'h2', 'h3', 'h4', 'h5', 'h6'):
            self.current = None


def extract(site):
    site = Path(site)
    pages = {}
    for page in sorted(site.rglob('*.html')):
        parser = Headings()
        parser.feed(page.read_text())
        if parser.ids:
            pages[page.relative_to(site).with_suffix('.md').as_posix()] = parser.ids
    return pages


if __name__ == '__main__':
    Path(sys.argv[2]).write_text(json.dumps(extract(sys.argv[1]), ensure_ascii=False, indent=1))
