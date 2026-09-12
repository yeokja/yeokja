"""Keep source heading links valid using the rendered English lesson IDs."""
import html
from html.parser import HTMLParser


class Headings(HTMLParser):
    def __init__(self, text):
        super().__init__(convert_charrefs=True)
        self.main_depth = 0
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
        if tag == 'div':
            if 'lesson-main' in attrs.get('class', '').split():
                self.main_depth = 1
            elif self.main_depth:
                self.main_depth += 1
        if self.main_depth and tag in ('h1', 'h2', 'h3', 'h4', 'h5', 'h6'):
            line, column = self.getpos()
            end = self.line_offsets[line - 1] + column + len(self.get_starttag_text())
            self.headings.append((tag, attrs.get('id'), end))

    def handle_endtag(self, tag):
        if tag == 'div' and self.main_depth:
            self.main_depth -= 1


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
