#!/usr/bin/env python3
"""원문(영어) 빌드와 번역 빌드를 쪽 단위로 비교합니다.

사용법: python3 scripts/check_site.py ORIGINAL_SITE KOREAN_SITE

다음 중 하나라도 걸리면 실패합니다.
- 두 빌드의 HTML 쪽 목록이 다름.
- 번역 빌드에만 있는 깨진 로컬 링크·그림·스타일·앵커(원문 빌드에서도 깨진 것은 보고만
  합니다). 한국어 제목은 Hugo 자동 ID가 원문과 달라지므로 제목 앵커 링크가 여기서
  걸립니다.
- 본문(`<div class="prose">`)의 제목(h2~h6), 그림, 코드 블록, 표, 목록 항목, 인용,
  알림 상자(note shortcode) 개수나 목차 항목 수가 원문과 다름.
- 본문 그림의 alt에 한글이 없음(그림 설명을 옮기지 않음), 본문이 있는 쪽에 한글이 없음.
- 번역 빌드에 방문 통계, Microsoft 로고, 로고 삽입 스크립트가 남아 있음.
- 번역 빌드의 쪽(다른 주소로 넘기는 별칭 쪽 제외)에 `lang="ko"`나 비공식 번역 고지가 없음.
"""

import re
import sys
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit

BASE_URL = "https://yeokja.moreal.dev/forge-lab/"
BASE_PATH = "/forge-lab/"
FORBIDDEN = ("applicationinsights", "instrumentation-key", "microsoft-logo", "brand-wordmarks")
HANGUL = re.compile(r"[가-힣]")
COUNTED = ("h2", "h3", "h4", "h5", "h6", "img", "pre", "table", "li", "blockquote", "aside")


class Page(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.ids: set[str] = set()
        self.links: list[str] = []
        self.counts = {tag: 0 for tag in COUNTED}
        self.counts["toc"] = 0
        self.alts: list[str] = []
        self.divs = 0
        self.prose_at: int | None = None
        self.in_toc = 0
        self.skip = 0
        self.text: list[str] = []
        self.redirect = False

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        classes = (attrs.get("class") or "").split()
        if attrs.get("id"):
            self.ids.add(attrs["id"])
        if tag == "a" and attrs.get("href"):
            self.links.append(attrs["href"])
        if tag in ("img", "script") and attrs.get("src"):
            self.links.append(attrs["src"])
        if tag == "link" and attrs.get("href") and attrs.get("rel") in ("stylesheet", "preload", "icon", "canonical"):
            self.links.append(attrs["href"])
        if tag == "meta" and attrs.get("http-equiv") == "refresh":
            self.redirect = True
        if tag == "aside" and "toc" in classes:
            self.in_toc += 1
        elif self.in_toc and tag == "aside":
            self.in_toc += 1
        if self.in_toc and tag == "a":
            self.counts["toc"] += 1
        if tag == "div":
            self.divs += 1
            if "prose" in classes and self.prose_at is None:
                self.prose_at = self.divs
        if self.prose_at is not None:
            if tag in self.counts:
                self.counts[tag] += 1
            if tag == "img":
                self.alts.append(attrs.get("alt") or "")
            if tag in ("pre", "code", "script", "style"):
                self.skip += 1

    def handle_endtag(self, tag):
        if tag == "aside" and self.in_toc:
            self.in_toc -= 1
        if self.prose_at is not None and tag in ("pre", "code", "script", "style"):
            self.skip -= 1
        if tag == "div":
            if self.prose_at == self.divs:
                self.prose_at = None
            self.divs -= 1

    def handle_data(self, data):
        if self.prose_at is not None and self.skip <= 0:
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


def resolve(site: Path, rel: str, link: str) -> tuple[str, str] | None:
    """링크를 (사이트 안 파일 경로, 조각)으로. 바깥 링크면 None."""
    if link.startswith(BASE_URL):
        link = BASE_PATH + link[len(BASE_URL):]
    parts = urlsplit(link)
    if parts.scheme or parts.netloc or link.startswith(("mailto:", "javascript:")):
        return None
    path = unquote(parts.path)
    if path == "":
        target = rel
    elif path.startswith(BASE_PATH):
        target = path[len(BASE_PATH):]
    elif path.startswith("/"):
        target = "../" + path.lstrip("/")  # 사이트 밖(도메인 루트)을 가리키는 링크
    else:
        base = rel.rsplit("/", 1)[0] + "/" if "/" in rel else ""
        target = base + path
    stack: list[str] = []
    for piece in target.split("/"):
        if piece in ("", "."):
            continue
        if piece == "..":
            if stack and stack[-1] != "..":
                stack.pop()
            else:
                stack.append("..")
            continue
        stack.append(piece)
    target = "/".join(stack)
    if target == "" or path.endswith("/") or (site / target).is_dir():
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
            if target.startswith("..") or not (site / target).is_file():
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
        failures.append(f"page sets differ: original only {only_original}, korean only {only_korean}")

    original_broken = broken_links(original_site, original)
    korean_broken = broken_links(korean_site, korean)
    for rel, links in sorted(korean_broken.items()):
        new = sorted(links - original_broken.get(rel, set()))
        if new:
            failures.append(f"{rel}: broken links not broken in the original: {new}")

    for rel in sorted(set(original) & set(korean)):
        o_page, _ = original[rel]
        k_page, k_raw = korean[rel]
        for key, count in o_page.counts.items():
            if count != k_page.counts[key]:
                failures.append(f"{rel}: {key} {count} -> {k_page.counts[key]}")
        for alt in k_page.alts:
            if not HANGUL.search(alt):
                failures.append(f"{rel}: untranslated image alt {alt[:60]!r}")
        prose = "".join(k_page.text)
        if prose.strip() and not HANGUL.search(prose):
            failures.append(f"{rel}: no Korean in the article body")

    for rel, (page, raw) in korean.items():
        lowered = raw.lower()
        for word in FORBIDDEN:
            if word in lowered:
                failures.append(f"{rel}: {word} left in page")
        if page.redirect:
            continue
        if '<html lang="ko">' not in raw:
            failures.append(f'{rel}: missing <html lang="ko">')
        if 'class="translation-notice"' not in raw:
            failures.append(f"{rel}: missing the unofficial-translation notice")

    links_checked = sum(len(p.links) for p, _ in korean.values())
    print(f"pages: original {len(original)}, korean {len(korean)}")
    print(
        f"links checked: {links_checked}; broken in original {sum(map(len, original_broken.values()))},"
        f" in korean {sum(map(len, korean_broken.values()))}"
    )
    for rel in sorted(set(original) & set(korean)):
        counts = original[rel][0].counts
        if any(counts.values()):
            print(f"  {rel}: " + ", ".join(f"{k} {v}" for k, v in counts.items() if v))
    for failure in failures:
        print("FAIL", failure)
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
