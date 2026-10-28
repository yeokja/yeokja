#!/usr/bin/env python3
"""원문(러시아어) 빌드와 번역 빌드를 쪽 단위로 비교합니다.

사용법: python3 scripts/check_site.py ORIGINAL_SITE KOREAN_SITE

다음 중 하나라도 걸리면 실패합니다.
- 두 빌드의 HTML 쪽 목록이 다름(번역 누락이나 .ru.md 이름 바꾸기 실수).
- 번역 빌드에만 있는 깨진 로컬 링크·그림·앵커(원문 빌드에서도 깨진 것은 원문 결함으로
  보고만 합니다). 한국어 제목은 Hugo 자동 ID가 원문과 달라지므로 제목 앵커 링크가
  여기서 걸립니다.
- 본문(article)의 제목(h2~h6), 그림, 코드 블록 개수나 수식(`$…$`, `$$…$$`)의 종류가 원문과 다름.
- 방문 통계 스크립트(googletagmanager, mc.yandex)가 남아 있음.

본문에 키릴 문자가 남은 쪽은 실패시키지 않고 개수만 보고합니다(코드 주석, 저자 이름,
수식 속 \\text{…}, 문제 이름 등은 원문 그대로 둡니다).
"""

import re
import sys
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

BASE = "https://yeokja.moreal.dev/algorithmica/"
TRACKERS = ("googletagmanager", "mc.yandex")

# 수식 집합이 원문과 다를 수밖에 없는 쪽과 그 이유. 비교 결과는 보고만 합니다.
KNOWN_MATH = {
    # 본문에 글자 그대로의 `$`(«$», 1000\$, `mississippi$`)가 있어 `$` 짝이 번역문의
    # 다른 글자를 끼고 맺어집니다. 실제 수식은 바뀌지 않았습니다.
    "cs/string-structures/suffix-array/index.html": "literal $ in prose",
    "math/probability/index.html": "literal $ in prose",
    # 원문 수식이 줄머리 `- 1`, `- b`, `+ 2`에서 끊겨 Markdown 목록으로 읽힙니다.
    # 번역은 문단을 한 줄로 이어 수식이 바르게 그려집니다.
    "cs/combinatorial-objects/generation/index.html": "original math broken by a list marker",
    "math/number-theory/модульная_арифметика/index.html": "original math broken by a list marker",
    "math/number-theory/простое_число/index.html": "original math broken by a list marker",
}


class Page(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: set[str] = set()
        self.links: list[str] = []
        self.counts = {"heading": 0, "img": 0, "pre": 0}
        self.depth_article = 0
        self.skip = 0
        self.text: list[str] = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs and attrs["id"]:
            self.ids.add(attrs["id"])
        if tag == "a" and attrs.get("href"):
            self.links.append(attrs["href"])
        if tag in ("img", "script", "link") and attrs.get("src"):
            self.links.append(attrs["src"])
        if tag == "article":
            self.depth_article += 1
        if self.depth_article:
            if re.fullmatch(r"h[2-6]", tag):
                self.counts["heading"] += 1
            elif tag in ("img", "pre"):
                self.counts[tag] += 1
            if tag in ("pre", "code", "script", "style"):
                self.skip += 1

    def handle_endtag(self, tag):
        if tag == "article":
            self.depth_article -= 1
        if self.depth_article and tag in ("pre", "code", "script", "style"):
            self.skip -= 1

    def handle_data(self, data):
        if self.depth_article:
            if self.skip <= 0:
                self.text.append(data)


def load(site: Path) -> dict[str, tuple[Page, str]]:
    pages = {}
    for path in sorted(site.rglob("*.html")):
        rel = path.relative_to(site).as_posix()
        if rel == "404.html":
            continue
        raw = path.read_text(encoding="utf-8")
        page = Page()
        page.feed(raw)
        pages[rel] = (page, raw)
    return pages


def article_html(raw: str) -> str:
    match = re.search(r"<article>(.*)</article>", raw, re.S)
    return match.group(1) if match else ""


def math_spans(html: str) -> set[str]:
    """본문의 서로 다른 수식(`$$…$$`, `$…$`) 집합.

    - 번역은 같은 변수를 한 번 더 쓸 수 있으므로 개수가 아니라 종류를 비교합니다.
    - 원문 문단 속 줄바꿈은 번역에서 한 줄로 합쳐지므로 공백을 하나로 접습니다.
    - HTML 태그를 건너 짝지어진 구간(글자 그대로의 `$`, 원문에서 Markdown 목록으로
      잘못 읽힌 수식)은 수식이 아니므로 뺍니다.
    """
    html = re.sub(r"<pre.*?</pre>|<code.*?</code>", "", html, flags=re.S)
    spans = set(re.findall(r"\$\$.*?\$\$", html, flags=re.S))
    rest = re.sub(r"\$\$.*?\$\$", "", html, flags=re.S)
    spans |= set(re.findall(r"\$[^$]*\$", rest))
    return {re.sub(r"\s+", " ", span) for span in spans if "<" not in span}


def resolve(site: Path, rel: str, link: str) -> tuple[str, str] | None:
    """링크를 (사이트 안 파일 경로, 조각)으로. 바깥 링크면 None."""
    if link.startswith(BASE):
        link = "/" + link[len(BASE):]
    parts = urlsplit(link)
    if parts.scheme or parts.netloc or link.startswith(("mailto:", "javascript:")):
        return None
    path = unquote(parts.path)
    if path == "":
        target = rel
    elif path.startswith("/"):
        target = path.lstrip("/")
    else:
        base = rel.rsplit("/", 1)[0] + "/" if "/" in rel else ""
        target = base + path
    # 정규화
    stack: list[str] = []
    for piece in target.split("/"):
        if piece in ("", "."):
            continue
        if piece == "..":
            if stack:
                stack.pop()
            continue
        stack.append(piece)
    target = "/".join(stack)
    if target == "" or (site / target).is_dir():
        target = (target + "/index.html").lstrip("/")
    return target, unquote(parts.fragment)


def broken_links(site: Path, pages) -> dict[str, set[tuple[str, str]]]:
    broken: dict[str, set[tuple[str, str]]] = {}
    for rel, (page, _) in pages.items():
        for link in page.links:
            resolved = resolve(site, rel, link)
            if resolved is None:
                continue
            target, fragment = resolved
            if not (site / target).is_file():
                broken.setdefault(rel, set()).add((target, fragment))
                continue
            if fragment and target in pages and fragment not in pages[target][0].ids:
                broken.setdefault(rel, set()).add((target, fragment))
    return broken


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    original_site, korean_site = Path(sys.argv[1]), Path(sys.argv[2])
    original, korean = load(original_site), load(korean_site)
    failures: list[str] = []

    only_original = sorted(set(original) - set(korean))
    only_korean = sorted(set(korean) - set(original))
    if only_original or only_korean:
        failures.append(f"page sets differ: original only {only_original[:10]}, korean only {only_korean[:10]}")

    original_broken = broken_links(original_site, original)
    korean_broken = broken_links(korean_site, korean)
    new_broken = {
        rel: sorted(links - original_broken.get(rel, set()))
        for rel, links in korean_broken.items()
        if links - original_broken.get(rel, set())
    }
    for rel, links in sorted(new_broken.items()):
        failures.append(f"{rel}: broken links not broken in the original: {links}")

    mismatched = []
    known: list[str] = []
    cyrillic_pages = 0
    for rel in sorted(set(original) & set(korean)):
        o_page, o_raw = original[rel]
        k_page, k_raw = korean[rel]
        for key in o_page.counts:
            if o_page.counts[key] != k_page.counts[key]:
                mismatched.append(f"{rel}: {key} {o_page.counts[key]} -> {k_page.counts[key]}")
        o_math, k_math = math_spans(article_html(o_raw)), math_spans(article_html(k_raw))
        if o_math != k_math and rel in KNOWN_MATH:
            known.append(f"{rel}: {KNOWN_MATH[rel]}")
        elif o_math != k_math:
            mismatched.append(
                f"{rel}: math missing {sorted(o_math - k_math)[:5]}, added {sorted(k_math - o_math)[:5]}"
            )
        if any(t in k_raw for t in TRACKERS):
            failures.append(f"{rel}: analytics script left in page")
        if re.search(r"[А-Яа-яЁё]", "".join(k_page.text)):
            cyrillic_pages += 1
    failures.extend(mismatched)

    links_checked = sum(len(p.links) for p, _ in korean.values())
    print(f"pages: original {len(original)}, korean {len(korean)}")
    print(f"links checked: {links_checked}; broken in original {sum(map(len, original_broken.values()))},"
          f" in korean {sum(map(len, korean_broken.values()))}, new in korean {sum(map(len, new_broken.values()))}")
    print(f"pages with Cyrillic left in article text (code excluded): {cyrillic_pages}")
    for note in known:
        print("known math difference", note)
    for failure in failures:
        print("FAIL", failure)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
