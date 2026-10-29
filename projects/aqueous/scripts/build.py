#!/usr/bin/env python3
"""Build the Korean Aqueous spec next to the original with the same Bikeshed.

Usage: build.py ROOT TREE

ROOT is the project directory (YEOKJA_ROOT), TREE the assembled derive tree
(upstream with ko/ overlaid), which is also the working directory of the yeokja
build command. Steps:

1. Append a metadata block to the translated spec (TREE/spec/index.bs ->
   TREE/spec/index.ko.bs): Korean title, status macro, the snapshot commit date,
   a custom warning box naming the snapshot commit, the original and the
   licence, and a Translation entry pointing at the original. Bikeshed merges
   metadata blocks and the later single-valued keys win, so the original file
   is never copied or edited.
2. Build ROOT/upstream/spec/index.bs and TREE/spec/index.ko.bs with
   `bikeshed --die-on=warning`.
3. Run scripts/check.py on the two HTML files.
4. Localise the strings Bikeshed generates in English (headings, header
   labels, sidebar script strings, counter labels); every substitution must
   match exactly the expected number of times, so a Bikeshed upgrade that
   changes its boilerplate fails the build instead of leaving English behind.
5. Write site/index.html and site/LICENSE.txt.

Standard library only; `bikeshed` and `git` come from PATH.
"""

from __future__ import annotations

import datetime as dt
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

TITLE = "Aqueous 아카이브 형식"
ORIGINAL_URL = "https://lifthrasiir.github.io/aqueous/spec/"
REPO_URL = "https://github.com/lifthrasiir/aqueous"
TRANSLATION_REPO = "https://github.com/yeokja/yeokja"


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    print("+", " ".join(cmd), file=sys.stderr)
    return subprocess.run(cmd, check=True, text=True, **kw)


def snapshot(upstream: Path) -> tuple[str, str]:
    out = run(["git", "-C", str(upstream), "log", "-1", "--format=%H %cs"], capture_output=True).stdout
    commit, date = out.split()
    return commit, date


def korean_date(iso: str) -> str:
    d = dt.date.fromisoformat(iso)
    return f"{d.year}년 {d.month}월 {d.day}일"


def metadata_block(commit: str, date: str) -> str:
    short = commit[:8]
    commit_url = f"{REPO_URL}/blob/{commit}/spec/index.bs"
    paragraphs = [
        f'이 문서는 Kang Seonghoon의 <a href="{ORIGINAL_URL}">Aqueous Archive Format</a> 편집자 초안을 '
        f'<a href="{TRANSLATION_REPO}">yeokja</a>로 기계 번역한 비공식 한국어 번역이며, '
        f'원문 커밋 <a href="{commit_url}"><code>{short}</code></a>({korean_date(date)}) 시점의 스냅샷입니다.',
        "원문은 매주 크게 바뀌는 편집자 초안이므로 이 번역은 그 시점의 내용만 반영하며, "
        "저자의 공식 번역이 아니고 저자가 검토하지 않았습니다. "
        f'최신 내용은 <a href="{ORIGINAL_URL}">원문</a>을 보십시오.',
        "원문은 공개 도메인에 헌정되었으며 그것이 불가능한 곳에서는 MIT-0으로 제공됩니다. "
        '이 번역도 같은 조건으로 제공합니다(<a href="LICENSE.txt">LICENSE.txt</a>).',
    ]
    lines = [
        "",
        "<pre class=metadata>",
        f"Title: {TITLE}",
        "Text Macro: LONGSTATUS 편집자 초안",
        f"Date: {date}",
        "Warning: custom",
        "Custom Warning Title: 비공식 한국어 번역",
    ]
    lines += [f"Custom Warning Text: <p>{p}</p>" for p in paragraphs]
    lines += [
        f"Translation: en {ORIGINAL_URL}, name 영어 원문, native-name English",
        "</pre>",
        "",
    ]
    return "\n".join(lines)


def bikeshed(src: Path, out: Path) -> None:
    out.unlink(missing_ok=True)
    run(["bikeshed", "--print=plain", "--die-on=warning", "--die-when=late", "--no-update",
         "spec", str(src), str(out)])
    # Bikeshed exits 0 even when it refuses to generate the output.
    if not out.is_file():
        sys.exit(f"bikeshed did not generate {out}")


def substitute(text: str, old: str, new: str, count: int = 1) -> str:
    n = text.count(old)
    if n != count:
        sys.exit(f"localise: expected {count} occurrence(s) of {old!r}, found {n}")
    return text.replace(old, new)


def localise(html: str, date: str) -> str:
    html = substitute(html, '<!doctype html><html lang="en">', '<!doctype html><html lang="ko">')
    headings = [
        ("abstract", "Abstract", "요약"),
        ("index", "Index", "색인"),
        ("index-defined-here", "Terms defined by this specification", "이 명세가 정의하는 용어"),
        ("index-defined-elsewhere", "Terms defined by reference", "참조로 정의된 용어"),
        ("references", "References", "참고 문헌"),
        ("normative", "Normative References", "규범적 참고 문헌"),
        ("informative", "Non-Normative References", "비규범적 참고 문헌"),
        ("cddl-index", "CDDL Index", "CDDL 색인"),
        ("issues-index", "Issues Index", "이슈 색인"),
    ]
    for id_, en, ko in headings:
        html = substitute(html, f'id="{id_}"><span class="content">{en}</span>',
                          f'id="{id_}"><span class="content">{ko}</span>')
    html = substitute(html, 'id="contents">Table of Contents</h2>', 'id="contents">목차</h2>')
    html = substitute(html, '<dt class="editor">Editor:', '<dt class="editor">편집자:')
    html = substitute(html, "<dt>Translations <small>(non-normative)</small>:",
                      "<dt>다른 언어 판 <small>(비규범적)</small>:")
    html = substitute(html, "<span>Collapse Sidebar</span>", "<span>사이드바 접기</span>")
    html = substitute(html, "<span>Pop Out Sidebar</span>", "<span>사이드바 펼치기</span>")
    html = substitute(html, "<span>Jump to Table of Contents</span>", "<span>목차로 이동</span>")
    html = substitute(html, 'mk.b({}, "Referenced in:")', 'mk.b({}, "참조된 곳:")')
    # The header date: Bikeshed formats it in English; the datetime attribute keeps the ISO date.
    html = substitute(html, f'<time class="dt-updated" datetime="{date}">', "\0TIME\0")
    html = re.sub("\0TIME\0[^<]*</time>",
                  f'<time class="dt-updated" datetime="{date}">{korean_date(date)}</time>', html, count=1)
    # Counter labels are CSS-generated; override them after Bikeshed's stylesheet.
    style = """<style>/* yeokja: Korean counter labels */
.issue:not(.no-marker)::before { content: "이슈 " counter(issue); }
.example:not(.no-marker)::before { content: "예시 " counter(example); }
.invalid.example:not(.no-marker)::before,
.illegal.example:not(.no-marker)::before { content: "잘못된 예시 " counter(example); }
figcaption:not(.no-marker)::before { content: "그림 " counter(figure) " "; }
figure.table figcaption:not(.no-marker)::before { content: "표 " counter(table) " "; }
/* The translation notice stays in the header instead of floating over the page. */
details.annoying-warning[open] { position: static; margin: 1em 0; text-align: left; }
</style>
<body class="h-entry">"""
    # Bikeshed omits the optional </head>; the stylesheet goes right before <body>.
    html = substitute(html, '<body class="h-entry">', style)
    return html


def main() -> int:
    root = Path(sys.argv[1]).resolve()
    tree = Path(sys.argv[2]).resolve()
    upstream = root / "upstream"
    en_src = upstream / "spec" / "index.bs"
    ko_src = tree / "spec" / "index.bs"
    ko_aug = tree / "spec" / "index.ko.bs"
    build = tree / "build"
    build.mkdir(exist_ok=True)

    commit, date = snapshot(upstream)
    ko_text = ko_src.read_text(encoding="utf-8")
    m = re.match(r"<h1>(.*?)</h1>", ko_text)
    if not m or m.group(1) != TITLE:
        sys.exit(f"translated <h1> is {m.group(1) if m else None!r}, expected {TITLE!r}; fix the glossary or TITLE")
    ko_aug.write_text(ko_text + metadata_block(commit, date), encoding="utf-8")

    en_html = build / "en.html"
    ko_html = build / "ko.html"
    bikeshed(en_src, en_html)
    bikeshed(ko_aug, ko_html)
    run([sys.executable, "-I", str(root / "scripts" / "check.py"), str(en_html), str(ko_html)])

    site = tree / "site"
    shutil.rmtree(site, ignore_errors=True)
    site.mkdir()
    (site / "index.html").write_text(localise(ko_html.read_text(encoding="utf-8"), date), encoding="utf-8")
    shutil.copyfile(upstream / "LICENSE.txt", site / "LICENSE.txt")
    print(f"site/index.html built from snapshot {commit[:8]} ({date})")
    return 0


if __name__ == "__main__":
    os.environ.setdefault("PYTHONIOENCODING", "utf-8")
    sys.exit(main())
