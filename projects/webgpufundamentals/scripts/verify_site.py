"""Reject incomplete lessons, broken metadata, and lost local assets."""
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit
import re
import sys

root = Path(__file__).resolve().parents[1]
site = Path(sys.argv[1]).resolve()
base = '/webgpufundamentals'
failures = []
class Links(HTMLParser):
    def __init__(self):
        super().__init__()
        self.urls = []
        self.ids = set()
    def handle_starttag(self, tag, attrs):
        for key, value in attrs:
            if value and (key == "id" or (tag == "a" and key == "name")):
                self.ids.add(value)
            if key in {'src', 'href', 'poster'} and value:
                self.urls.append((tag, value))

parsed_pages = {}

for source in sorted((root / 'upstream/webgpu/lessons').glob('*.md')):
    page = site / 'webgpu/lessons/ko' / source.with_suffix('.html').name
    if not page.is_file():
        failures.append(f'missing lesson: {page.name}')
        continue
    text = page.read_text()
    if not re.search(r'[가-힣]', text):
        failures.append(f'no Korean: {page.name}')
    if '죄송합니다. 이 글은 아직 번역이 되지 않았습니다.' in text:
        failures.append(f'placeholder lesson: {page.name}')
    parser = Links()
    parser.feed(text)
    parsed_pages[page.resolve()] = parser
    for tag, url in parser.urls:
        parts = urlsplit(url)
        if parts.scheme or parts.netloc:
            continue
        path = unquote(parts.path)
        if not path:
            target = page
        elif path.startswith(base + '/'):
            target = site / path[len(base)+1:]
        elif path.startswith('/'):
            failures.append(f'{page.name}: URL escapes project mount: {url}')
            continue
        else:
            target = page.parent / path
        if target.is_dir():
            target = target / 'index.html'
        if not target.exists():
            failures.append(f'{page.name}: missing {tag} target: {url}')
        elif tag == 'a' and parts.fragment and target.suffix == '.html':
            key = target.resolve()
            if key not in parsed_pages:
                target_parser = Links()
                target_parser.feed(target.read_text())
                parsed_pages[key] = target_parser
            target_parser = parsed_pages[key]
            if unquote(parts.fragment) not in target_parser.ids:
                failures.append(f'{page.name}: missing fragment target: {url}')
if failures:
    raise SystemExit('\n'.join(sorted(set(failures))))
print('Verified every Korean lesson and its local href/src/poster and fragment targets.')
