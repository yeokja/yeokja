#!/usr/bin/env python3
"""Check the translation against the source EPUB.

    check.py skeleton UPSTREAM_OEBPS KO_OEBPS
        Every translated content document has the source's element structure:
        the same block elements in order and, inside each, the same inline
        elements (in any order — Korean word order moves them; an emphasis may
        be split). Only text and the alt/title values differ.

    check.py epub UPSTREAM_TREE KO_EPUB
        The built EPUB: table of contents and NCX labels match the chapter and
        section headings they point at, every TOC link resolves, and the
        language is Korean.
"""

import html.entities
import re
import sys
import xml.etree.ElementTree as ET
import zipfile
from pathlib import Path

TRANSLATED_ATTRS = {"alt", "title"}
XHTML = "{http://www.w3.org/1999/xhtml}"


def parse(text: str) -> ET.Element:
    # The DTD's named entities, as character references ElementTree can read.
    def entity(m: re.Match) -> str:
        name = m.group(1)
        if name in ("lt", "gt", "amp", "quot", "apos"):
            return m.group(0)
        return f"&#{html.entities.name2codepoint[name]};"

    text = re.sub(r"&([A-Za-z][A-Za-z0-9]*);", entity, text)
    text = re.sub(r"<!DOCTYPE[^>]*>", "", text)
    return ET.fromstring(text.encode("utf-8"))


# Elements that end a run of inline content (crates/parser-xhtml BLOCK).
BLOCK = {
    "address", "article", "aside", "blockquote", "body", "caption", "center", "dd", "details", "dialog",
    "dir", "div", "dl", "dt", "fieldset", "figcaption", "figure", "footer", "form", "h1", "h2", "h3",
    "h4", "h5", "h6", "head", "header", "hgroup", "hr", "html", "legend", "li", "main", "menu", "nav",
    "noscript", "ol", "p", "pre", "section", "summary", "table", "tbody", "td", "tfoot", "th", "thead",
    "tr", "ul",
}
# Emphasis Korean word order may split in two.
EMPHASIS = {"em", "i", "cite", "dfn", "var", "strong", "b"}


def skeleton(element: ET.Element) -> tuple:
    """Block elements in order; the inline elements inside each block as a
    multiset, since translation moves them with the words they wrap (and may
    split an emphasis, so emphasis only has to be there)."""

    def name(e: ET.Element) -> str:
        return e.tag.removeprefix(XHTML)

    def head(e: ET.Element) -> tuple:
        return (name(e), tuple(sorted((k, "" if k in TRANSLATED_ATTRS else v) for k, v in e.attrib.items())))

    blocks, inline = [], []

    def inline_of(e: ET.Element) -> None:
        for child in e:
            if name(child) in BLOCK:
                blocks.append(skeleton(child))
            else:
                inline.append(head(child))
                inline_of(child)

    inline_of(element)
    counted = sorted(h for h in inline if h[0] not in EMPHASIS)
    present = sorted({h for h in inline if h[0] in EMPHASIS})
    return (head(element), tuple(blocks), tuple(counted), tuple(present))


def plain(element: ET.Element) -> str:
    return " ".join("".join(element.itertext()).split())


def check_skeleton(upstream: Path, ko: Path) -> list[str]:
    problems = []
    files = sorted(ko.glob("*.html"))
    for path in files:
        source = upstream / path.name
        try:
            translated = parse(path.read_text(encoding="utf-8"))
        except ET.ParseError as error:
            problems.append(f"{path.name}: not well-formed: {error}")
            continue
        if skeleton(translated) != skeleton(parse(source.read_text(encoding="utf-8"))):
            problems.append(f"{path.name}: element structure differs from the source")
    print(f"skeleton: {len(files)} documents checked")
    return problems


def headings(root: ET.Element) -> tuple[dict[str, str], str | None]:
    """Heading text by id, and the document's title heading."""
    by_id, title = {}, None
    for element in root.iter():
        tag = element.tag.removeprefix(XHTML)
        if re.fullmatch(r"h[1-6]", tag):
            if element.get("id"):
                by_id[element.get("id")] = plain(element)
            classes = (element.get("class") or "").split()
            if title is None and {"chapter-title", "front-matter-title", "back-matter-title"} & set(classes):
                title = plain(element)
    return by_id, title


def check_epub(upstream: Path, epub: Path) -> list[str]:
    problems = []
    with zipfile.ZipFile(epub) as z:
        names = z.namelist()
        if names[0] != "mimetype" or z.getinfo("mimetype").compress_type != zipfile.ZIP_STORED:
            problems.append("mimetype is not the first, uncompressed entry")
        docs = {n.removeprefix("OEBPS/"): parse(z.read(n).decode("utf-8")) for n in names if n.endswith(".html")}
        toc = docs["table-of-contents.html"]
        links = 0
        for a in toc.iter(f"{XHTML}a"):
            href = a.get("href", "")
            target, _, anchor = href.partition("#")
            span = next((s for s in a if "toc-" in (s.get("class") or "")), None)
            if span is None or target not in docs:
                continue
            links += 1
            by_id, title = headings(docs[target])
            expected = by_id.get(anchor) if anchor else title
            # A chapter entry adds the chapter number; a section heading
            # carries its own ("1.1 …").
            shown = plain(span) if anchor else re.sub(r"^\d+\.\s*", "", plain(span))
            if expected is None:
                problems.append(f"TOC link {href} has no heading to point at")
            elif shown != expected:
                problems.append(f"TOC entry for {href} does not match its heading")
        ncx = ET.fromstring(z.read("toc.ncx"))
        ns = "{http://www.daisy.org/z3986/2005/ncx/}"
        for point in ncx.iter(f"{ns}navPoint"):
            label = plain(point.find(f"{ns}navLabel"))
            src = point.find(f"{ns}content").get("src").removeprefix("OEBPS/")
            _, title = headings(docs[src.partition("#")[0]])
            if title is not None and re.sub(r"^\d+\.\s*", "", label) != title:
                problems.append(f"NCX label for {src} does not match its title")
            if re.search(r"[A-Za-z]{4,}", label) and title is None:
                problems.append(f"NCX label for {src} is still English")
        opf = z.read("book.opf").decode("utf-8")
        if "<dc:language>ko</dc:language>" not in opf:
            problems.append("OPF language is not ko")
        for name, root in docs.items():
            if root.get("{http://www.w3.org/XML/1998/namespace}lang") != "ko":
                problems.append(f"{name}: xml:lang is not ko")
    print(f"epub: {len(docs)} documents, {links} TOC links checked")
    return problems


def main() -> int:
    if len(sys.argv) != 4 or sys.argv[1] not in ("skeleton", "epub"):
        print(__doc__, file=sys.stderr)
        return 2
    mode, a, b = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
    problems = check_skeleton(a, b) if mode == "skeleton" else check_epub(a, b)
    for problem in problems:
        print(problem, file=sys.stderr)
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
