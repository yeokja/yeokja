"""Keep English heading links valid alongside mdBook's Korean heading IDs.

Use mdBook's rendered source as the authority for IDs, including duplicate
headings and inline formatting. Do not guess its slug algorithm from Markdown.
"""
import argparse
import html
from html.parser import HTMLParser
from pathlib import Path


class Headings(HTMLParser):
    def __init__(self, text):
        super().__init__(convert_charrefs=True)
        self.in_main = False
        self.headings = []
        self.ids = set()
        self.line_offsets = [0]
        for line in text.splitlines(keepends=True):
            self.line_offsets.append(self.line_offsets[-1] + len(line))
        self.feed(text)

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if 'id' in attrs:
            self.ids.add(attrs['id'])
        if tag == 'main':
            self.in_main = True
        if self.in_main and tag in ('h1', 'h2', 'h3', 'h4', 'h5', 'h6'):
            line, column = self.getpos()
            end = self.line_offsets[line - 1] + column + len(self.get_starttag_text())
            self.headings.append((tag, attrs.get('id'), end))

    def handle_endtag(self, tag):
        if tag == 'main':
            self.in_main = False


def preserve_anchors(original, translated):
    source, target = Headings(original), Headings(translated)
    if [h[0] for h in source.headings] != [h[0] for h in target.headings]:
        raise ValueError('translation changed the number or levels of headings')
    insertions = []
    owners = {h[1]: i for i, h in enumerate(target.headings) if h[1]}
    for i, ((_, old_id, _), (_, new_id, end)) in enumerate(zip(source.headings, target.headings)):
        if old_id and old_id != new_id:
            if old_id in owners and owners[old_id] != i:
                raise ValueError(f'original anchor collides with another heading: {old_id}')
            if old_id not in target.ids:
                insertions.append((end, f'<span id="{html.escape(old_id, quote=True)}"></span>'))
                target.ids.add(old_id)
    for offset, alias in reversed(insertions):
        translated = translated[:offset] + alias + translated[offset:]
    return translated


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('original', type=Path)
    parser.add_argument('translated', type=Path)
    args = parser.parse_args()
    count = 0
    for source in sorted(args.original.rglob('*.html')):
        relative = source.relative_to(args.original)
        if relative == Path("print.html"):
            continue  # Rebuilt from the repaired chapters by build_print.py.
        target = args.translated / relative
        if not target.is_file():
            raise ValueError(f'missing translated page: {relative}')
        try:
            result = preserve_anchors(source.read_text(), target.read_text())
        except ValueError as error:
            raise ValueError(f'{relative}: {error}') from error
        target.write_text(result)
        count += 1
    print(f'Preserved original heading anchors in {count} HTML pages')


if __name__ == '__main__':
    main()
