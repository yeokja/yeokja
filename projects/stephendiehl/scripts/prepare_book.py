#!/usr/bin/env python3
"""Lay out an mdBook from the reconstructed blog (English or Korean).

``prepare_book.py <root> <out> --lang en|ko`` reads ``<root>/posts/*.md``,
``<root>/index.md`` and ``<root>/license.md`` (the English ``source/`` tree or
the assembled tree with ``ko/`` overlaid) plus ``manifest.json`` and writes
``<out>/book.toml``, ``<out>/custom.css`` and ``<out>/src/``: a SUMMARY grouped
by year (newest first), each post with its front matter replaced by a visible
date/tags/original-link line, the home page with the site notice, and the
license page (English text, with a Korean summary in the Korean book).

Both languages are laid out the same way so heading counts match and the
English heading anchors can be carried over to the Korean pages.
"""

from __future__ import annotations

import argparse
import html
import json
import re
from pathlib import Path

KO_MONTH = "{y}년 {m}월 {d}일"
EN_MONTHS = ["January", "February", "March", "April", "May", "June", "July",
             "August", "September", "October", "November", "December"]

TITLES = {"en": "Stephen Diehl", "ko": "Stephen Diehl 블로그 한국어 번역"}
INDEX_TITLE = {"en": "About", "ko": "소개"}
LICENSE_TITLE = {"en": "License", "ko": "라이선스와 고지"}

KO_INDEX_NOTICE = """\
## 이 사이트에 대하여

이 사이트는 Stephen Diehl의 블로그 [stephendiehl.com](https://www.stephendiehl.com)에
실린 글을 한국어로 옮긴 **비공식 번역**입니다. 저자가 검토하거나 승인한
번역이 아니며, [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의
`claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로
작업하였습니다. 번역 과정에서 생긴 오류는 원문이 아니라 번역의 책임입니다.

- 원문: Stephen Diehl, [stephendiehl.com](https://www.stephendiehl.com). 글은
  [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/), 코드는 MIT로
  제공됩니다. 이 번역은 원문을 한국어로 옮기며 **변경한** 파생물이며 같은
  CC BY-SA 4.0으로 제공합니다.
- 저자가 밝힌 대로, 이 글들은 저자가 **개인 자격으로** 쓴 것입니다. 이 번역본을
  인용하거나 언급할 때 저자의 고용주·고객·단체 등 어떤 조직과의 연관도 밝히거나
  암시해서는 안 됩니다. 저자가 정한 소개 문구는 "Stephen Diehl은 런던의
  소프트웨어 엔지니어이자 작가입니다"입니다. 자세한 내용은
  [라이선스와 고지](license.md)를 보십시오.
- 글마다 원문 링크와 게시일을 상단에 적었습니다. 이미지는 재배포하지 않고 원문
  사이트의 것을 그대로 불러옵니다.
"""

EN_INDEX_NOTICE = """\
## About this site

This is the English text of the posts on [stephendiehl.com](https://www.stephendiehl.com),
reconstructed from the published pages as the source of the Korean translation.
Written content is © Stephen Diehl, [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/);
code is MIT. See [License](license.md).
"""

KO_LICENSE_SUMMARY = """\
# 라이선스와 고지

아래는 원문 사이트의 [라이선스 페이지](https://www.stephendiehl.com/license/)를
한국어로 요약한 것이고, 그 뒤에 영어 원문 전체를 그대로 실었습니다. 해석이
다를 때는 영어 원문이 기준입니다.

## 요약

- **글**(에세이, 블로그 글, 노트, 서평 등 모든 산문)은
  [Creative Commons Attribution-ShareAlike 4.0 International](https://creativecommons.org/licenses/by-sa/4.0/)(CC BY-SA 4.0)로
  제공됩니다. 출처를 "Stephen Diehl, [stephendiehl.com](http://stephendiehl.com)"으로
  밝히고, 라이선스 링크를 달고, 변경했다면 변경 사실을 표시해야 하며, 개작물은
  같은 라이선스로 배포해야 합니다. 저자는 모든 글에 대한 저작인격권(저자로
  식별될 권리 포함)을 주장합니다.
- **코드**(글 속 스니펫, 전체 예제, 사이트 소스)는 글에서 따로 밝히지 않는 한
  [MIT 라이선스](https://opensource.org/licenses/MIT)입니다.
- **개인 의견 고지**: 이 사이트의 모든 글은 저자가 사적인 시민으로서 공익에 관한
  사안에 대해 자비로 쓴 것이며, 어떤 외국 정부·정당·캠페인·외국 단체의 자금도
  받지 않았고 지시나 조율을 받지도 않았습니다. 여기 담긴 의견·분석·주장은 전적으로
  저자 개인의 것이며, 저자가 과거나 현재에 관계된 어떤 고용주·고객·자문처·이사회·
  비영리 단체·조직의 견해도 대변하지 않습니다. 이 글들에서 어떤 기관의 지지도
  추론해서는 안 됩니다. **이는 정중한 부탁이 아니라 엄격한 규칙입니다**: 이
  사이트의 글을 인용·참조·언급할 때 그런 조직을 이름 붙이거나, 특정하거나,
  암시하거나, 그 밖의 방식으로 관여를 시사해서는 안 됩니다. 그렇게 하면 이 글의
  출처를 왜곡하고, 저자만의 견해에 제3자를 잘못 결부시키게 됩니다.
- **인용과 언론**: 언론·학술·기타 출판물에서 저자의 글을 인용할 때는 위 라이선스
  조건 아래 자유롭게 할 수 있습니다. 저자가 미리 승인한 소개 문구는
  "Stephen Diehl is a software engineer and writer from London."(Stephen Diehl은
  런던의 소프트웨어 엔지니어이자 작가입니다)이며, 이 문구와 원문 글로의 링크를
  함께 쓰면 라이선스의 출처 표기 요건과 위 고지를 모두 충족합니다. 학술 인용은
  APA 형식 "Diehl, S. (연도, 월 일). *글 제목*. Stephen Diehl.
  https://stephendiehl.com/posts/slug"를 권합니다.
- 위에 해당하지 않는 용도는 저자에게 직접 문의하십시오.

## 이 번역본에 대하여

이 사이트는 위 CC BY-SA 4.0 글을 한국어로 옮긴 파생물입니다. 번역하면서 원문을
변경했고(언어, 그리고 사이트 구성), 같은 CC BY-SA 4.0으로 제공합니다. 코드
블록은 원문 그대로이며 MIT입니다. 번역은 [yeokja](https://github.com/yeokja/yeokja)와
함께 Anthropic 사의 `claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두
비허용한 상태로 작업하였습니다. 저자가 검토하거나 승인한 번역이 아닙니다.

---

## Original license text

"""

CUSTOM_CSS = """\
.post-meta { color: var(--fg); opacity: 0.7; font-size: 0.9em; margin-top: -0.5em; }
.post-meta time { font-variant-numeric: tabular-nums; }
footer.translation-credit { margin-top: 3em; font-size: 0.85em; opacity: 0.8; }
main img { max-width: 100%; height: auto; }
"""


def split_front_matter(text: str) -> tuple[dict[str, object], str]:
    """Return (fields, body) for a post written by fetch_site.py."""
    match = re.match(r"---\n(.*?)\n---\n+", text, re.S)
    if not match:
        return {}, text
    fields: dict[str, object] = {}
    for line in match.group(1).splitlines():
        key, _, value = line.partition(":")
        value = value.strip()
        if value.startswith("["):
            fields[key.strip()] = json.loads(value)
        else:
            fields[key.strip()] = json.loads(value) if value.startswith('"') else value
    return fields, text[match.end():]


def first_heading(body: str) -> str:
    match = re.search(r"^# (.+)$", body, re.M)
    if not match:
        raise ValueError("post has no level-1 heading")
    return match.group(1).strip()


def format_date(iso: str, lang: str) -> str:
    y, m, d = (int(part) for part in iso.split("-"))
    if lang == "ko":
        return KO_MONTH.format(y=y, m=m, d=d)
    return f"{EN_MONTHS[m - 1]} {d}, {y}"


def meta_line(fields: dict[str, object], lang: str) -> str:
    date = str(fields.get("date", ""))
    tags = fields.get("tags") or []
    url = str(fields.get("url", ""))
    parts = [f'<time datetime="{html.escape(date)}">{format_date(date, lang)}</time>'] if date else []
    if tags:
        label = "태그" if lang == "ko" else "Tags"
        parts.append(f"{label}: " + ", ".join(html.escape(str(t)) for t in tags))
    if url:
        label = "원문" if lang == "ko" else "Original"
        parts.append(f'<a href="{html.escape(url)}">{label}</a>')
    if lang == "ko":
        parts.append("한국어 번역(원문에서 변경됨)")
    return '<p class="post-meta">' + " · ".join(parts) + "</p>\n"


def render_post(text: str, lang: str) -> tuple[str, str, str]:
    """(title, date, page) for one post: the meta line goes right after the H1."""
    fields, body = split_front_matter(text)
    title = first_heading(body)
    heading_end = re.search(r"^# .+\n", body, re.M).end()
    page = body[:heading_end] + "\n" + meta_line(fields, lang) + body[heading_end:]
    return title, str(fields.get("date", "")), page


def summary(entries: list[tuple[str, str, str]], lang: str) -> str:
    """SUMMARY.md: prefix chapter, one part per year (newest first), license suffix."""
    lines = [f"[{INDEX_TITLE[lang]}](index.md)", ""]
    year = None
    for slug, title, date in entries:
        this_year = date[:4] if date else "?"
        if this_year != year:
            if year is not None:
                lines.append("")
            year = this_year
            lines += [f"# {year}", ""]
        safe_title = title.replace("[", "\\[").replace("]", "\\]")
        lines.append(f"- [{safe_title}](posts/{slug}.md)")
    lines += ["", "---", "", f"[{LICENSE_TITLE[lang]}](license.md)", ""]
    return "\n".join(lines)


def book_toml(lang: str) -> str:
    return f"""\
[book]
title = {json.dumps(TITLES[lang], ensure_ascii=False)}
language = "{lang}"
src = "src"

[output.html]
additional-css = ["custom.css"]

[preprocessor.katex]
after = ["links"]
"""


def prepare(root: Path, out: Path, lang: str, manifest: Path | None = None) -> int:
    manifest = manifest or root / "manifest.json"
    posts = json.loads(manifest.read_text())["posts"]
    src = out / "src" / "posts"
    src.mkdir(parents=True, exist_ok=True)
    entries = []
    for post in posts:
        slug = post["slug"]
        text = (root / "posts" / f"{slug}.md").read_text()
        title, date, page = render_post(text, lang)
        (src / f"{slug}.md").write_text(page)
        entries.append((slug, title, date or post["date"]))
    entries.sort(key=lambda e: e[2], reverse=True)
    (out / "src" / "SUMMARY.md").write_text(summary(entries, lang))

    blurb = (root / "index.md").read_text()
    notice = KO_INDEX_NOTICE if lang == "ko" else EN_INDEX_NOTICE
    (out / "src" / "index.md").write_text(blurb.rstrip("\n") + "\n\n" + notice)

    license_text = (root / "license.md").read_text()
    if lang == "ko":
        license_text = KO_LICENSE_SUMMARY + license_text
    (out / "src" / "license.md").write_text(license_text)

    (out / "book.toml").write_text(book_toml(lang))
    (out / "custom.css").write_text(CUSTOM_CSS)
    return len(entries)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("root", type=Path)
    parser.add_argument("out", type=Path)
    parser.add_argument("--lang", choices=["en", "ko"], required=True)
    parser.add_argument("--manifest", type=Path, default=None)
    args = parser.parse_args()
    count = prepare(args.root, args.out, args.lang, args.manifest)
    print(f"Prepared {args.lang} book with {count} posts in {args.out}")


if __name__ == "__main__":
    main()
