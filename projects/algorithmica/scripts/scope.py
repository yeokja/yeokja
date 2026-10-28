#!/usr/bin/env python3
"""번역 범위(yeokja.toml `[[sources]].exclude`)를 원문에서 계산하고 검사합니다.

러시아어판(`upstream/content/russian`)에서 다음을 범위 밖으로 둡니다.

- front matter에 `draft: true`가 있는 문서. Hugo는 `--buildDrafts` 없이 초안을
  내놓지 않으므로 원문 사이트에도 보이지 않습니다.
- `dl/aux/`: reveal-hugo 슬라이드 묶음(`outputs: [Reveal]`)과 그 섹션 머리말.
  번역 빌드는 reveal-hugo 테마를 쓰지 않습니다.

exclude 목록을 yeokja.toml에 고정해 두면 `yeokja status --check upstream/content/russian`
이 정확히 번역 범위만 봅니다. 원문을 올려 초안 여부가 바뀌면 `--check`가 실패하므로
`print`의 출력으로 목록을 바꿉니다.

사용법:
  python3 scripts/scope.py print   # exclude 배열을 TOML로 출력
  python3 scripts/scope.py check   # yeokja.toml의 exclude가 계산값과 같은지 검사
"""

import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "upstream" / "content" / "russian"
EXCLUDED_DIRS = ["dl/aux/**"]


def is_draft(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    if not text.startswith("---\n"):
        return False
    end = text.find("\n---", 4)
    front = text[4:end] if end != -1 else ""
    return re.search(r"^draft:\s*true\s*$", front, re.M) is not None


def computed() -> list[str]:
    drafts = []
    for path in sorted(SOURCE.rglob("*.md")):
        rel = path.relative_to(SOURCE).as_posix()
        if rel.startswith("dl/aux/"):
            continue
        if is_draft(path):
            drafts.append(rel)
    return EXCLUDED_DIRS + drafts


def configured() -> list[str]:
    config = tomllib.loads((ROOT / "yeokja.toml").read_text(encoding="utf-8"))
    for source in config["sources"]:
        if source["path"] == "upstream/content/russian":
            return list(source.get("exclude", []))
    raise SystemExit("yeokja.toml: no source for upstream/content/russian")


def main() -> int:
    mode = sys.argv[1] if len(sys.argv) > 1 else "check"
    want = computed()
    if mode == "print":
        print("exclude = [")
        for item in want:
            print(f'  "{item}",')
        print("]")
        return 0
    if mode == "check":
        have = configured()
        if have != want:
            missing = [x for x in want if x not in have]
            extra = [x for x in have if x not in want]
            print(f"exclude out of date: missing {missing}, extra {extra}", file=sys.stderr)
            return 1
        print(f"exclude matches upstream: {len(want) - len(EXCLUDED_DIRS)} drafts + {EXCLUDED_DIRS}")
        return 0
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main())
