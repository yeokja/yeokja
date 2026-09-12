#!/usr/bin/env python3
"""Extract landing-page prose for Yeokja and restore its original HTML markup.

Run from any directory: ``prepare_index.py extract`` then, after translating
ui/index.md to ko-ui/index.md with the Markdown parser, ``prepare_index.py render``.
Inline HTML is represented by immutable code spans so whole paragraphs retain
context while the original links, attributes and accessibility markup survive.
"""

import argparse
from dataclasses import dataclass
from html import escape, unescape
from html.parser import HTMLParser
from pathlib import Path
import re

ROOT = Path(__file__).resolve().parents[1]
TOKEN = re.compile(r'`HTML_\d{3}`')
MARKER = re.compile(r'<!-- yeokja-index:(\d{3}) -->\s*\n(.*?)(?=\n<!-- yeokja-index:|\Z)', re.S)
BLOCKS = {'title', 'h1', 'h2', 'h3', 'h4', 'h5', 'h6', 'p', 'li'}
BOUNDARIES = BLOCKS | {'div', 'ul', 'ol', 'section', 'main', 'header', 'footer', 'html', 'body', 'head'}


@dataclass
class Unit:
    start: int
    end: int
    kind: str
    text: str
    tags: list[str]


class IndexParser(HTMLParser):
    def __init__(self, source):
        super().__init__(convert_charrefs=False)
        self.source = source
        self.lines = [0] + [m.end() for m in re.finditer('\n', source)]
        self.units = []
        self.active = None
        self.inline = []

    def absolute_position(self):
        line, column = self.getpos()
        return self.lines[line - 1] + column

    def finish(self, end):
        if self.active is None:
            return
        start, kind = self.active
        raw = self.source[start:end]
        leading = len(raw) - len(raw.lstrip())
        trailing = len(raw.rstrip())
        start += leading
        end = self.active[0] + trailing
        raw = self.source[start:end]
        if raw:
            chunks = []
            tags = []
            cursor = start
            for a, b in self.inline:
                if a < start or b > end:
                    continue
                chunks.append(unescape(self.source[cursor:a]))
                chunks.append(f'`HTML_{len(tags):03d}`')
                tags.append(self.source[a:b])
                cursor = b
            chunks.append(unescape(self.source[cursor:end]))
            text = re.sub(r'\s+', ' ', ''.join(chunks)).strip()
            self.units.append(Unit(start, end, kind, text, tags))
        self.active = None
        self.inline = []

    def handle_starttag(self, tag, attrs):
        pos = self.absolute_position()
        raw = self.get_starttag_text()
        if tag in BOUNDARIES:
            self.finish(pos)
        if tag in BLOCKS:
            self.active = (pos + len(raw), tag)
        elif self.active is not None:
            self.inline.append((pos, pos + len(raw)))
        if tag == 'img':
            if self.active is not None:
                raise ValueError('Images inside prose need explicit extraction support')
            match = re.search(r'\balt\s*=\s*([\'"])(.*?)\1', raw, re.S | re.I)
            if not match:
                raise ValueError('Image must have a quoted alt attribute')
            if match[2].strip():
                self.units.append(Unit(pos + match.start(2), pos + match.end(2), 'alt', unescape(match[2]), []))

    def handle_startendtag(self, tag, attrs):
        self.handle_starttag(tag, attrs)

    def handle_endtag(self, tag):
        pos = self.absolute_position()
        if tag in BOUNDARIES:
            self.finish(pos)
        elif self.active is not None:
            self.inline.append((pos, self.source.index('>', pos) + 1))

    def handle_data(self, data):
        if data.strip() and self.active is None:
            raise ValueError(f'Uncovered visible text at {self.getpos()}: {data!r}')

    def handle_entityref(self, name):
        if self.active is None:
            raise ValueError(f'Uncovered entity: &{name};')

    handle_charref = handle_entityref

    def handle_comment(self, data):
        if self.active is not None:
            pos = self.absolute_position()
            self.inline.append((pos, self.source.index('-->', pos) + 3))


def extract_units(source):
    parser = IndexParser(source)
    parser.feed(source)
    parser.close()
    parser.finish(len(source))
    units = sorted(parser.units, key=lambda unit: unit.start)
    if not units:
        raise ValueError('No index translation units found')
    for previous, current in zip(units, units[1:]):
        if previous.end > current.start:
            raise ValueError('Overlapping index translation units')
    return units


def markdown(units):
    return '\n\n'.join(f'<!-- yeokja-index:{i:03d} -->\n\n{unit.text}' for i, unit in enumerate(units)) + '\n'


def render(source, translated):
    units = extract_units(source)
    matches = list(MARKER.finditer(translated))
    identifiers = [int(match[1]) for match in matches]
    if identifiers != list(range(len(units))):
        raise ValueError(f'Expected {len(units)} ordered translation units; got {identifiers}')
    if translated[:matches[0].start()].strip():
        raise ValueError('Unexpected content before first translation unit')
    replacements = []
    for unit, match in zip(units, matches):
        text = match[2].strip()
        if not re.search('[가-힣]', text):
            raise ValueError(f'Unit {match[1]} has no Korean translation')
        expected = [f'`HTML_{i:03d}`' for i in range(len(unit.tags))]
        if TOKEN.findall(text) != expected:
            raise ValueError(f'Unit {match[1]} has changed or missing HTML tokens')
        if '<!-- yeokja-index:' in text:
            raise ValueError(f'Unit {match[1]} contains an invalid unit marker')
        value = escape(text, quote=True)
        for token, tag in zip(expected, unit.tags):
            value = value.replace(token, tag)
        replacements.append((unit.start, unit.end, value))
    result = source
    for start, end, value in reversed(replacements):
        result = result[:start] + value + result[end:]
    # The upstream document omits its optional html element. Add a language
    # declaration for screen readers without changing its existing structure.
    if not re.search(r'<html\b', result, re.I):
        result = re.sub(r'(<!DOCTYPE html>)', r'\1\n<html lang="ko">', result, count=1, flags=re.I)
        result = result.rstrip() + '\n</html>\n'
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('command', choices=['extract', 'render'])
    parser.add_argument('--check', action='store_true', help='Check extracted Markdown freshness without writing')
    parser.add_argument('--source', type=Path, default=ROOT / 'upstream/index.html')
    parser.add_argument('--markdown', type=Path, default=ROOT / 'ui/index.md')
    parser.add_argument('--translated', type=Path, default=ROOT / 'ko-ui/index.md')
    parser.add_argument('--output', type=Path, default=ROOT / 'site/index.html')
    args = parser.parse_args()
    if args.check and args.command != 'extract':
        parser.error('--check is only valid with extract')
    source = args.source.read_text(encoding='utf-8')
    if args.command == 'extract':
        units = extract_units(source)
        extracted = markdown(units)
        if args.check:
            if not args.markdown.exists() or args.markdown.read_text(encoding='utf-8') != extracted:
                parser.error(f'{args.markdown} is missing or stale; run extract')
            print(f'Checked {len(units)} current translation units in {args.markdown}')
            return
        args.markdown.parent.mkdir(parents=True, exist_ok=True)
        args.markdown.write_text(extracted, encoding='utf-8')
        print(f'Extracted {len(units)} translation units to {args.markdown}')
    else:
        result = render(source, args.translated.read_text(encoding='utf-8'))
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(result, encoding='utf-8')
        print(f'Restored translated index to {args.output}')


if __name__ == '__main__':
    main()
