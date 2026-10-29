#!/usr/bin/env python3
"""영어 원문 빌드와 한국어 빌드를 쪽 단위로 비교합니다.

사용법: python3 scripts/check.py ORIGINAL_BOOK KOREAN_BOOK

다음 중 하나라도 걸리면 실패합니다.
- 두 빌드의 HTML 쪽 목록이 다름.
- 쪽마다 본문(main)의 제목(h1~h6) 수가 다름(오디오북 쪽은 정적 목록으로 바뀌어 제외).
- 원문 쪽의 절 앵커(`sec-…`)와 명시 id가 번역 쪽에 없음. 원문이 영어 제목에서 자동으로
  만든 id는 번역에서 한글 id가 되므로 보지 않습니다.
- 번역 빌드의 로컬 링크(href·src)가 가리키는 파일이나 `#앵커`가 없음. 원문 빌드에서도
  깨진 것은 원문 결함으로 보고만 합니다.
- 풀리지 않은 교차 참조(`?@sec-`)가 본문에 남음.
- 권리가 확인되지 않은 그림(assets/rights.json의 ARR·unresolved)이 산출물에 있음.
- 본문 문단·목록 항목 중 한글이 없고 라틴 문자가 40자 이상인 것이 허용 목록 밖에 있음.
  허용: URL이 든 항목(참고 문헌), 코드, 오디오북 목록(영어 원제), 인용 출처 줄.
"""

import json
import re
import sys
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

HEADING_EXEMPT = {"audiobook.html", "reading-trail.html"}
RESTRICTED_CLASSES = {"third-party-permission-required", "unresolved"}
HANGUL = re.compile(r"[가-힣]")
LATIN = re.compile(r"[A-Za-z]")
# (쪽, 문단 머리) — 영어로 남겨 두는 것이 맞는 문단.
ALLOW_ENGLISH = {
    ("chapters/acknowledgements.html", "Nat Sharpe"),
}


class Page(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: set[str] = set()
        self.links: list[str] = []
        self.headings = 0
        self.in_main = 0
        self.skip = 0
        self.block: list[str] | None = None
        self.block_tag = ""
        self.block_class = ""
        self.blocks: list[tuple[str, str, str]] = []  # (tag, class, text)
        self.text: list[str] = []

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if attrs.get("id"):
            self.ids.add(attrs["id"])
        if tag == "a" and attrs.get("href"):
            self.links.append(attrs["href"])
        if tag in ("img", "script", "link") and (attrs.get("src") or attrs.get("href")):
            self.links.append(attrs.get("src") or attrs.get("href"))
        if tag == "main":
            self.in_main += 1
            return
        if not self.in_main:
            return
        if re.fullmatch(r"h[1-6]", tag):
            self.headings += 1
        if tag in ("pre", "code", "script", "style", "nav"):
            self.skip += 1
        if tag in ("p", "li", "h1", "h2", "h3", "h4", "h5", "h6", "blockquote") and self.block is None:
            self.block = []
            self.block_tag = tag
            self.block_class = attrs.get("class") or ""

    def handle_endtag(self, tag):
        if tag == "main":
            self.in_main -= 1
            return
        if not self.in_main:
            return
        if tag in ("pre", "code", "script", "style", "nav"):
            self.skip -= 1
        if self.block is not None and tag == self.block_tag:
            self.blocks.append((self.block_tag, self.block_class, "".join(self.block)))
            self.block = None

    def handle_data(self, data):
        if self.in_main:
            self.text.append(data)
            if self.block is not None and self.skip <= 0:
                self.block.append(data)


def parse(path: Path) -> Page:
    page = Page()
    page.feed(path.read_text(encoding="utf-8", errors="replace"))
    return page


def pages(book: Path) -> dict[str, Page]:
    # 원문 빌드는 include-in-header용 HTML 조각(assets/reader/*.html)도 복사하므로 쪽이 아닌 그것은 뺍니다.
    return {p.relative_to(book).as_posix(): parse(p) for p in sorted(book.rglob("*.html"))
            if "site_libs" not in p.parts and "assets" not in p.parts}


def broken_links(book: Path, site: dict[str, Page]) -> set[tuple[str, str]]:
    broken = set()
    for name, page in site.items():
        base = (book / name).parent
        for link in page.links:
            parts = urlsplit(link)
            if parts.scheme or parts.netloc or link.startswith(("mailto:", "tel:", "javascript:")):
                continue
            target_path = unquote(parts.path)
            if target_path.startswith("/"):
                broken.add((name, link))
                continue
            target = book / name if not target_path else (base / target_path)
            if not target.exists():
                broken.add((name, link))
                continue
            if parts.fragment:
                target_name = target.resolve().relative_to(book.resolve()).as_posix() if target.is_file() else None
                if target_name and target_name.endswith(".html") and target_name in site:
                    if unquote(parts.fragment) not in site[target_name].ids:
                        broken.add((name, link))
    return broken


def english_blocks(site: dict[str, Page]) -> list[tuple[str, str]]:
    found = []
    for name, page in site.items():
        for tag, cls, text in page.blocks:
            text = " ".join(text.split())
            if HANGUL.search(text) or len(LATIN.findall(text)) < 40:
                continue
            if "http://" in text or "https://" in text or "www." in text:
                continue
            if name == "audiobook.html" and tag == "li":
                continue
            if text.startswith(("—", "–", "- ")):
                continue
            if any(name == p and text.startswith(head) for p, head in ALLOW_ENGLISH):
                continue
            found.append((name, text[:90]))
    return found


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    original_book, korean_book = Path(sys.argv[1]), Path(sys.argv[2])
    original, korean = pages(original_book), pages(korean_book)
    failures: list[str] = []

    only_original = sorted(set(original) - set(korean))
    only_korean = sorted(set(korean) - set(original))
    if only_original or only_korean:
        failures.append(f"page set differs: missing {only_original}, extra {only_korean}")

    for name in sorted(set(original) & set(korean)):
        if name in HEADING_EXEMPT:
            continue
        if original[name].headings != korean[name].headings:
            failures.append(f"{name}: {original[name].headings} headings in original, {korean[name].headings} in translation")
        missing = sorted(i for i in original[name].ids if i.startswith("sec-") and i not in korean[name].ids)
        if missing:
            failures.append(f"{name}: missing anchors {missing[:5]}")

    original_broken = broken_links(original_book, original)
    korean_broken = broken_links(korean_book, korean)
    new_broken = sorted(korean_broken - original_broken)
    if new_broken:
        failures.append(f"{len(new_broken)} broken local links in translation: {new_broken[:10]}")
    if original_broken:
        print(f"note: {len(original_broken)} local links are broken in the original build too (reported only)")

    for name, page in korean.items():
        if "?@sec-" in "".join(page.text):
            failures.append(f"{name}: unresolved cross-reference (?@sec-)")

    rights = json.loads((original_book.parent / "original-source" / "assets" / "rights.json").read_text(encoding="utf-8")) \
        if (original_book.parent / "original-source" / "assets" / "rights.json").exists() else {}
    shipped = [p for p, e in rights.items() if e.get("class") in RESTRICTED_CLASSES and (korean_book / p).exists()]
    if shipped:
        failures.append(f"restricted images shipped: {shipped}")
    if not rights:
        print("note: rights.json not found next to the original build; restricted-image check skipped")

    english = english_blocks(korean)
    if english:
        failures.append(f"{len(english)} untranslated paragraphs: " + "; ".join(f"{n}: {t}" for n, t in english[:15]))

    print(f"check: {len(korean)} pages, {sum(p.headings for p in korean.values())} headings, "
          f"{sum(len(p.links) for p in korean.values())} links checked")
    if failures:
        for failure in failures:
            print(f"FAIL: {failure}")
        return 1
    print("check: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
