"""Verify every generated page and local navigation/image target."""
from html.parser import HTMLParser
from pathlib import Path
import sys
from urllib.parse import unquote, urljoin, urlsplit

root = Path(sys.argv[1]).resolve()
base = 'https://yeokja.moreal.dev/putting-the-you-in-cpu/'

class Page(HTMLParser):
    def __init__(self, text):
        super().__init__()
        self.ids = set()
        self.links = []
        self.language = None
        self.feed(text)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag == 'html':
            self.language = attrs.get('lang')
        if attrs.get('id'):
            self.ids.add(attrs['id'])
        for key in ('src', 'href'):
            if key in attrs:
                self.links.append(attrs[key])

pages = {p: Page(p.read_text()) for p in root.rglob('*.html')}
assert len(pages) == 10, f'Expected 10 pages, found {len(pages)}'
links = 0
for path, page in pages.items():
    assert page.language == 'ko', f'Wrong language: {path}'
    rel = str(path.relative_to(root))
    url = base + (rel[:-10] if rel.endswith('index.html') else rel)
    for link in page.links:
        parsed = urlsplit(urljoin(url, link))
        if parsed.netloc != 'yeokja.moreal.dev':
            continue
        prefix = '/putting-the-you-in-cpu'
        assert parsed.path == prefix or parsed.path.startswith(prefix + '/'), (path, link)
        dest = root / unquote(parsed.path[len(prefix):].lstrip('/'))
        if dest.is_dir():
            dest /= 'index.html'
        assert dest.exists(), (path, link, 'missing file')
        if parsed.fragment and dest in pages:
            assert unquote(parsed.fragment) in pages[dest].ids, (path, link, 'missing anchor')
        links += 1
assert (root/'LICENSE').read_text().startswith('MIT License')
print(f'Validated {len(pages)} Korean pages and {links} local links/assets.')
