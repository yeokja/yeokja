"""Check the built site against the upstream Markdown.

Usage: check_site.py <site-dir>   (run inside the assembled tree, YEOKJA_ROOT set)

For every upstream Markdown document, the translated page must have as many
headings, <details> boxes and footnotes as upstream rendered by the same
pandoc reader, and every relative href/src in the site must name an existing
file (and, with a fragment, an existing id in that page).
"""
from html.parser import HTMLParser
import os
from pathlib import Path
import posixpath
import re
import subprocess
import sys
from urllib.parse import unquote, urlsplit


def page_path(path):
    return path.with_name('index.html') if path.name == 'README.md' else path.with_suffix('.html')


def counts(markup):
    return {
        'headings': len(re.findall(r'<h[1-6][\s>]', markup)),
        'details': markup.count('<details'),
        'footnotes': len(re.findall(r'<a [^>]*class="footnote-ref"', markup)),
    }


class Collector(HTMLParser):
    def __init__(self):
        super().__init__()
        self.ids, self.links = set(), []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get('id'):
            self.ids.add(attrs['id'])
        if tag == 'a' and attrs.get('name'):
            self.ids.add(attrs['name'])
        for key in ('href', 'src'):
            if attrs.get(key):
                self.links.append(attrs[key])


def main():
    site = Path(sys.argv[1])
    upstream = Path(os.environ['YEOKJA_ROOT']) / 'upstream'
    errors = []
    documents = sorted(p.relative_to(upstream) for p in upstream.rglob('*.md'))
    for document in documents:
        original = subprocess.run(['pandoc', '-f', 'gfm', '-t', 'html5'], input=(upstream / document).read_text(),
                                  text=True, stdout=subprocess.PIPE, check=True).stdout
        built = (site / page_path(document)).read_text()
        main_part = built.split('<main>', 1)[1].rsplit('</main>', 1)[0]
        expected, actual = counts(original), counts(main_part)
        if expected != actual:
            errors.append(f'{document}: structure {actual} != upstream {expected}')
    pages = {}
    for html_file in site.rglob('*.html'):
        collector = Collector()
        collector.feed(html_file.read_text(errors='replace'))
        pages[html_file] = collector
    checked = 0
    for html_file, collector in sorted(pages.items()):
        for link in collector.links:
            parts = urlsplit(link)
            if parts.scheme or parts.netloc or link.startswith(('mailto:', 'data:')):
                continue
            if not parts.path:
                target = html_file
            else:
                target = Path(posixpath.normpath(str(html_file.parent / unquote(parts.path))))
            checked += 1
            if not target.exists():
                errors.append(f'{html_file.relative_to(site)}: missing {link}')
                continue
            if parts.fragment and target.suffix == '.html':
                ids = pages[target].ids if target in pages else set()
                if unquote(parts.fragment) not in ids:
                    errors.append(f'{html_file.relative_to(site)}: missing anchor {link}')
    for error in errors:
        print(error)
    print(f'Checked {len(documents)} documents and {checked} relative links: {len(errors)} problem(s)')
    sys.exit(1 if errors else 0)


if __name__ == '__main__':
    main()
