#!/usr/bin/env python3
"""Render the translated CNXML book as a static Korean website.

    build_site.py TREE OUT UPSTREAM

TREE is the assembled tree (upstream with ko/ laid over it): the collection
document and modules/mNNNNN/index.cnxml, translated, plus media/. UPSTREAM is
the untranslated source; only its English page titles are read, to rebuild the
openstax.org page addresses the footer and exercise boxes link to.

The renderer stands in for OpenStax's own build (enki): it numbers chapters,
sections, figures and tables, fills empty cross references ("그림 1.2"),
labels notes, collates the end-of-section material (summary, key terms, CFA
note, exercises, videos) onto per-chapter pages, writes footnotes, emits
MathML as is (MathJax renders it where the browser cannot), and replaces
exercises served by the OpenStax exercise service with a link to the original
page. It fails on a cross reference it cannot resolve or an element it does
not know.
"""

import html
import re
import shutil
import sys
import textwrap
import unicodedata
from pathlib import Path

from lxml import etree

C = "http://cnx.rice.edu/cnxml"
COL = "http://cnx.rice.edu/collxml"
MD = "http://cnx.rice.edu/mdml"
MML = "http://www.w3.org/1998/Math/MathML"
COLLECTION = "collections/principles-finance-2e.collection.xml"
BOOK_PAGES = "https://openstax.org/books/principles-finance-2e/pages/"
BOOK_DETAILS = "https://openstax.org/details/books/principles-finance-2e"
LICENSE_URL = "https://creativecommons.org/licenses/by-nc-sa/4.0/deed.ko"
MATHJAX = "https://cdn.jsdelivr.net/npm/mathjax@3.2.2/es5/mml-chtml.js"

NOTE_LABELS = {
    "concepts-practice": "개념 적용",
    "excel-spreadsheet": "엑셀 활용",
    "link-to-learning": "더 알아보기",
    "think-through": "생각해 보기",
}

# End-of-chapter pages, in the original's order: (section class, page slug,
# heading). Key terms come from the modules' glossary elements.
COLLATED = [
    ("section-summary", "summary", "요약"),
    (None, "key-terms", "핵심 용어"),
    ("cfa-institute", "cfa-institute", "CFA Institute"),
    ("multiple-choice", "multiple-choice", "객관식 문제"),
    ("review-questions", "review-questions", "복습 문제"),
    ("problem-set", "problems", "연습 문제"),
    ("video-activity", "video-activity", "동영상 활동"),
]
COLLATED_CLASSES = {c for c, _, _ in COLLATED if c}

BLOCKISH = {"list", "table", "figure", "equation", "note", "exercise", "quote", "preformat"}
LIST_STYLES = {"lower-alpha": "a", "upper-alpha": "A", "lower-roman": "i", "upper-roman": "I"}


def local(e) -> str:
    return etree.QName(e).localname


def ns(e) -> str:
    return etree.QName(e).namespace or ""


def is_c(e, *names) -> bool:
    return isinstance(e.tag, str) and ns(e) == C and local(e) in names


def esc(text) -> str:
    return html.escape(text or "", quote=False)


def attr(text) -> str:
    return html.escape(text or "", quote=True)


def plain(e) -> str:
    return " ".join("".join(t for t in e.itertext()).split()) if e is not None else ""


def slug(title: str) -> str:
    t = unicodedata.normalize("NFKD", title).encode("ascii", "ignore").decode().lower()
    t = re.sub(r"['’]", "", t)
    return re.sub(r"[^a-z0-9]+", "-", t).strip("-")


def child(e, name):
    return e.find(f"{{{C}}}{name}")


def has_content(e) -> bool:
    return e is not None and (len(e) > 0 or bool((e.text or "").strip()))


def is_embed(exercise) -> bool:
    return exercise.find(f".//{{{C}}}link[@class='os-embed']") is not None


def collated_class(e):
    """The collated section class of the section an element sits in, if any."""
    while e is not None:
        if is_c(e, "section") and e.get("class") in COLLATED_CLASSES:
            return e.get("class")
        e = e.getparent()
    return None


class Page:
    def __init__(self, file, title, heading, origin, kind, chapter=None):
        self.file = file
        self.title = title  # plain text, for <title> and navigation
        self.heading = heading  # HTML for <h1>
        self.origin = origin  # openstax.org page
        self.kind = kind
        self.chapter = chapter
        self.body = ""
        self.math = False


class Book:
    def __init__(self, tree: Path, upstream: Path):
        self.tree = tree
        collection = etree.parse(str(tree / COLLECTION)).getroot()
        self.title = plain(collection.find(f"{{{COL}}}metadata/{{{MD}}}title"))
        self.docs, self.english = {}, {}
        content = collection.find(f"{{{COL}}}content")

        def load(m):
            self.docs[m] = etree.parse(str(tree / "modules" / m / "index.cnxml")).getroot()
            self.english[m] = etree.parse(str(upstream / "modules" / m / "index.cnxml")).getroot()

        self.preface = None
        self.chapters = []  # (number, title, intro module, [section modules])
        for item in content:
            if not isinstance(item.tag, str):
                continue
            if local(item) == "module":
                self.preface = item.get("document")
                load(self.preface)
            elif local(item) == "subcollection":
                title = plain(item.find(f"{{{MD}}}title"))
                intro, sections = None, []
                for mod in item.iter(f"{{{COL}}}module"):
                    m = mod.get("document")
                    load(m)
                    if self.docs[m].get("class") == "introduction":
                        intro = m
                    else:
                        sections.append(m)
                self.chapters.append((len(self.chapters) + 1, title, intro, sections))

        self.pages = []
        self.module_page = {}
        self.section_number = {}  # module -> "n.k"
        self.end_pages = {}  # (chapter, slug) -> Page
        self.pages_by_chapter = {}
        self.build_pages()
        self.ids = {}  # (module, id) -> (page file, anchor)
        self.labels = {}  # (module, id) -> cross reference text
        self.numbered = {"figure": 0, "table": 0}
        self.index_targets()

    def doc_title(self, m) -> str:
        return plain(child(self.docs[m], "title"))

    def english_title(self, m) -> str:
        return plain(child(self.english[m], "title"))

    def build_pages(self):
        if self.preface:
            p = Page("preface.html", self.doc_title(self.preface), esc(self.doc_title(self.preface)),
                     BOOK_PAGES + "preface", "preface")
            p.module = self.preface
            self.pages.append(p)
            self.module_page[self.preface] = p
        for n, title, intro, sections in self.chapters:
            chapter_pages = []
            p = Page(f"{n}-intro.html", f"{n}장 {title}", f"<span class=\"num\">{n}장</span> {esc(title)}",
                     BOOK_PAGES + f"{n}-{slug(self.english_title(intro))}", "intro", n)
            p.module = intro
            chapter_pages.append(p)
            self.module_page[intro] = p
            for k, m in enumerate(sections, 1):
                num = f"{n}.{k}"
                self.section_number[m] = num
                t = self.doc_title(m)
                p = Page(f"{n}-{k}.html", f"{num} {t}", f"<span class=\"num\">{num}</span> {esc(t)}",
                         BOOK_PAGES + f"{n}-{k}-{slug(self.english_title(m))}", "section", n)
                p.module = m
                chapter_pages.append(p)
                self.module_page[m] = p
            for cls, s, label in COLLATED:
                mods = [m for m in [intro] + sections if self.collated(m, cls)]
                if not mods:
                    continue
                p = Page(f"{n}-{s}.html", f"{n}장 {label}", f"<span class=\"num\">{n}장</span> {esc(label)}",
                         BOOK_PAGES + f"{n}-{s}", s, n)
                p.modules = mods
                p.cls = cls
                chapter_pages.append(p)
                self.end_pages[(n, s)] = p
            self.pages_by_chapter[n] = chapter_pages
            self.pages.extend(chapter_pages)

    def collated(self, m, cls):
        root = self.docs[m]
        if cls is None:
            return root.find(f".//{{{C}}}glossary/{{{C}}}definition") is not None
        return root.find(f".//{{{C}}}section[@class='{cls}']") is not None

    def page_of(self, m, e) -> Page:
        cls = collated_class(e)
        if cls is None and any(is_c(a, "glossary") for a in e.iterancestors()):
            return self.end_pages[(self.module_page[m].chapter, "key-terms")]
        if cls is None:
            return self.module_page[m]
        s = next(s for c, s, _ in COLLATED if c == cls)
        return self.end_pages[(self.module_page[m].chapter, s)]

    def index_targets(self):
        for m, root in self.docs.items():
            for e in root.iter():
                if isinstance(e.tag, str) and ns(e) == C and e.get("id"):
                    self.ids[(m, e.get("id"))] = (self.page_of(m, e).file, f"{m}-{e.get('id')}")
                if is_c(e, "section") and e.get("id"):
                    self.labels[(m, e.get("id"))] = plain(child(e, "title"))
        for n, _, intro, sections in self.chapters:
            counters = {"figure": 0, "table": 0}
            for m in [intro] + sections:
                for e in self.docs[m].iter(f"{{{C}}}figure", f"{{{C}}}table"):
                    if "unnumbered" in (e.get("class") or "").split():
                        continue
                    kind = local(e)
                    counters[kind] += 1
                    self.numbered[kind] += 1
                    word = "그림" if kind == "figure" else "표"
                    self.labels[(m, e.get("id"))] = f"{word} {n}.{counters[kind]}"
        if self.preface:
            counters = {"figure": 0, "table": 0}
            for e in self.docs[self.preface].iter(f"{{{C}}}figure", f"{{{C}}}table"):
                if "unnumbered" not in (e.get("class") or "").split():
                    counters[local(e)] += 1
                    self.numbered[local(e)] += 1
                    word = "그림" if local(e) == "figure" else "표"
                    self.labels[(self.preface, e.get("id"))] = f"{word} {counters[local(e)]}"


class Renderer:
    def __init__(self, book: Book, page: Page, media: set):
        self.book = book
        self.page = page
        self.media = media
        self.m = None
        self.depth = 1
        self.footnotes = []
        self.exercise_no = 0
        self.unknown = set()

    # --- helpers -----------------------------------------------------------
    def children(self, e, skip=()):
        out = [esc(e.text)]
        nodes = [c for c in e]
        i = 0
        while i < len(nodes):
            c = nodes[i]
            if not isinstance(c.tag, str):  # comment or processing instruction
                out.append(esc(c.tail))
                i += 1
                continue
            if is_c(c, "exercise") and is_embed(c):
                run = [c]
                while (i + len(run) < len(nodes) and is_c(nodes[i + len(run)], "exercise")
                       and is_embed(nodes[i + len(run)]) and not (run[-1].tail or "").strip()):
                    run.append(nodes[i + len(run)])
                out.append(self.embed_box(run))
                out.append(esc(run[-1].tail))
                i += len(run)
                continue
            if not (ns(c) == C and local(c) in skip):
                out.append(self.render(c))
            out.append(esc(c.tail))
            i += 1
        return "".join(out)

    def anchor(self, e) -> str:
        return f' id="{attr(self.m + "-" + e.get("id"))}"' if e.get("id") else ""

    def href(self, file, anchor=None) -> str:
        if file == self.page.file:
            return f"#{anchor}" if anchor else "#"
        return file + (f"#{anchor}" if anchor else "")

    # --- dispatch ----------------------------------------------------------
    def render(self, e) -> str:
        if ns(e) == MML:
            self.page.math = True
            return self.math(e, top=True)
        if ns(e) != C:
            self.unknown.add(etree.QName(e).text)
            return self.children(e)
        name = local(e)
        method = getattr(self, "e_" + name.replace("-", "_"), None)
        if method is None:
            self.unknown.add(name)
            return self.children(e)
        return method(e)

    # --- math --------------------------------------------------------------
    def math(self, e, top=False) -> str:
        name = local(e)
        attrs = "".join(f' {k}="{attr(v)}"' for k, v in e.attrib.items() if "}" not in k)
        if top and name == "math" and self.in_equation:
            attrs += ' display="block"'
        inner = esc(e.text) + "".join(self.math(c) + esc(c.tail) for c in e if isinstance(c.tag, str))
        return f"<{name}{attrs}>{inner}</{name}>"

    in_equation = False

    # --- structure ---------------------------------------------------------
    def e_content(self, e):
        return self.children(e)

    def e_metadata(self, e):
        return ""

    def e_glossary(self, e):
        return ""  # collected on the key terms page

    def e_title(self, e):
        return f"<p class=\"block-title\">{self.children(e)}</p>" if has_content(e) else ""

    def e_label(self, e):
        return ""

    def e_section(self, e):
        cls = e.get("class") or ""
        if cls in COLLATED_CLASSES:
            return ""
        return self.section(e)

    def section(self, e):
        cls = e.get("class") or ""
        title = child(e, "title")
        level = min(self.depth + 1, 6)
        heading = f"<h{level}>{self.children(title)}</h{level}>" if has_content(title) else ""
        self.depth += 1
        body = self.children(e, skip=("title",))
        self.depth -= 1
        classes = f' class="{attr(cls)}"' if cls else ""
        return f"<section{self.anchor(e)}{classes}>{heading}{body}</section>"

    def e_para(self, e):
        tag = "div" if any(is_c(d, *BLOCKISH) or (is_c(d, "code") and self.block_code(d))
                           or (is_c(d, "media") and child(d, "iframe") is not None)
                           for d in e.iterdescendants()) else "p"
        return f"<{tag}{self.anchor(e)}>{self.children(e)}</{tag}>"

    def e_list(self, e):
        title = child(e, "title")
        head = f"<p class=\"list-title\">{self.children(title)}</p>" if has_content(title) else ""
        if e.get("list-type") == "enumerated":
            t = LIST_STYLES.get(e.get("number-style"))
            tag, extra = "ol", (f' type="{t}"' if t else "")
        else:
            tag, extra = "ul", (' class="plain"' if e.get("bullet-style") == "none" else "")
        return f"{head}<{tag}{self.anchor(e)}{extra}>{self.children(e, skip=('title',))}</{tag}>"

    def e_item(self, e):
        return f"<li{self.anchor(e)}>{self.children(e)}</li>"

    def e_note(self, e):
        cls = e.get("class") or ""
        label = NOTE_LABELS.get(cls)
        if label is None:
            raise SystemExit(f"{self.m}: note class {cls!r} has no label")
        title = child(e, "title")
        t = f"<span class=\"note-title\">{self.children(title)}</span>" if has_content(title) else ""
        return (f"<aside{self.anchor(e)} class=\"note {attr(cls)}\"><div class=\"note-head\">"
                f"<span class=\"note-label\">{label}</span>{t}</div>"
                f"{self.children(e, skip=('title',))}</aside>")

    def e_quote(self, e):
        return f"<blockquote{self.anchor(e)}>{self.children(e)}</blockquote>"

    # --- exercises ---------------------------------------------------------
    def e_exercise(self, e):
        problem = child(e, "problem")
        solutions = e.findall(f"{{{C}}}solution")
        out = [f"<div{self.anchor(e)} class=\"exercise\">"]
        if problem is not None:
            out.append(f"<div class=\"problem\"{self.anchor(problem)}>{self.children(problem)}</div>")
        for s in solutions:
            out.append(f"<details class=\"solution\"{self.anchor(s)}><summary>풀이</summary>"
                       f"{self.children(s)}</details>")
        out.append("</div>")
        return "".join(out)

    def embed_box(self, run):
        first = self.exercise_no + 1
        self.exercise_no += len(run)
        last = self.exercise_no
        numbers = f"{first}번" if first == last else f"{first}–{last}번"
        anchors = "".join(f"<span{self.anchor(x)}></span>" for x in run)
        return (f"<div class=\"exercise-embed\">{anchors}<p>문제 {numbers}은 원문 사이트의 연습문제 서비스가 "
                f"제공하는 문제라 이 번역에 싣지 않았습니다. <a href=\"{attr(self.page.origin)}\">원문 페이지"
                f"</a>에서 풀어 볼 수 있습니다(영어).</p></div>")

    def e_problem(self, e):
        return f"<div class=\"problem\"{self.anchor(e)}>{self.children(e)}</div>"

    def e_solution(self, e):
        return f"<details class=\"solution\"{self.anchor(e)}><summary>풀이</summary>{self.children(e)}</details>"

    # --- figures, tables, media, equations ---------------------------------
    def caption_html(self, e):
        label = self.book.labels.get((self.m, e.get("id")))
        parts = []
        if label:
            parts.append(f"<span class=\"label\">{esc(label)}</span>")
        title = child(e, "title")
        if has_content(title):
            parts.append(f"<span class=\"title\">{self.children(title)}</span>")
        caption = child(e, "caption")
        if has_content(caption):
            parts.append(f"<span class=\"caption\">{self.children(caption)}</span>")
        return " ".join(parts)

    def e_figure(self, e):
        cls = e.get("class") or ""
        body = self.children(e, skip=("title", "caption"))
        cap = self.caption_html(e)
        cap = f"<figcaption>{cap}</figcaption>" if cap else ""
        return f"<figure{self.anchor(e)} class=\"{attr(cls)}\">{body}{cap}</figure>"

    def e_subfigure(self, e):
        return self.e_figure(e)

    def e_media(self, e):
        image = child(e, "image")
        if image is not None:
            name = Path(image.get("src")).name
            self.media.add(name)
            return (f"<img src=\"media/{attr(name)}\" alt=\"{attr(e.get('alt'))}\" loading=\"lazy\""
                    f"{self.anchor(e)}>")
        iframe = child(e, "iframe")
        if iframe is not None:
            src = iframe.get("src")
            return (f"<p class=\"video-link\"{self.anchor(e)}><a href=\"{attr(src)}\">동영상 보기</a>"
                    f" <span class=\"host\">(외부 사이트, 영어)</span></p>")
        self.unknown.add("media without image or iframe")
        return ""

    def e_image(self, e):
        return ""

    def e_iframe(self, e):
        return ""

    def e_equation(self, e):
        self.in_equation = True
        body = self.children(e, skip=("label",))
        self.in_equation = False
        return f"<div class=\"equation\"{self.anchor(e)}>{body}</div>"

    def e_table(self, e):
        cap = self.caption_html(e)
        cap = f"<caption>{cap}</caption>" if cap else ""
        summary = (e.get("summary") or "").strip()
        aria = f' aria-label="{attr(summary)}"' if summary else ""
        cls = e.get("class") or ""
        groups = "".join(self.tgroup(g) for g in e.findall(f"{{{C}}}tgroup"))
        return (f"<div class=\"table-wrap\"><table{self.anchor(e)} class=\"{attr(cls)}\"{aria}>"
                f"{cap}{groups}</table></div>")

    def tgroup(self, g):
        names = {}
        for i, spec in enumerate(g.findall(f"{{{C}}}colspec"), 1):
            names[spec.get("colname")] = int(spec.get("colnum") or i)
        out = []
        for part, cell in (("thead", "th"), ("tbody", "td"), ("tfoot", "td")):
            section = child(g, part)
            if section is None:
                continue
            rows = []
            for row in section.findall(f"{{{C}}}row"):
                cls = f' class="{attr(row.get("class"))}"' if row.get("class") else ""
                cells = []
                for entry in row.findall(f"{{{C}}}entry"):
                    a = ""
                    if entry.get("namest") and entry.get("nameend"):
                        span = names[entry.get("nameend")] - names[entry.get("namest")] + 1
                        if span > 1:
                            a += f' colspan="{span}"'
                    if entry.get("morerows"):
                        a += f' rowspan="{int(entry.get("morerows")) + 1}"'
                    align = entry.get("align")
                    if align in ("left", "right", "center"):
                        a += f' class="{align}"'
                    cells.append(f"<{cell}{self.anchor(entry)}{a}>{self.children(entry)}</{cell}>")
                rows.append(f"<tr{cls}>{''.join(cells)}</tr>")
            out.append(f"<{part}>{''.join(rows)}</{part}>")
        return "".join(out)

    def e_tgroup(self, e):
        return self.tgroup(e)

    def e_caption(self, e):
        return self.children(e)

    # --- inline ------------------------------------------------------------
    def e_emphasis(self, e):
        tag = {"italics": "em", "underline": "u"}.get(e.get("effect"), "strong")
        return f"<{tag}>{self.children(e)}</{tag}>"

    def e_term(self, e):
        if (e.get("class") or "") == "no-emphasis":
            return f"<span class=\"term-plain\"{self.anchor(e)}>{self.children(e)}</span>"
        return f"<dfn class=\"term\"{self.anchor(e)}>{self.children(e)}</dfn>"

    def e_foreign(self, e):
        return f"<i>{self.children(e)}</i>"

    def e_cite_title(self, e):
        return f"<cite>{self.children(e)}</cite>"

    def e_sub(self, e):
        return f"<sub>{self.children(e)}</sub>"

    def e_sup(self, e):
        return f"<sup>{self.children(e)}</sup>"

    def e_newline(self, e):
        return "<br>" * int(e.get("count") or 1)

    def e_span(self, e):
        cls = e.get("class") or ""
        if cls in ("primary-function", "secondary-function"):
            return f"<kbd class=\"{cls}\">{self.children(e)}</kbd>"
        return f"<span class=\"{attr(cls)}\">{self.children(e)}</span>"

    def block_code(self, e):
        return e.get("display") == "block" or "centered-code" in (e.get("class") or "").split()

    def e_code(self, e):
        if not self.block_code(e):
            return f"<code>{self.children(e)}</code>"
        inner = self.children(e)
        # Source indentation is the module's, not the code's.
        inner = textwrap.dedent(inner.replace("\t", "    ")).strip("\n")
        cls = " ".join(["code"] + (e.get("class") or "").split())
        return f"<pre class=\"{attr(cls)}\"{self.anchor(e)}><code>{inner}</code></pre>"

    def e_preformat(self, e):
        return f"<pre{self.anchor(e)}>{self.children(e)}</pre>"

    def e_footnote(self, e):
        n = len(self.footnotes) + 1
        fid = f"fn-{self.m}-{e.get('id')}"
        self.footnotes.append((fid, self.children(e)))
        return f"<sup class=\"fn-ref\" id=\"ref-{attr(fid)}\"><a href=\"#{attr(fid)}\">{n}</a></sup>"

    def e_link(self, e):
        if e.get("class") == "os-embed":
            return f"<a href=\"{attr(self.page.origin)}\">[원문 연습문제]</a>"
        if e.get("url"):
            url = e.get("url")
            text = self.children(e) if has_content(e) else esc(url)
            return f"<a href=\"{attr(url)}\">{text}</a>"
        doc = e.get("document") or self.m
        target = e.get("target-id")
        if target:
            if (doc, target) not in self.book.ids:
                raise SystemExit(f"{self.m}: link to missing {doc}#{target}")
            file, anchor = self.book.ids[(doc, target)]
            text = self.children(e) if has_content(e) else esc(self.book.labels.get((doc, target), ""))
            if not text.strip():
                raise SystemExit(f"{self.m}: no text for link to {doc}#{target}")
            return f"<a class=\"xref\" href=\"{attr(self.href(file, anchor))}\">{text}</a>"
        if doc not in self.book.module_page:
            raise SystemExit(f"{self.m}: link to module {doc} outside the book")
        page = self.book.module_page[doc]
        text = self.children(e) if has_content(e) else esc(page.title)
        return f"<a href=\"{attr(self.href(page.file))}\">{text}</a>"

    # --- pages -------------------------------------------------------------
    def footnote_list(self):
        if not self.footnotes:
            return ""
        items = "".join(f"<li id=\"{attr(fid)}\">{body} <a class=\"back\" href=\"#ref-{attr(fid)}\" "
                        f"aria-label=\"본문으로\">↩</a></li>" for fid, body in self.footnotes)
        return f"<section class=\"footnotes\"><h2>각주</h2><ol>{items}</ol></section>"

    def module_body(self, m, depth):
        self.m = m
        self.depth = depth
        return self.children(child(self.book.docs[m], "content"))


def outline(book: Book, n: int) -> str:
    items = "".join(f"<li><a href=\"{p.file}\">{esc(p.title)}</a></li>"
                    for p in book.pages_by_chapter[n] if p.kind != "intro")
    return f"<nav class=\"outline\" aria-label=\"장 목차\"><h2>이 장의 구성</h2><ul>{items}</ul></nav>"


def render_page(book: Book, page: Page, media: set) -> Renderer:
    r = Renderer(book, page, media)
    if page.kind in ("preface", "section"):
        page.body = r.module_body(page.module, 1)
    elif page.kind == "intro":
        body = r.module_body(page.module, 2)
        page.body = (outline(book, page.chapter) + f"<h2>{esc(book.doc_title(page.module))}</h2>" + body)
    elif page.kind == "key-terms":
        entries = []
        for m in page.modules:
            r.m = m
            for d in book.docs[m].iter(f"{{{C}}}definition"):
                term = child(d, "term")
                meaning = child(d, "meaning")
                entries.append((plain(term).lower(), f"<dt{r.anchor(d)}>{r.children(term)}</dt>"
                                                     f"<dd>{r.children(meaning)}</dd>"))
        entries.sort(key=lambda x: x[0])
        page.body = f"<dl class=\"glossary\">{''.join(h for _, h in entries)}</dl>"
    else:
        parts = []
        for m in page.modules:
            r.m = m
            r.depth = 2
            num = book.section_number.get(m)
            head = f"{num} {book.doc_title(m)}" if num else book.doc_title(m)
            target = book.module_page[m].file
            parts.append(f"<h2><a href=\"{target}\">{esc(head)}</a></h2>")
            for s in book.docs[m].iter(f"{{{C}}}section"):
                if s.get("class") == page.cls:
                    r.depth = 2
                    parts.append(f"<div{r.anchor(s)} class=\"{attr(page.cls)}\">"
                                 f"{r.children(s, skip=('title',))}</div>")
        page.body = "".join(parts)
    page.body += r.footnote_list()
    return r


FOOTER = """<footer class="site-footer">
<p>원서: Julie Dahlquist, Rainford Knight, <cite>Principles of Finance 2e</cite>, OpenStax(Rice University).
<a href="{origin}">이 쪽의 원문</a> · <a href="{details}">원서 소개</a></p>
<p>원서와 이 번역은 <a href="{license}" rel="license">CC BY-NC-SA 4.0</a>에 따라 비상업적 목적으로만 이용할 수
있습니다. 이 번역은 원서를 한국어로 옮기고 연습문제 서비스의 문제를 원문 링크로 바꾼 비공식 2차적
저작물이며, 원 저작자나 OpenStax의 공식 번역이 아니고 그 보증을 받지 않았습니다.</p>
<p><a href="https://github.com/yeokja/yeokja">yeokja</a>와 함께 Anthropic 사의 <code>claude-opus-5-5</code> 모델을
활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.</p>
</footer>"""


def document(book: Book, title: str, main: str, origin: str, math: bool, pager: str = "") -> str:
    head_math = f'<script defer src="{MATHJAX}"></script>' if math else ""
    footer = FOOTER.format(origin=attr(origin), details=BOOK_DETAILS, license=LICENSE_URL)
    return f"""<!doctype html>
<html lang="ko">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{esc(title)}</title>
<link rel="stylesheet" href="site.css">
{head_math}
</head>
<body>
<header class="site-header"><a href="index.html">{esc(book.title)}</a></header>
{pager}
<main>
{main}
</main>
{pager}
{footer}
</body>
</html>
"""


def pager(prev, nxt) -> str:
    left = f"<a rel=\"prev\" href=\"{prev.file}\">← {esc(prev.title)}</a>" if prev else "<span></span>"
    right = f"<a rel=\"next\" href=\"{nxt.file}\">{esc(nxt.title)} →</a>" if nxt else "<span></span>"
    return f"<nav class=\"pager\">{left}<a href=\"index.html\">목차</a>{right}</nav>"


def landing(book: Book) -> str:
    toc = []
    if book.preface:
        p = book.module_page[book.preface]
        toc.append(f"<li><a href=\"{p.file}\">{esc(p.title)}</a></li>")
    for n, title, _, _ in book.chapters:
        pages = book.pages_by_chapter[n]
        inner = "".join(f"<li><a href=\"{p.file}\">{esc(p.title)}</a></li>" for p in pages[1:])
        toc.append(f"<li><a href=\"{pages[0].file}\">{n}장 {esc(title)}</a><ul>{inner}</ul></li>")
    main = f"""<h1>{esc(book.title)}</h1>
<p class="subtitle">Principles of Finance 2e 한국어 번역 · Julie Dahlquist, Rainford Knight 지음</p>
<p>OpenStax가 펴낸 개방형 재무관리 입문 교재를 한국어로 옮긴 비공식 번역입니다. 재무의 기초와 기업 지배구조,
회계와 재무제표, 화폐의 시간가치, 채권과 주식의 가치평가, 위험과 수익률, 자본예산과 자본비용, 운전자본
관리와 국제재무까지 20개 장으로 이루어져 있습니다.</p>
<p>각 절 끝의 객관식·복습·연습 문제와 동영상 활동 문제는 원문 사이트의 연습문제 서비스가 제공하므로 이 번역에
싣지 않고 원문 페이지로 연결합니다. 그림 속 글자와 엑셀 화면은 원문 그대로입니다.</p>
<nav class="toc" aria-label="목차"><h2>목차</h2><ul>{''.join(toc)}</ul></nav>"""
    return document(book, book.title, main, BOOK_DETAILS, False)


CSS = """:root {
  --fg: #1d1d1f; --bg: #fdfdfb; --muted: #5f6368; --line: #d9d9d4; --accent: #0b5cad;
  --note: #f2f5f9; --note-line: #8aa6c7; --code: #f4f4f1; --head: #eef1f4;
}
@media (prefers-color-scheme: dark) {
  :root {
    --fg: #e6e6e3; --bg: #17181a; --muted: #a2a6ab; --line: #3a3c40; --accent: #7fb6f0;
    --note: #1f252d; --note-line: #4f6e92; --code: #222427; --head: #23272c;
  }
  img { background: #fff; }
}
* { box-sizing: border-box; }
html { -webkit-text-size-adjust: 100%; }
body {
  margin: 0; padding: 0 16px; color: var(--fg); background: var(--bg);
  font-family: "Pretendard", "Apple SD Gothic Neo", "Noto Sans KR", "Malgun Gothic", system-ui, sans-serif;
  line-height: 1.75; word-break: keep-all; overflow-wrap: break-word;
}
main, .site-header, .pager, .site-footer { max-width: 46rem; margin: 0 auto; }
a { color: var(--accent); }
.site-header { padding: 1rem 0; border-bottom: 1px solid var(--line); font-weight: 700; }
.site-header a { text-decoration: none; color: var(--fg); }
.pager { display: flex; justify-content: space-between; gap: 1rem; padding: .75rem 0; font-size: .9rem; }
.pager a { text-decoration: none; }
.pager a[rel=next] { text-align: right; }
h1 { font-size: 1.8rem; line-height: 1.35; margin: 1.5rem 0 1rem; }
h2 { font-size: 1.4rem; line-height: 1.4; margin-top: 2.2rem; }
h3 { font-size: 1.15rem; margin-top: 1.8rem; }
h4, h5, h6 { font-size: 1rem; }
.num { color: var(--muted); margin-right: .3em; }
.subtitle { color: var(--muted); }
img { max-width: 100%; height: auto; }
figure { margin: 1.5rem 0; text-align: center; }
figure.scaled-down img { max-width: min(100%, 36rem); }
figure.scaled-down-30 img { max-width: min(100%, 20rem); }
figcaption, caption { font-size: .9rem; color: var(--muted); text-align: left; margin-top: .5rem; }
figcaption .label, caption .label { font-weight: 700; color: var(--fg); margin-right: .3em; }
figcaption .title, caption .title { font-weight: 600; color: var(--fg); }
.table-wrap { overflow-x: auto; margin: 1.5rem 0; }
table { border-collapse: collapse; font-size: .9rem; min-width: 50%; }
caption { caption-side: top; margin-bottom: .5rem; }
th, td { border: 1px solid var(--line); padding: .35rem .6rem; vertical-align: top; }
th { background: var(--head); }
td.right, th.right { text-align: right; } td.center, th.center { text-align: center; }
tr.gray-shading td { background: var(--head); }
.equation { overflow-x: auto; overflow-y: hidden; margin: 1rem 0; text-align: center; }
code, kbd { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: .9em; }
pre.code { background: var(--code); padding: .75rem 1rem; overflow-x: auto; line-height: 1.5; }
pre.centered-code { text-align: center; }
kbd { border: 1px solid var(--line); border-radius: 3px; padding: 0 .3em; background: var(--code); }
kbd.secondary-function { font-style: italic; }
.blue-code { color: #2a62c9; } .red-code { color: #c0392b; }
dfn.term { font-style: normal; font-weight: 700; }
aside.note, section.learning-objectives {
  background: var(--note); border-left: 4px solid var(--note-line); padding: .5rem 1rem; margin: 1.5rem 0;
}
section.learning-objectives h2 { margin-top: .5rem; font-size: 1.1rem; }
.note-head { font-weight: 700; margin: .5rem 0; }
.note-label { display: inline-block; font-size: .8rem; color: var(--muted); margin-right: .6em; }
.exercise { border-top: 1px dashed var(--line); padding-top: .5rem; margin-top: 1rem; }
details.solution { margin: .5rem 0 1rem; }
details.solution summary { cursor: pointer; font-weight: 600; }
.exercise-embed { border: 1px solid var(--line); border-radius: 6px; padding: .25rem 1rem; margin: 1rem 0; font-size: .95rem; }
.video-link .host { color: var(--muted); font-size: .85rem; }
ul.plain { list-style: none; padding-left: 0; }
.outline ul, .toc ul { padding-left: 1.2rem; }
.toc > ul > li { margin-top: .6rem; font-weight: 600; }
.toc > ul > li li { font-weight: 400; }
dl.glossary dt { font-weight: 700; margin-top: 1rem; }
dl.glossary dd { margin-left: 0; }
.footnotes { font-size: .85rem; border-top: 1px solid var(--line); margin-top: 3rem; }
.footnotes h2 { font-size: 1rem; }
.fn-ref a { text-decoration: none; }
.site-footer { border-top: 1px solid var(--line); margin-top: 2rem; padding: 1rem 0 3rem; font-size: .8rem; color: var(--muted); }
"""


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__, file=sys.stderr)
        return 2
    tree, out, upstream = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    book = Book(tree, upstream)
    if out.exists():
        shutil.rmtree(out)
    (out / "media").mkdir(parents=True)
    media, unknown = set(), set()
    for page in book.pages:
        unknown |= render_page(book, page, media).unknown
    if unknown:
        raise SystemExit(f"unknown elements: {sorted(unknown)}")
    for i, page in enumerate(book.pages):
        prev = book.pages[i - 1] if i > 0 else None
        nxt = book.pages[i + 1] if i + 1 < len(book.pages) else None
        main_html = f"<h1>{page.heading}</h1>\n{page.body}"
        (out / page.file).write_text(document(book, page.title, main_html, page.origin, page.math,
                                              pager(prev, nxt)), encoding="utf-8")
    (out / "index.html").write_text(landing(book), encoding="utf-8")
    (out / "site.css").write_text(CSS, encoding="utf-8")
    for name in sorted(media):
        shutil.copy2(tree / "media" / name, out / "media" / name)
    print(f"site: {len(book.pages) + 1} pages, {len(media)} images, "
          f"{book.numbered['figure']} figures and {book.numbered['table']} tables numbered")
    return 0


if __name__ == "__main__":
    sys.exit(main())
