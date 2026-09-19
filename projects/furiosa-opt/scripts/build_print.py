"""Build the print view from translated chapters with unambiguous local IDs."""
import html
from html.parser import HTMLParser
from pathlib import Path
import posixpath
import re
from urllib.parse import quote, unquote, urlsplit, urlunsplit
import sys


class Page(HTMLParser):
    def __init__(self, text):
        super().__init__(convert_charrefs=True)
        self.text = text
        self.start = self.end = None
        self.tags = []
        self.ids = []
        self.links = []
        self.lines = [0]
        for line in text.splitlines(keepends=True):
            self.lines.append(self.lines[-1] + len(line))
        self.feed(text)

    def position(self):
        line, column = self.getpos()
        return self.lines[line - 1] + column

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        raw = self.get_starttag_text()
        if tag == 'a' and attrs.get('href'):
            self.links.append(attrs['href'])
        if tag == 'main':
            self.start = self.position() + len(raw)
        elif self.start is not None and self.end is None:
            self.tags.append((self.position(), raw))
            if attrs.get('id'):
                self.ids.append(attrs['id'])

    def handle_endtag(self, tag):
        if tag == 'main':
            self.end = self.position()

    def content(self):
        if self.start is None or self.end is None:
            raise ValueError('page has no main element')
        return self.text[self.start:self.end]


ATTR = re.compile(r'''([\w:-]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))''', re.I)


def rewrite(page, name, chapters):
    prefix = chapters[name]

    def attribute(match):
        attr = match.group(1).lower()
        if attr not in ('id', 'href', 'src', 'poster', 'for', 'aria-labelledby', 'aria-describedby'):
            return match.group(0)
        value = html.unescape(next(v for v in match.groups()[1:] if v is not None))
        if attr == 'id':
            value = f'{prefix}-{value}'
        elif attr in ('for', 'aria-labelledby', 'aria-describedby'):
            value = ' '.join(f'{prefix}-{v}' for v in value.split())
        else:
            url = urlsplit(value)
            if url.scheme or url.netloc or url.path.startswith('/'):
                return match.group(0)
            path = posixpath.normpath(posixpath.join(posixpath.dirname(name), unquote(url.path))) if url.path else name
            if attr == 'href' and path in chapters and not url.query:
                value = '#' + quote(chapters[path] + ('-' + unquote(url.fragment) if url.fragment else ''), safe='')
            else:
                value = urlunsplit(('', '', path, url.query, url.fragment))
        return f'{match.group(1)}="{html.escape(value, quote=True)}"'

    content = page.content()
    for offset, raw in reversed(page.tags):
        start = offset - page.start
        content = content[:start] + ATTR.sub(attribute, raw) + content[start + len(raw):]
    return f'<section id="{prefix}">\n{content}\n</section>\n'


def build_print(root):
    root = Path(root)
    toc = Page((root / 'toc.html').read_text())
    names = []
    for link in toc.links:
        url = urlsplit(link)
        if not url.scheme and not url.netloc and url.path.endswith('.html'):
            name = posixpath.normpath(unquote(url.path))
            if name not in names:
                names.append(name)
    if not names:
        raise ValueError('table of contents contains no chapters')
    chapters = {name: f'chapter-{i}' for i, name in enumerate(names)}
    pages = {name: Page((root / name).read_text()) for name in names}
    for name, page in pages.items():
        if len(page.ids) != len(set(page.ids)):
            raise ValueError(f'duplicate chapter IDs: {name}')
    contents = ''.join(rewrite(pages[name], name, chapters) for name in names)
    path = root / 'print.html'
    template = Page(path.read_text())
    template.content()  # Require the expected mdBook template before replacing it.
    path.write_text(template.text[:template.start] + contents + template.text[template.end:])
    print(f'Built print view from {len(names)} translated chapters')


if __name__ == '__main__':
    build_print(Path(sys.argv[1]))
