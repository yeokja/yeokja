#!/usr/bin/env python3
"""Independently verify protected Markdeep content in translated Ray Tracing books.

No translation parser or catalogs are imported. Code is byte-for-byte checked
(after universal newline decoding); mathematical expressions are exact apart
from runs of whitespace. Diagnostics identify the book, line, and item number.
"""
from __future__ import annotations

import argparse
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
import re
import sys


@dataclass(frozen=True)
class Item:
    value: str
    line: int


FENCE = re.compile(r'^\s*(~{3,}|`{3,})([^\r\n]*)$')
MATH = re.compile(r'(?<!\\)(\$\$)(.*?)(?<!\\)\$\$|(?<![\\$])\$(?!\$)(.*?)(?<![\\$])\$(?!\$)|\\\[(.*?)\\\]|\\\((.*?)\\\)', re.S)
IDENTIFIER = re.compile(r'\[(Figure|Listing|Equation|Section)\s*\[([^]\n]+)\]', re.I)
DEFINITION = re.compile(r'^\s*\[([^][\n]+)\]:\s*(\S+)', re.M)
REFERENCE = re.compile(r'(?<!!)\[([^]\n]+)\]\[([^]\n]*)\]')
ASSET = re.compile(r'''\]\(\s*<?([^\s)>]+)>?(?:[^)]*)\)|\b(?:src|href)\s*=\s*["']([^"']+)["']''', re.I)


def extract(text: str) -> dict[str, list[Item]]:
    result: dict[str, list[Item]] = {key: [] for key in (
        'code', 'math', 'identifier', 'reference definition', 'reference use', 'bare reference', 'insert directive', 'asset')}
    lines = text.splitlines(keepends=True)
    masked = list(lines)
    block: list[str] | None = None
    start = 0
    marker = ''
    for index, line in enumerate(lines):
        fence = FENCE.match(line.rstrip('\r\n'))
        if block is not None:
            block.append(line)
            masked[index] = '\n' if line.endswith('\n') else ''
            # A language-bearing fence switches highlighting, it does not close.
            if fence and fence[1][0] == marker and not fence[2].strip():
                result['code'].append(Item(''.join(block), start + 1))
                block = None
        elif fence:
            block, start, marker = [line], index, fence[1][0]
            masked[index] = '\n' if line.endswith('\n') else ''
    if block is not None:
        result['code'].append(Item(''.join(block), start + 1))
    prose = ''.join(masked)

    def add(kind: str, pattern: re.Pattern, transform):
        for match in pattern.finditer(prose):
            result[kind].append(Item(transform(match), prose.count('\n', 0, match.start()) + 1))

    add('math', MATH, lambda m: re.sub(r'\s+', ' ', m[0]).strip())
    add('identifier', IDENTIFIER, lambda m: m[1].lower() + ' [' + m[2] + ']')
    add('reference definition', DEFINITION, lambda m: m[1] + ' -> ' + m[2])
    add('reference use', REFERENCE, lambda m: m[2] or m[1])
    add('bare reference', re.compile(r'\[([^][\n]+)\](?![:(\[])'), lambda m: m[1])
    add('insert directive', re.compile(r'\(insert\s+[^)]+\s+here\)'), lambda m: m[0])
    add('asset', ASSET, lambda m: m[1] or m[2])
    return result


def verify_book(source_text: str, target_text: str, filename: str = 'book') -> list[str]:
    source, target = extract(source_text), extract(target_text)
    errors = []
    for kind, expected in source.items():
        actual = target[kind]
        expected_values = [item.value for item in expected]
        actual_values = [item.value for item in actual]
        if expected_values == actual_values:
            continue
        # Korean syntax can reorder expressions and references inside a sentence.
        # Definitions and whole code listings still retain their original order.
        if kind in {'math', 'reference use', 'bare reference', 'asset'}:
            missing = Counter(expected_values) - Counter(actual_values)
            extra = Counter(actual_values) - Counter(expected_values)
            if not missing and not extra:
                continue
            for label, difference, items in [('missing', missing, expected), ('unexpected', extra, actual)]:
                if difference:
                    value = next(iter(difference))
                    line = next(item.line for item in items if item.value == value)
                    location = 'upstream ' if label == 'missing' else ''
                    errors.append(f'{filename}: {kind} {label} {difference[value]} occurrence(s) of {value[:180]!r} ({location}line {line})')
            continue
        if len(expected) != len(actual):
            errors.append(f'{filename}: {kind} count changed: expected {len(expected)}, got {len(actual)}')
        for number, (before, after) in enumerate(zip(expected, actual), 1):
            if before.value == after.value:
                continue
            line = after.line
            if kind == 'code':
                before_lines, after_lines = before.value.splitlines(), after.value.splitlines()
                offset = next((i for i, pair in enumerate(zip(before_lines, after_lines)) if pair[0] != pair[1]), min(len(before_lines), len(after_lines)))
                line += offset
                expected_snippet = before_lines[offset] if offset < len(before_lines) else '<end>'
                actual_snippet = after_lines[offset] if offset < len(after_lines) else '<end>'
            else:
                expected_snippet, actual_snippet = before.value, after.value
            errors.append(f'{filename}:{line}: {kind} #{number} differs (upstream line {before.line}); expected {expected_snippet[:180]!r}, got {actual_snippet[:180]!r}')
            # The first mismatch plus counts is usually enough to locate a shift.
            break
    return errors


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    project = Path(__file__).resolve().parents[1]
    parser.add_argument('--source', type=Path, default=project / 'upstream' / 'books')
    parser.add_argument('--target', type=Path, default=project / 'ko' / 'books')
    args = parser.parse_args()
    sources = sorted(args.source.glob('*.html'))
    if not sources:
        print(f'{args.source}: no upstream HTML books found', file=sys.stderr)
        return 1
    errors = []
    for source in sources:
        target = args.target / source.name
        if not target.is_file():
            errors.append(f'{target}: missing translated book')
            continue
        errors.extend(verify_book(source.read_text(encoding='utf-8'), target.read_text(encoding='utf-8'), str(target)))
    for error in errors:
        print(error, file=sys.stderr)
    if errors:
        return 1
    print(f'Verified {len(sources)} books: code, math, identifiers, references, and assets preserved.')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
