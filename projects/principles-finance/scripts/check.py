#!/usr/bin/env python3
"""Check the translation and the built site.

    check.py skeleton UPSTREAM TREE
        Every translated CNXML document (modules/*/index.cnxml and the
        collection) has the source's element structure: the same block
        elements in order and, inside each, the same inline elements as a
        multiset (Korean word order moves them; an emphasis may be split, so
        emphasis only has to be there). Text, MathML mtext, and the translated
        attributes (media@alt, table@summary) may differ. The same comparison
        as the cnxml vocabulary's skeleton_with in crates/parser-xhtml, plus
        the same element ids as often as in the source (a duplicated term
        would otherwise pass as "emphasis present").

    check.py site SITE
        The built site: every page has one <h1>; every local link, image, and
        stylesheet points at a file that exists, and every #fragment at an id
        on that page; no id repeats on a page; no cross reference is empty.
"""

import sys
from collections import Counter
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

from lxml import etree

C = "http://cnx.rice.edu/cnxml"
MML = "http://www.w3.org/1998/Math/MathML"

# crates/parser-xhtml/src/vocab.rs, CNXML profile.
BLOCK = {
    "document", "content", "section", "para", "list", "item", "title", "caption", "figure", "subfigure",
    "table", "tgroup", "thead", "tbody", "tfoot", "row", "entry", "note", "exercise", "problem",
    "solution", "commentary", "example", "rule", "statement", "proof", "glossary", "definition",
    "meaning", "seealso", "equation", "footnote", "quote", "label", "preformat", "metadata",
    "collection", "subcollection",
}
OPAQUE = {"math", "preformat", "iframe", "object", "embed"}
TRANSLATED_ATTRS = {("media", "alt"), ("table", "summary")}


def local(e) -> str:
    return etree.QName(e).localname



def head(e) -> tuple:
    name = local(e)
    attrs = tuple(sorted(
        (k, "" if (name, k) in TRANSLATED_ATTRS else v) for k, v in e.attrib.items()))
    return (etree.QName(e).text, attrs)


def opaque(e) -> tuple:
    """A skipped element compared whole, except the text of MathML mtext."""
    if not isinstance(e.tag, str):
        return ()
    text = "" if local(e) == "mtext" else " ".join((e.text or "").split())
    return (head(e), text, tuple(opaque(c) for c in e if isinstance(c.tag, str)),
            tuple(" ".join((c.tail or "").split()) for c in e if isinstance(c.tag, str)
                  and local(e) != "mtext"))


def breaks(e) -> bool:
    if local(e) in BLOCK:
        return True
    if local(e) in OPAQUE:
        return False
    return any(isinstance(d.tag, str) and local(d) in BLOCK for d in e.iterdescendants())


def skeleton(element) -> tuple:
    blocks, inline = [], []

    def inline_of(e) -> None:
        for c in e:
            if not isinstance(c.tag, str):
                continue
            if local(c) in OPAQUE:
                (blocks if local(c) in BLOCK else inline).append(("opaque", opaque(c)))
            elif breaks(c):
                blocks.append(skeleton(c))
            else:
                inline.append(head(c))
                inline_of(c)

    inline_of(element)
    counted = sorted(h for h in inline if not (h[0] != "opaque" and is_emphasis_name(h[0])))
    present = sorted({h for h in inline if h[0] != "opaque" and is_emphasis_name(h[0])})
    return (head(element), tuple(blocks), tuple(map(repr, counted)), tuple(map(repr, present)))


def is_emphasis_name(qname: str) -> bool:
    return qname.startswith("{" + C + "}") and qname.split("}")[1] in ("emphasis", "term", "foreign", "cite-title")


def ids(root) -> Counter:
    return Counter(e.get("id") for e in root.iter() if isinstance(e.tag, str) and e.get("id"))


def check_skeleton(upstream: Path, tree: Path) -> list[str]:
    problems = []
    files = sorted(upstream.glob("modules/*/index.cnxml")) + sorted(upstream.glob("collections/*.xml"))
    for source in files:
        rel = source.relative_to(upstream)
        target = tree / rel
        try:
            translated = etree.parse(str(target)).getroot()
        except (OSError, etree.XMLSyntaxError) as error:
            problems.append(f"{rel}: {error}")
            continue
        original = etree.parse(str(source)).getroot()
        if skeleton(translated) != skeleton(original):
            problems.append(f"{rel}: element structure differs from the source")
        # Emphasis is only checked for presence above, so a duplicated term
        # would pass; every id must still appear exactly as often as before.
        if ids(translated) != ids(original):
            problems.append(f"{rel}: element ids differ from the source")
    print(f"skeleton: {len(files)} documents checked")
    return problems


class Page(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.ids = Counter()
        self.refs = []
        self.h1 = 0
        self.empty_xref = 0
        self._xref = None

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if a.get("id"):
            self.ids[a["id"]] += 1
        if tag == "h1":
            self.h1 += 1
        for key in ("href", "src"):
            if a.get(key) and not (tag == "script" and key == "src"):
                self.refs.append(a[key])
        if tag == "a" and "xref" in (a.get("class") or "").split():
            self._xref = ""

    def handle_data(self, data):
        if self._xref is not None:
            self._xref += data

    def handle_endtag(self, tag):
        if tag == "a" and self._xref is not None:
            if not self._xref.strip():
                self.empty_xref += 1
            self._xref = None


def check_site(site: Path) -> list[str]:
    problems = []
    pages = {}
    for path in sorted(site.glob("*.html")):
        p = Page()
        p.feed(path.read_text(encoding="utf-8"))
        pages[path.name] = p
    links = 0
    for name, p in pages.items():
        if p.h1 != 1:
            problems.append(f"{name}: {p.h1} <h1> elements")
        for i, n in p.ids.items():
            if n > 1:
                problems.append(f"{name}: id {i!r} repeats")
        if p.empty_xref:
            problems.append(f"{name}: {p.empty_xref} empty cross references")
        for ref in p.refs:
            parts = urlsplit(ref)
            if parts.scheme or parts.netloc or ref.startswith("mailto:"):
                continue
            links += 1
            target = unquote(parts.path) or name
            if not (site / target).is_file():
                problems.append(f"{name}: {ref} points at a missing file")
                continue
            if parts.fragment and target.endswith(".html"):
                if parts.fragment not in pages[target].ids:
                    problems.append(f"{name}: {ref} points at a missing anchor")
    print(f"site: {len(pages)} pages, {links} local links checked")
    return problems


def main() -> int:
    if len(sys.argv) < 3 or sys.argv[1] not in ("skeleton", "site"):
        print(__doc__, file=sys.stderr)
        return 2
    if sys.argv[1] == "skeleton":
        problems = check_skeleton(Path(sys.argv[2]), Path(sys.argv[3]))
    else:
        problems = check_site(Path(sys.argv[2]))
    for problem in problems:
        print(problem, file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
