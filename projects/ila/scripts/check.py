#!/usr/bin/env python3
"""Check the translation and the built site.

    check.py skeleton UPSTREAM_SRC KO_SRC
        Every translated PreTeXt file has the source's element structure, as
        the pretext parser reads it (docs/superpowers/specs/
        2026-10-25-xml-vocabulary-profiles-design.md, 2.4 and 2.7): block
        elements in order and nesting; inside each block, its inline elements
        as a multiset (emphasis only has to be present, since Korean word
        order may split it, and so do inline formulas, which Korean repeats
        where English uses a pronoun); attributes equal except the translated
        ``type-name``. Elements the parser does not translate (formulas,
        TikZ, demos, index entries ...) must be byte for byte the source's.

    check.py site SITE_DIR
        The built site: every relative link, knowl and image resolves to a
        file, every ``#fragment`` to an id in its target (except the
        in-context and index-letter anchors the book's stylesheet leaves
        dangling in the English build too), no LaTeX placeholder
        is left unrendered, and every page is in Korean (lang="ko-KR").
"""

import re
import sys
from collections import Counter
import xml.etree.ElementTree as ET
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

XML = "{http://www.w3.org/XML/1998/namespace}"
XI = "{http://www.w3.org/2001/XInclude}"

# crates/parser-xhtml/src/vocab.rs, Vocabulary::Pretext.
INLINE = {
    "em", "term", "alert", "q", "sq", "dq", "c", "m", "xref", "url", "fn", "sup", "sub", "foreign",
    "pubtitle", "articletitle", "booktitle", "abbr", "acro", "init", "email", "var", "taxon",
    "personname", "quantity", "kbd", "tag", "tage", "attr", "insert", "delete", "stale", "icon", "fillin",
    "rsq", "lsq", "ldq", "rdq", "ndash", "mdash", "nbsp", "ellipsis", "today", "trademark", "registered",
    "copyright", "less", "greater", "lt", "gt", "restrict-version",
}
UNTRANSLATED = {
    "m", "c", "me", "men", "md", "mdn", "latex-code", "latex-image-code", "mathbox", "image", "macros",
    "docinfo", "idx", "usage", "pre", "cd", "program", "sage", "console", "input", "output", "sageplot",
    "asymptote", "video", "interactive",
}
EMPHASIS = {"em", "foreign", "pubtitle", "articletitle", "booktitle", "term", "alert"}
# Inline formulas a Korean sentence may repeat where English used a pronoun
# ("A의 열 ... A의 행"): each must be the source's, but only presence counts.
PRESENT = EMPHASIS | {"m", "c"}
TRANSLATED_ATTRS = {"type-name"}


def name(e: ET.Element) -> str:
    return e.tag.replace(XI, "xi:")


def is_block(e: ET.Element) -> bool:
    n = name(e)
    if n in UNTRANSLATED:
        return False
    if n not in INLINE and (len(e) or (e.text or "").strip()):
        return True
    return any(is_block(c) for c in e)


def head(e: ET.Element) -> tuple:
    attrs = tuple(sorted((k, "" if k in TRANSLATED_ATTRS else v) for k, v in e.attrib.items()))
    if name(e) in UNTRANSLATED:
        inner = (e.text or "") + "".join(ET.tostring(c, encoding="unicode") for c in e)
        return (name(e), attrs, inner)
    return (name(e), attrs)


def skeleton(e: ET.Element) -> tuple:
    blocks, inline = [], []

    def walk(parent: ET.Element) -> None:
        for child in parent:
            if is_block(child):
                blocks.append(skeleton(child))
            else:
                inline.append(head(child))
                if name(child) not in UNTRANSLATED:
                    walk(child)

    walk(e)
    counted = sorted(h for h in inline if h[0] not in PRESENT)
    present = sorted({h for h in inline if h[0] in PRESENT})
    return (head(e), tuple(blocks), tuple(counted), tuple(present))


def first_difference(a: tuple, b: tuple, path: str = "") -> str:
    if a[0] != b[0]:
        return f"{path}/{a[0][0]}: element or attributes differ"
    if len(a[1]) != len(b[1]):
        return f"{path}/{a[0][0]}: {len(a[1])} blocks in the source, {len(b[1])} in the translation"
    for x, y in zip(a[1], b[1]):
        if x != y:
            return first_difference(x, y, f"{path}/{a[0][0]}")
    if a[2] != b[2]:
        diff = (Counter(a[2]) - Counter(b[2])) + (Counter(b[2]) - Counter(a[2]))
        sample = str(next(iter(diff)))[:200]
        return f"{path}/{a[0][0]}: inline elements differ, e.g. {sample}"
    return f"{path}/{a[0][0]}: emphasis or inline formulas differ: {sorted(set(a[3]) ^ set(b[3]))[:3]}"


def check_skeleton(upstream: Path, ko: Path) -> list[str]:
    problems = []
    files = sorted(ko.glob("*.xml"))
    for path in files:
        try:
            translated = ET.parse(path).getroot()
        except ET.ParseError as error:
            problems.append(f"{path.name}: not well-formed: {error}")
            continue
        a, b = skeleton(ET.parse(upstream / path.name).getroot()), skeleton(translated)
        if a != b:
            problems.append(f"{path.name}: {first_difference(a, b)}")
    print(f"skeleton: {len(files)} files checked")
    return problems


class Page(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.ids, self.refs, self.lang = set(), [], None
        self.incontext = 0

    def handle_endtag(self, tag):
        if tag == "span" and self.incontext:
            self.incontext -= 1

    def handle_starttag(self, tag, attrs):
        a = dict(attrs)
        if tag == "span" and (self.incontext or "incontext" in (a.get("class") or "").split()):
            self.incontext += 1
        if tag == "html":
            self.lang = a.get("lang")
        for key in ("id", "name") if tag == "a" else ("id",):
            if a.get(key):
                self.ids.add(a[key])
        for key in ("href", "src", "knowl"):
            if a.get(key) is not None and not (tag == "link" and a.get("rel") == "manifest" and key != "href"):
                self.refs.append((tag, key, a[key].strip(), bool(self.incontext)))


def check_site(site: Path) -> list[str]:
    problems = []
    pages = {p.relative_to(site).as_posix(): p for p in site.rglob("*.html")}
    parsed = {}
    for rel, path in pages.items():
        text = path.read_text(encoding="utf-8")
        if "text/x-latex-" in text:
            problems.append(f"{rel}: unrendered LaTeX placeholder")
        page = Page()
        page.feed(text)
        parsed[rel] = page
        if "/" not in rel and page.lang != "ko-KR":
            problems.append(f"{rel}: lang is {page.lang!r}, not ko-KR")
    links, inherited = 0, 0
    for rel, page in parsed.items():
        for tag, key, ref, incontext in page.refs:
            if not ref or ref.startswith(("http://", "https://", "mailto:", "javascript:", "data:")):
                continue
            url = urlsplit(ref)
            # Knowls are loaded into the page that shows them, so every
            # page's links (knowls' too) are relative to the site root.
            target = unquote(url.path).removeprefix("./") or rel
            if not (site / target).exists():
                problems.append(f"{rel}: {key}={ref!r} points to a missing file")
                continue
            links += 1
            if url.fragment and target in parsed and url.fragment not in parsed[target].ids:
                # The book's stylesheet gives no id to elements born hidden
                # in a knowl, yet links to them "in context", and its index
                # jump bar lists every letter: the English build has the same
                # dangling anchors. The skeleton check keeps them the same.
                if incontext or url.fragment.startswith("indexletter-"):
                    inherited += 1
                    continue
                problems.append(f"{rel}: {key}={ref!r} points to a missing anchor")
    print(f"site: {len(pages)} pages, {links} links/knowls/images checked "
          f"({inherited} in-context and index-letter anchors the original stylesheet leaves dangling)")
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
    if problems:
        print(f"{len(problems)} problems", file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
