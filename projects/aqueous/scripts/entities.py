#!/usr/bin/env python3
"""Keep HTML character references that the original spelled out.

Usage: entities.py check|fix [STATE_FILE]

The inline-tag codec shows the model `&lt;`, `&gt;` and `&amp;` as the
characters they stand for, and a translation that writes `<<` or `<name>`
back as bare characters is read by Bikeshed as a CSS production (`<<foo>>`)
or an HTML tag. The original uses such references on a handful of lines
(`|a| &lt;&lt; |b|`, `&lt;public+aqr@mearie.org&gt;`). For every segment
whose source contains a character reference, this script re-escapes the
bare `<`, `>` and `&` in the translation, leaving alone the inline HTML tags
that also occur in the source (`<sup>`, `</sup>`, `<dfn …>`) and references
already written. `check` reports and exits 1; `fix` rewrites the state file.
Run once after `yeokja translate`, then run `yeokja translate` again so
that ko/ is reconstructed from the fixed state (no model call is made); the
build's element comparison and Bikeshed's warning would otherwise reject the
output.

Standard library only.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

DEFAULT_STATE = Path(__file__).resolve().parent.parent / "state" / "upstream" / "spec" / "index.bs.yeokja.json"
TAG = re.compile(r"</?[A-Za-z][^<>]*>")
REFERENCE = re.compile(r"&(?:[A-Za-z]+|#[0-9]+|#x[0-9A-Fa-f]+);")


def escape_translation(source: str, translation: str) -> str:
    tags = set(TAG.findall(source))
    out: list[str] = []
    i = 0
    while i < len(translation):
        c = translation[i]
        if c == "\\" and i + 1 < len(translation) and translation[i + 1] in "<>&":
            # A Markdown escape the codec wrote for the bare character: the
            # reference replaces both the backslash and the character.
            i += 1
            c = translation[i]
        if c == "<":
            m = TAG.match(translation, i)
            if m and m.group(0) in tags:
                out.append(m.group(0))
                i = m.end()
                continue
            out.append("&lt;")
        elif c == ">":
            out.append("&gt;")
        elif c == "&":
            m = REFERENCE.match(translation, i)
            if m:
                out.append(m.group(0))
                i = m.end()
                continue
            out.append("&amp;")
        else:
            out.append(c)
        i += 1
    return "".join(out)


def main() -> int:
    if len(sys.argv) < 2 or sys.argv[1] not in ("check", "fix"):
        print(__doc__, file=sys.stderr)
        return 2
    mode = sys.argv[1]
    path = Path(sys.argv[2]) if len(sys.argv) > 2 else DEFAULT_STATE
    data = json.loads(path.read_text(encoding="utf-8"))
    segments = data["segments"] if isinstance(data, dict) else data
    changed = 0
    for seg in segments:
        source = seg.get("source") or ""
        translation = seg.get("translation")
        if not translation or not REFERENCE.search(source):
            continue
        fixed = escape_translation(source, translation)
        if fixed != translation:
            changed += 1
            print(f"{seg.get('id', '?')}:\n  - {translation}\n  + {fixed}")
            if mode == "fix":
                seg["translation"] = fixed
    if mode == "fix" and changed:
        path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"entities: {changed} translation(s) {'fixed' if mode == 'fix' else 'to fix'}")
    return 1 if (mode == "check" and changed) else 0


if __name__ == "__main__":
    sys.exit(main())
