#!/usr/bin/env python3
"""Compare the Bikeshed build of the original spec with that of the translation.

Usage: check.py EN_HTML KO_HTML

Both files come out of the same Bikeshed. The translation must keep the
document's skeleton: the same element ids, the same multiset of element names
(a translation that turns `&lt;name&gt;` into a tag shows up here), the same
number of <dfn>s, the same number of headings per level, every internal
`href="#…"` resolving to an id, and no `bikeshed:` autolink destination left
behind by the yeokja bikeshed parser. The Bikeshed header (`div.head`) is left
out of the comparisons: the translation adds a warning box and a Translations
entry there on purpose.

Standard library only.
"""

from __future__ import annotations

import re
import sys
from collections import Counter
from html.parser import HTMLParser

VOID = {
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta",
    "source", "track", "wbr",
}
HEADINGS = ("h1", "h2", "h3", "h4", "h5", "h6")
HASHED = re.compile(r"^(issue|example)-[0-9a-f]{8}$")


class Skeleton(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: Counter[str] = Counter()
        self.all_ids: set[str] = set()
        self.names: Counter[str] = Counter()
        self.headings: Counter[str] = Counter()
        self.hrefs: list[str] = []
        self.dfns = 0
        self.stack: list[str] = []
        self.head_depth: int | None = None
        self.depth = 0

    def in_head(self) -> bool:
        return False

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        a = dict(attrs)
        self.depth += 1
        if a.get("id"):
            self.all_ids.add(a["id"])
        if not self.in_head():
            self.names[tag] += 1
            if a.get("id"):
                # Bikeshed names unlabeled issues and examples by a hash of
                # their text, which the translation changes; count those.
                self.ids[HASHED.sub(r"\1-*", a["id"])] += 1
            if tag == "dfn":
                self.dfns += 1
            if tag in HEADINGS:
                self.headings[tag] += 1
        href = a.get("href")
        if href:
            self.hrefs.append(href)
        if tag in VOID:
            self.depth -= 1

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self.handle_starttag(tag, attrs)
        if tag not in VOID:
            self.depth -= 1

    def handle_endtag(self, tag: str) -> None:
        if tag in VOID:
            return
        self.depth -= 1


HEAD_START = '<div class="head">'
HEAD_SEP = '<hr title="Separator for header">'


def without_header(html: str) -> str:
    """Drop Bikeshed's header block: the translation adds a warning box and a
    Translations entry there on purpose. Bikeshed omits optional end tags, so
    the block is cut by its markers rather than by tracking the tree."""
    start = html.index(HEAD_START)
    sep = html.index(HEAD_SEP, start)
    end = html.index("</div>", sep) + len("</div>")
    return html[:start] + html[end:]


def skeleton(path: str) -> Skeleton:
    p = Skeleton()
    with open(path, encoding="utf-8") as f:
        p.feed(without_header(f.read()))
    p.close()
    return p


def diff_counter(label: str, a: Counter[str], b: Counter[str], errors: list[str]) -> None:
    for key in sorted(set(a) | set(b)):
        if a[key] != b[key]:
            errors.append(f"{label} {key!r}: original {a[key]}, translation {b[key]}")


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    en = skeleton(sys.argv[1])
    ko = skeleton(sys.argv[2])
    errors: list[str] = []

    diff_counter("id", en.ids, ko.ids, errors)
    diff_counter("element", en.names, ko.names, errors)
    if en.dfns != ko.dfns:
        errors.append(f"dfn count: original {en.dfns}, translation {ko.dfns}")
    diff_counter("heading", en.headings, ko.headings, errors)

    for name, doc in (("original", en), ("translation", ko)):
        dup = [i for i, n in doc.ids.items() if n > 1 and not i.endswith("-*")]
        if dup:
            errors.append(f"{name}: duplicate ids {dup[:10]}")
        unresolved = sorted({h for h in doc.hrefs if h.startswith("#") and h[1:] not in doc.all_ids})
        if unresolved:
            errors.append(f"{name}: {len(unresolved)} internal links do not resolve: {unresolved[:10]}")
    leftovers = [h for h in ko.hrefs if h.startswith("bikeshed:")]
    if leftovers:
        errors.append(f"translation: {len(leftovers)} bikeshed: hrefs left: {leftovers[:5]}")

    if errors:
        print("check: FAILED", file=sys.stderr)
        for e in errors:
            print(f"  {e}", file=sys.stderr)
        return 1
    internal = sum(1 for h in ko.hrefs if h.startswith("#"))
    print(
        f"check: ok — {len(ko.ids)} ids, {sum(ko.names.values())} elements, {ko.dfns} dfns, "
        f"{sum(ko.headings.values())} headings, {internal} internal links resolve"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
