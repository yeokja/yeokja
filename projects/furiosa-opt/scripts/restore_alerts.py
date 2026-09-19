"""Put GFM alert markers back on their own line in the assembled Korean book.

yeokja joins a paragraph's lines into one, so the source

    > [!NOTE]
    > Prose.

comes back as ``> [!NOTE] 문장.``. GitHub and mdBook only recognise an alert
whose marker ends its line; otherwise the page shows a plain quote that starts
with the literal text ``[!NOTE]``. Every alert in the upstream book puts the
marker on its own line, so this restores that line break (with the same quote
prefix) and changes nothing else.
"""
from pathlib import Path
import re
import sys

ALERT = re.compile(
    r'^(?P<prefix>[ \t]*(?:>[ \t]?)+)'
    r'(?P<marker>\[!(?:NOTE|TIP|IMPORTANT|WARNING|CAUTION)\])'
    r'[ \t]+(?P<prose>\S.*)$'
)
FENCE = re.compile(r'^[ \t]*(```|~~~)')


def restore_alert_lines(text):
    lines = text.split('\n')
    result = []
    fence = None
    for line in lines:
        opener = FENCE.match(line)
        if opener:
            if fence is None:
                fence = opener.group(1)
            elif opener.group(1) == fence:
                fence = None
        match = None if fence else ALERT.match(line)
        if match:
            prefix = match.group('prefix')
            result.append(prefix + match.group('marker'))
            result.append(prefix + match.group('prose'))
        else:
            result.append(line)
    return '\n'.join(result)


def write_local(path, text):
    # The assembled tree links files to upstream/ and ko/; replace the link,
    # never its target.
    temporary = path.with_name(path.name + '.preparing')
    temporary.write_text(text)
    temporary.replace(path)


def restore(root):
    for page in Path(root).rglob('*.md'):
        text = page.read_text()
        restored = restore_alert_lines(text)
        if restored != text:
            write_local(page, restored)


if __name__ == '__main__':
    restore(sys.argv[1])
