#!/usr/bin/env python3
"""Build the Korean EPUB from the assembled tree.

    build_epub.py TREE OUT.epub

TREE is `build/tree` (upstream with the `ko/` overlay). The book is staged in
TREE/_epub as real files (the tree's files may be links into upstream/), then:

1. the table of contents and the NCX take their entries from the translated
   chapter and section headings;
2. the language becomes Korean and the OPF names the translation;
3. the copyright page gets the translation notice;
4. the one figure printed by permission of its rights holder is left out;
5. everything is zipped with `mimetype` first and uncompressed.

See docs/superpowers/specs/2026-10-22-technicalpm-korean-translation-design.md.
"""

import html
import re
import shutil
import sys
import zipfile
from pathlib import Path

ORIGINAL = "https://wisc.pb.unizin.org/technicalpm/"
IDENTIFIER = "https://github.com/yeokja/yeokja/tree/main/projects/technicalpm"
LICENSE = "https://creativecommons.org/licenses/by/4.0/"
MODEL = "Anthropic 사의 <code>claude-opus-5-5</code>"

# Labels of the documents that have no heading of their own.
UI = {"Cover": "표지", "Title Page": "속표지", "Copyright": "판권", "Table Of Contents": "목차"}

# Printed in the original "used with permission" of The Minerals, Metals &
# Materials Society; that permission does not reach this translation.
WITHHELD_DOC = "chapter-001-foundations-principles-concepts.html"
WITHHELD_FIGURE = "attachment_343"
WITHHELD_IMAGE = "assets/Figure-1-3.png"

NOTICE = f"""<div class="translation-notice">
<h2 class="section-header">한국어판에 대하여</h2>
<p class="nonindent">이 책은 Jeffrey S. Russell, Wayne P. Pferdehirt, John S. Nelson의 <a href="{ORIGINAL}"><em>Technical Project Management in Living and Geometric Order</em></a>(3판, Board of Regents of the University of Wisconsin System)을 한국어로 옮긴 비공식 번역본입니다. 원서와 이 번역본은 모두 <a href="{LICENSE}">크리에이티브 커먼즈 저작자표시 4.0 국제 라이선스(CC BY 4.0)</a>로 배포됩니다.</p>
<p class="nonindent">번역하면서 원문을 고쳤습니다. 목차와 장 제목은 번역에 맞춰 다시 만들었고, 원저작권자의 허락을 받아 원서에만 실린 1장의 그림 한 점은 싣지 않았습니다. 그림 속 영어는 번역하지 않았습니다.</p>
<p class="nonindent"><a href="https://github.com/yeokja/yeokja">yeokja</a>와 {MODEL} 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.</p>
</div>
"""

PLACEHOLDER = f"""<p class="nonindent withheld-figure">[그림 생략] 이 그림은 저작권자(The Minerals, Metals &amp; Materials Society)의 허락을 받아 원서에 실린 것이라 번역본에는 싣지 않았습니다. <a href="{ORIGINAL}">원서</a> 1장에서 볼 수 있습니다.</p>"""


def fail(message: str) -> None:
    sys.exit(f"build_epub: {message}")


def plain(fragment: str) -> str:
    return " ".join(html.unescape(re.sub(r"<[^>]+>", "", fragment)).split())


def heading_inner(document: str, element: str) -> str | None:
    """The inner markup of the first `element` (a regex for a start tag)."""
    m = re.search(rf"<(h[1-6])\b{element}[^>]*>(.*?)</\1>", document, re.S)
    return m.group(2).strip() if m else None


def doc_title(document: str) -> str | None:
    for cls in ("chapter-title", "front-matter-title", "back-matter-title", "title"):
        inner = heading_inner(document, rf'[^>]*\bclass="{cls}"')
        if inner is not None:
            return inner
    return None


def stage(tree: Path) -> Path:
    staged = tree / "_epub"
    shutil.rmtree(staged, ignore_errors=True)
    staged.mkdir()
    for name in ("mimetype", "book.opf", "toc.ncx"):
        shutil.copyfile(tree / name, staged / name)
    for name in ("META-INF", "OEBPS"):
        shutil.copytree(tree / name, staged / name)
    return staged


def rewrite_toc(oebps: Path, docs: dict[str, str]) -> None:
    path = oebps / "table-of-contents.html"
    toc = path.read_text(encoding="utf-8")

    def entry(m: re.Match) -> str:
        target, anchor, kind, old = m.group(2), m.group(3), m.group(4), m.group(5)
        if target not in docs:
            fail(f"TOC points at {target}, which is not in the book")
        if anchor:
            new = heading_inner(docs[target], rf'[^>]*\bid="{re.escape(anchor)}"')
        else:
            title = doc_title(docs[target])
            prefix = re.match(r"\s*\d+\.\s*", old)
            new = None if title is None else (prefix.group(0) if prefix else "") + title
        if new is None:
            fail(f"no heading for TOC entry {target}#{anchor}")
        return f'{m.group(1)}<span class="toc-{kind}-title">{new}</span>'

    toc, count = re.subn(
        r'(<a href="([^"#]+)(?:#([^"]*))?">)<span class="toc-(chapter|subsection)-title">(.*?)</span>',
        entry,
        toc,
        flags=re.S,
    )
    if count == 0:
        fail("no TOC entries found")
    toc = re.sub(r"(<h1[^>]*>)\s*Contents\s*(</h1>)", r"\1목차\2", toc)
    path.write_text(toc, encoding="utf-8")


def set_head_titles(oebps: Path, title: str) -> None:
    # Document titles that are labels, kept the same as the NCX's (UI).
    for name, label in (("table-of-contents.html", "목차"), ("front-cover.html", "표지"), ("title-page.html", "속표지")):
        path = oebps / name
        text = re.sub(r"<title>.*?</title>", f"<title>{label} -- {html.escape(title, quote=False)}</title>", path.read_text(encoding="utf-8"), count=1, flags=re.S)
        path.write_text(text, encoding="utf-8")


def set_language(oebps: Path) -> None:
    for path in oebps.glob("*.html"):
        text = path.read_text(encoding="utf-8")
        text = text.replace('xml:lang="en"', 'xml:lang="ko"').replace(' lang="en"', ' lang="ko"')
        path.write_text(text, encoding="utf-8")


def rewrite_ncx(staged: Path, docs: dict[str, str], title: str) -> None:
    path = staged / "toc.ncx"
    ncx = path.read_text(encoding="utf-8")

    def point(m: re.Match) -> str:
        label, src = m.group(2), m.group(4)
        target = src.removeprefix("OEBPS/").partition("#")[0]
        if label in UI:
            new = UI[label]
        else:
            title = doc_title(docs.get(target, ""))
            if title is None:
                fail(f"NCX entry {src} has no title to take")
            prefix = re.match(r"\s*\d+\.\s*", label)
            new = (prefix.group(0) if prefix else "") + plain(title)
        return f"{m.group(1)}{html.escape(new, quote=False)}{m.group(3)}{src}"

    ncx, count = re.subn(r"(<navLabel>\s*<text>)([^<]*)(</text>\s*</navLabel>\s*<content src=\")([^\"]*)", point, ncx)
    if count == 0:
        fail("no NCX entries found")
    ncx = re.sub(r"(<docTitle>\s*<text>)[^<]*(</text>)", rf"\g<1>{html.escape(title, quote=False)}\g<2>", ncx, count=1)
    ncx = re.sub(r'(<meta name="dtb:uid" content=")[^"]*', rf"\g<1>{IDENTIFIER}", ncx)
    path.write_text(ncx, encoding="utf-8")


def rewrite_opf(staged: Path, title: str) -> None:
    path = staged / "book.opf"
    opf = path.read_text(encoding="utf-8")
    replacements = [
        (r"<dc:title>[^<]*</dc:title>", f"<dc:title>{html.escape(title, quote=False)}</dc:title>"),
        (r"<dc:language>en</dc:language>", f"<dc:language>ko</dc:language>\n<dc:source>{ORIGINAL}</dc:source>"),
        (r'(<dc:identifier id="PrimaryID"[^>]*>)[^<]*', rf"\g<1>{IDENTIFIER}"),
    ]
    for pattern, replacement in replacements:
        opf, count = re.subn(pattern, replacement, opf, count=1)
        if count != 1:
            fail(f"OPF has no {pattern}")
    opf, count = re.subn(rf'\s*<item [^>]*href="OEBPS/{re.escape(WITHHELD_IMAGE)}"[^>]*/>', "", opf)
    if count != 1:
        fail(f"OPF manifest has no {WITHHELD_IMAGE}")
    path.write_text(opf, encoding="utf-8")


def add_notice(oebps: Path) -> None:
    path = oebps / "copyright.html"
    text = path.read_text(encoding="utf-8")
    if text.count("</body>") != 1:
        fail("copyright page has no single </body>")
    path.write_text(text.replace("</body>", NOTICE + "</body>"), encoding="utf-8")


def withhold_figure(oebps: Path) -> None:
    path = oebps / WITHHELD_DOC
    text = path.read_text(encoding="utf-8")
    start = text.find(f'id="{WITHHELD_FIGURE}"')
    if start < 0:
        fail(f"{WITHHELD_FIGURE} not found")
    pattern = re.compile(r'<a>\s*<img [^>]*src="' + re.escape(WITHHELD_IMAGE) + r'"[^>]*/>\s*</a>', re.S)
    m = pattern.search(text, start)
    if m is None or m.start() - start > 400:
        fail(f"the image of {WITHHELD_FIGURE} is not where it was")
    path.write_text(text[: m.start()] + PLACEHOLDER + text[m.end() :], encoding="utf-8")
    for other in oebps.glob("*.html"):
        if WITHHELD_IMAGE in other.read_text(encoding="utf-8"):
            fail(f"{other.name} still uses {WITHHELD_IMAGE}")
    (oebps / WITHHELD_IMAGE).unlink()


def pack(staged: Path, out: Path) -> None:
    out.unlink(missing_ok=True)
    with zipfile.ZipFile(out, "w") as z:
        z.write(staged / "mimetype", "mimetype", compress_type=zipfile.ZIP_STORED)
        for path in sorted(staged.rglob("*")):
            name = path.relative_to(staged).as_posix()
            if path.is_file() and name != "mimetype":
                z.write(path, name, compress_type=zipfile.ZIP_DEFLATED)


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    tree, out = Path(sys.argv[1]), Path(sys.argv[2])
    staged = stage(tree)
    oebps = staged / "OEBPS"
    docs = {p.name: p.read_text(encoding="utf-8") for p in oebps.glob("*.html")}
    title = doc_title(docs["title-page.html"])
    if title is None:
        fail("the title page has no title")
    title = plain(title)
    rewrite_toc(oebps, docs)
    rewrite_ncx(staged, docs, title)
    set_head_titles(oebps, title)
    set_language(oebps)
    rewrite_opf(staged, title)
    add_notice(oebps)
    withhold_figure(oebps)
    pack(staged, out)
    print(f"build_epub: wrote {out}")


if __name__ == "__main__":
    main()
