#!/usr/bin/env python3
"""번역된 프런트매터로 책 지도(assets/generated/book-map.html)를 다시 만듭니다.

사용법: python3 scripts/book_map.py TREE [--check]

upstream/scripts/generate-book-map.mjs를 표준 라이브러리 Python으로 옮긴 것입니다.
TREE/_quarto-web.yml(오버레이의 한국어 부 제목)에서 부별 장 목록을, 각 장의
`# 제목 {#sec-…}`과 프런트매터 `summary.sentence`에서 제목·요약을 읽어 index.qmd가
포함하는 HTML 조각을 씁니다. 원문 생성기와 요소·클래스·속성 구조는 같되 링크와 삽화
경로는 루트 절대(`/chapters/…`)가 아니라 상대(`chapters/…`)로 씁니다 — 이 조각은
루트의 index.qmd에만 들어가므로 상대 경로로 충분하고 하위 경로 배포에서도 맞습니다.
부별 장 수(7/12/10/11/4)와 번호 장 44개가 어긋나면 실패합니다.

조립 트리의 파일은 원본을 가리키는 심볼릭 링크이므로 링크를 끊고 새 파일을 씁니다.
`--check`는 쓰지 않고 현재 파일과 비교만 합니다.
"""

import html
import re
import sys
from pathlib import Path

DEPARTMENTS = [
    {
        "manifestName": "당신의 코딩 커리어",
        "slug": "guides",
        "sequence": "01",
        "verb": "자리 잡기",
        "title": "가이드",
        "description": "발 디딜 곳을 찾고, 구직을 헤쳐 나가고, 첫 직무에서 시니어와 그 너머까지 가는 길을 봅니다.",
        "cover": "careers-README",
        "expected": 7,
    },
    {
        "manifestName": "원칙",
        "slug": "principles",
        "sequence": "02",
        "verb": "다지기",
        "title": "원칙",
        "description": "배우고, 만들고, 쓰고, 나누고, 다른 사람과 일하는 데 오래가는 기본값을 세웁니다.",
        "cover": "principles-README",
        "expected": 12,
    },
    {
        "manifestName": "전략",
        "slug": "strategies",
        "sequence": "03",
        "verb": "고르기",
        "title": "전략",
        "description": "지형을 읽고, 더 나은 커리어 베팅을 하고, 자신의 일을 둘러싼 비즈니스와 연결합니다.",
        "cover": "strategy-README",
        "expected": 10,
    },
    {
        "manifestName": "전술",
        "slug": "tactics",
        "sequence": "04",
        "verb": "움직이기",
        "title": "전술",
        "description": "협상, 글쓰기, 발표, 마케팅, 쓸모 있는 사이드 프로젝트로 방향을 움직임으로 바꿉니다.",
        "cover": "tactics-README",
        "expected": 11,
    },
    {
        "manifestName": "개념",
        "slug": "concepts",
        "sequence": "05",
        "verb": "넓히기",
        "title": "보너스 개념",
        "description": "공개적으로 배우기, 행운 만들기, 일을 복리로 쌓기에 관한 기억에 남는 모형 몇 가지를 챙깁니다.",
        "cover": "concepts-README",
        "expected": 4,
    },
]

PART = re.compile(r'^ {4}- part: "([^"]+)"$')
NESTED = re.compile(r"^ {8}- (chapters/[A-Za-z0-9-]+\.qmd)$")
TOP = re.compile(r"^ {4}- (chapters/[A-Za-z0-9-]+\.qmd)$")
TITLE = re.compile(r"^#\s+(.+?)(?:\s+\{[^}]+\})?\s*$", re.M)
FRONT_MATTER = re.compile(r"\A---\n(.*?)\n---(?:\n|\Z)", re.S)
SENTENCE = re.compile(r'^\s{2}sentence:\s*&chapter_summary\s+("(?:\\.|[^"\\])*")\s*$', re.M)


def escape_html(value: str) -> str:
    return html.escape(str(value), quote=True)


def read_manifest_groups(tree: Path) -> dict[str, list[str]]:
    yaml = (tree / "_quarto-web.yml").read_text(encoding="utf-8")
    groups = {d["manifestName"]: [] for d in DEPARTMENTS}
    current = None
    for line in yaml.split("\n"):
        part = PART.match(line)
        if part:
            current = part.group(1) if part.group(1) in groups else None
            continue
        nested = NESTED.match(line)
        if nested and current:
            groups[current].append(nested.group(1))
        top = TOP.match(line)
        if top and top.group(1) == "chapters/finale.qmd":
            groups["전술"].append(top.group(1))
    for d in DEPARTMENTS:
        actual = len(groups[d["manifestName"]])
        if actual != d["expected"]:
            raise SystemExit(f"Expected {d['expected']} {d['manifestName']} chapters, found {actual}")
    return groups


def unquote(raw: str) -> str:
    """YAML 큰따옴표 스칼라의 값. 원문 생성기는 JSON.parse를 쓰며, 파서가 되돌려 쓰는
    따옴표 값도 같은 이스케이프(\\" \\\\)만 씁니다."""
    body = raw[1:-1]
    return re.sub(r"\\(.)", lambda m: m.group(1), body)


def chapter_title(tree: Path, source_path: str) -> str:
    source = (tree / source_path).read_text(encoding="utf-8")
    match = TITLE.search(source)
    if not match:
        raise SystemExit(f"Missing chapter title: {source_path}")
    return match.group(1).strip()


def chapter_summary(tree: Path, source_path: str) -> str:
    source = (tree / source_path).read_text(encoding="utf-8")
    front = FRONT_MATTER.match(source)
    if not front:
        raise SystemExit(f"Missing summary front matter: {source_path}")
    sentence = SENTENCE.search(front.group(1))
    if not sentence:
        raise SystemExit(f"Invalid summary front matter: {source_path}")
    return unquote(sentence.group(1))


def route_for(source_path: str) -> str:
    return re.sub(r"\.qmd$", ".html", source_path)


def render(tree: Path) -> str:
    groups = read_manifest_groups(tree)
    chapter_number = 1
    sections = []
    for department in DEPARTMENTS:
        paths = groups[department["manifestName"]]
        start = chapter_number
        chapters = []
        for source_path in paths:
            title = chapter_title(tree, source_path)
            summary = chapter_summary(tree, source_path)
            number = chapter_number
            chapter_number += 1
            chapters.append(
                f"""      <li class="book-chapter-preview">
        <a href="{escape_html(route_for(source_path))}" aria-label="{number}장: {escape_html(title)}">
          <span class="book-chapter-preview__number" aria-hidden="true">{number:02d}</span>
          <span class="book-chapter-preview__copy">
            <strong>{escape_html(title)}</strong>
            <span>{escape_html(summary)}</span>
          </span>
          <span class="book-chapter-preview__arrow" aria-hidden="true">&#8599;</span>
        </a>
      </li>"""
            )
        art = f"assets/illustrations-magazine/{department['cover']}.webp"
        slug = department["slug"]
        sections.append(
            f"""  <section class="book-department book-department--{slug}" id="book-department-{slug}" aria-labelledby="book-department-{slug}-title">
    <div class="book-department__lead">
      <div class="book-department__art">
        <img src="{art}" alt="" width="1024" height="1024" loading="lazy" decoding="async">
      </div>
      <div class="book-department__intro">
        <span class="book-department__eyebrow">{department['sequence']} / {department['verb']} · {len(paths)}개 장</span>
        <h3 id="book-department-{slug}-title"><a href="{route_for(paths[0])}">{escape_html(department['title'])}</a></h3>
        <p>{escape_html(department['description'])}</p>
      </div>
    </div>
    <ol class="book-department__chapters" start="{start}">
{chr(10).join(chapters)}
    </ol>
  </section>"""
        )
    if chapter_number != 45:
        raise SystemExit(f"Expected 44 numbered chapters, found {chapter_number - 1}")
    index = "\n".join(
        f'  <a href="#book-department-{d["slug"]}"><span>{d["sequence"]}</span>{escape_html(d["title"])}</a>'
        for d in DEPARTMENTS
    )
    return f"""```{{=html}}
<!-- Generated by scripts/book_map.py (ported from upstream scripts/generate-book-map.mjs). Do not edit directly. -->
<nav class="book-map-index" aria-label="책의 부로 이동">
{index}
</nav>
<div class="book-map" aria-label="책의 44개 장 전체">
{chr(10).join(sections)}
</div>
```
"""


def main() -> int:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 1:
        print(__doc__, file=sys.stderr)
        return 2
    tree = Path(args[0])
    output = tree / "assets" / "generated" / "book-map.html"
    generated = render(tree)
    if "--check" in sys.argv:
        if output.read_text(encoding="utf-8") != generated:
            raise SystemExit(f"{output} is stale")
        print("Generated book map is current (5 departments, 44 chapters).")
        return 0
    if output.is_symlink():
        output.unlink()
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(generated, encoding="utf-8")
    print(f"Wrote {output}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
