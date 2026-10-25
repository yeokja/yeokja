"""Check that opcode tables and opcode headings keep their machine tokens.

Table cells in 8-opcodes.md (the opcode table) and 5-instr.md (the font address
table), and the ``### 8XY4 - VX += VY`` headings, are translated as prose
segments. Their opcodes, registers, hex literals, numbers and operators are not
prose, and a dropped or "corrected" token would silently change the reference.
For every table row and opcode heading, in order, the multiset of such tokens in
the translation must equal the original's.

Usage: check_opcodes.py <original src dir> <translated src dir>
"""
from collections import Counter
from pathlib import Path
import re
import sys

# ASCII boundaries: Korean letters count as \w, so \b would miss "VX가".
L, R = r'(?<![A-Za-z0-9_])', r'(?![A-Za-z0-9_])'
TOKEN = re.compile('|'.join([
    rf'{L}0x[0-9A-FN]+{R}',               # hex literals, 0xNNN
    rf'{L}[0-9A-FXYN]{{4}}{R}',           # opcodes: 00E0, 8XY4, DXYN
    rf'{L}V[0-9A-FXY]{R}',                # registers V0-VF, VX, VY
    rf'{L}(?:I|PC|SP|DT|ST|BCD|RAM){R}',  # named registers and units
    rf'{L}[A-F]{R}',                      # single hex digits (font table)
    rf'{L}N{{1,3}}{R}',                   # N, NN, NNN operands
    rf'{L}\d+{R}',                        # decimal numbers
    r'rand\(\)',
    r'\\\|=|&=|\^=|\+=|-=|>>=|<<=|==|!=',  # compound and comparison operators
    r'(?<![=!<>+\-&^|\\])=(?!=)',          # plain assignment
    r'(?<=\s)[&+](?=\s)',                 # binary & and +
]))
OPCODE_HEADING = re.compile(r'^#{1,6}\s+[0-9A-FXYN]{4}(?![A-Za-z0-9_])')
FENCE = re.compile(r'^(```|~~~)')


def checked_lines(text):
    lines, code = [], False
    for number, line in enumerate(text.splitlines(), 1):
        if FENCE.match(line):
            code = not code
            continue
        if not code and (line.startswith('|') or OPCODE_HEADING.match(line)):
            lines.append((number, line))
    return lines


def compare(original, translated):
    problems = []
    source, target = checked_lines(original), checked_lines(translated)
    if len(source) != len(target):
        problems.append(f'{len(source)} table rows/opcode headings in original, {len(target)} in translation')
        return problems, len(source)
    for (n, a), (m, b) in zip(source, target):
        want, got = Counter(TOKEN.findall(a)), Counter(TOKEN.findall(b))
        if want != got:
            missing = ' '.join((want - got).elements()) or '-'
            extra = ' '.join((got - want).elements()) or '-'
            problems.append(f'line {n}->{m}: missing [{missing}] extra [{extra}]\n    {a.strip()}\n    {b.strip()}')
    return problems, len(source)


def main(original_root, translated_root):
    original_root, translated_root = Path(original_root), Path(translated_root)
    failures, checked = 0, 0
    for source in sorted(original_root.glob('*.md')):
        target = translated_root / source.name
        if not target.is_file():
            print(f'{source.name}: missing translation')
            failures += 1
            continue
        problems, count = compare(source.read_text(), target.read_text())
        checked += count
        for problem in problems:
            print(f'{source.name}: {problem}')
        failures += len(problems)
    print(f'Checked opcode tokens in {checked} table rows and opcode headings: {failures} problems')
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main(sys.argv[1], sys.argv[2]))
