#!/usr/bin/env python3
"""Build the web edition from the staged Korean EPUB.

    build_site.py TREE SITE

Run after build_epub.py, which leaves the finished book in TREE/_epub and the
EPUB file next to it. The site serves the content documents as they are in
the EPUB, in spine order, each with a navigation bar and a common footer
(attribution, license, translation notice), plus a landing page.
"""

import html
import re
import shutil
import sys
from pathlib import Path

ORIGINAL = "https://wisc.pb.unizin.org/technicalpm/"
LICENSE = "https://creativecommons.org/licenses/by/4.0/"
EPUB = "technicalpm-ko.epub"

STYLE = """
body { max-width: 46rem; margin: 0 auto; padding: 0 1rem 3rem; line-height: 1.75;
  font-family: system-ui, -apple-system, "Apple SD Gothic Neo", "Noto Sans KR", sans-serif; word-break: keep-all; }
img { max-width: 100%; height: auto; }
nav.site { display: flex; flex-wrap: wrap; gap: .5rem 1.25rem; padding: .75rem 0; border-bottom: 1px solid #ccc; font-size: .95rem; }
nav.site.bottom { border-top: 1px solid #ccc; border-bottom: none; margin-top: 2rem; }
footer.site { margin-top: 2rem; font-size: .85rem; color: #555; }
.withheld-figure { border: 1px dashed #999; padding: .75rem; }
@media (prefers-color-scheme: dark) {
  body { background: #161616; color: #e6e6e6; }
  /* The EPUB style sheet sets dark text colors meant for a white page. */
  body :not(a):not(a *) { color: inherit !important; }
  a, a * { color: #8ab4ff !important; }
  footer.site { color: #aaa !important; }
  .textbox, .shaded, .textbox__header, .textbox__content { background: #222 !important; border-color: #444 !important; }
}
"""


def fail(message: str) -> None:
    sys.exit(f"build_site: {message}")


def book_title(staged: Path) -> str:
    opf = (staged / "book.opf").read_text(encoding="utf-8")
    m = re.search(r"<dc:title>(.*?)</dc:title>", opf, re.S)
    if m is None:
        fail("the OPF has no title")
    return html.unescape(m.group(1)).strip()


def spine(staged: Path) -> list[str]:
    opf = (staged / "book.opf").read_text(encoding="utf-8")
    hrefs = dict(re.findall(r'<item id="([^"]+)" href="OEBPS/([^"]+)"', opf))
    order = [hrefs[idref] for idref in re.findall(r'<itemref idref="([^"]+)"', opf)]
    if not order:
        fail("the OPF has no spine")
    return order


def title_of(document: str) -> str:
    m = re.search(r"<title>(.*?)</title>", document, re.S)
    return html.unescape(m.group(1)).split(" -- ")[0].strip() if m else ""


def nav(prev: str | None, next_: str | None, titles: dict[str, str], where: str) -> str:
    links = ['<a href="index.html">처음</a>', '<a href="table-of-contents.html">목차</a>']
    if prev:
        links.append(f'<a href="{prev}">← {html.escape(titles[prev])}</a>')
    if next_:
        links.append(f'<a href="{next_}">{html.escape(titles[next_])} →</a>')
    return f'<nav class="site {where}">' + " ".join(links) + "</nav>"


def footer(book_title: str) -> str:
    return (
        '<footer class="site"><p>'
        f'원서: Jeffrey S. Russell, Wayne P. Pferdehirt, John S. Nelson, <a href="{ORIGINAL}"><em>Technical Project '
        "Management in Living and Geometric Order</em></a>(3판), © Board of Regents of the University of Wisconsin "
        f'System, <a href="{LICENSE}">CC BY 4.0</a>. '
        f"이 페이지는 원문을 번역·수정한 비공식 한국어판 「{html.escape(book_title)}」의 일부이며 같은 CC BY 4.0으로 배포합니다. "
        '<a href="https://github.com/yeokja/yeokja">yeokja</a>와 Anthropic 사의 <code>claude-opus-5-5</code> 모델을 '
        "활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다."
        "</p></footer>"
    )


def page(document: str, top: str, bottom: str, foot: str) -> str:
    head_extra = '<meta name="viewport" content="width=device-width, initial-scale=1" />\n<link rel="stylesheet" href="site.css" type="text/css" />\n'
    if document.count("</head>") != 1 or document.count("</body>") != 1 or not re.search(r"<body[^>]*>", document):
        fail("a content document has no single head and body")
    document = document.replace("</head>", head_extra + "</head>")
    document = re.sub(r"(<body[^>]*>)", lambda m: m.group(1) + "\n" + top, document, count=1)
    return document.replace("</body>", bottom + "\n" + foot + "\n</body>")


def landing(book_title: str, first: str) -> str:
    return f"""<!DOCTYPE html>
<html lang="ko">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>{html.escape(book_title)}</title>
<link rel="stylesheet" href="site.css" type="text/css" />
</head>
<body>
<h1>{html.escape(book_title)}</h1>
<p>Jeffrey S. Russell, Wayne P. Pferdehirt, John S. Nelson 지음 · 3판의 비공식 한국어 번역</p>
<p>위스콘신 대학교 매디슨 캠퍼스가 개방형 교재로 펴낸 <a href="{ORIGINAL}"><em>Technical Project Management in Living and Geometric Order</em></a>를 한국어로 옮겼습니다. 예측 가능한 계획 중심의 일(기하학적 질서)과 불확실성 속에서 적응해야 하는 일(살아 있는 질서)을 함께 다루며 프로젝트 관리의 원칙과 실무를 설명합니다.</p>
<ul>
<li><a href="{first}">처음부터 읽기</a></li>
<li><a href="table-of-contents.html">목차</a></li>
<li><a href="{EPUB}">EPUB 내려받기</a></li>
</ul>
{footer(book_title)}
</body>
</html>
"""


def main() -> None:
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    tree, site = Path(sys.argv[1]), Path(sys.argv[2])
    staged = tree / "_epub"
    epub = tree / EPUB
    if not staged.is_dir() or not epub.is_file():
        fail("run build_epub.py first")
    shutil.rmtree(site, ignore_errors=True)
    shutil.copytree(staged / "OEBPS", site)
    shutil.copyfile(epub, site / EPUB)
    (site / "site.css").write_text(STYLE.lstrip(), encoding="utf-8")
    order = spine(staged)
    docs = {name: (site / name).read_text(encoding="utf-8") for name in order}
    titles = {name: title_of(text) for name, text in docs.items()}
    title = book_title(staged)
    foot = footer(title)
    for i, name in enumerate(order):
        prev = order[i - 1] if i > 0 else None
        next_ = order[i + 1] if i + 1 < len(order) else None
        top, bottom = nav(prev, next_, titles, "top"), nav(prev, next_, titles, "bottom")
        (site / name).write_text(page(docs[name], top, bottom, foot), encoding="utf-8")
    first = next(name for name in order if name.startswith("front-matter"))
    (site / "index.html").write_text(landing(title, first), encoding="utf-8")
    print(f"build_site: wrote {site} ({len(order)} pages)")


if __name__ == "__main__":
    main()
