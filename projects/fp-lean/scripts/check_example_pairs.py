#!/usr/bin/env python3
"""예제 모듈과 원고의 같은 문장이 같은 번역인지 확인합니다.

`anchorEqSteps` 블록은 예제 모듈(upstream/examples)의 등식 단계 근거를
원고(upstream/book)의 본문과 한 줄씩 비교해 렌더링하므로, 한쪽만 고치면
Lean 빌드가 출력 비교에서 실패합니다. 빌드 전에 싸게 걸러 냅니다.
"""

from __future__ import annotations

import json
import sys
from collections import defaultdict
from pathlib import Path


def translations(root: Path) -> dict[str, set[str]]:
    found: dict[str, set[str]] = defaultdict(set)
    for path in sorted(root.rglob("*.yeokja.json")):
        for segment in json.loads(path.read_text(encoding="utf-8"))["segments"]:
            if segment.get("translation") is not None:
                found[segment["source"]].add(segment["translation"])
    return found


def mismatches(state: Path) -> list[tuple[str, set[str], set[str]]]:
    examples = translations(state / "upstream" / "examples")
    book = translations(state / "upstream" / "book")
    return [
        (source, examples[source], book[source])
        for source in sorted(examples)
        if source in book and examples[source] != book[source]
    ]


def main() -> int:
    state = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("state")
    found = mismatches(state)
    for source, example, book in found:
        print(f"mismatch: {source!r}\n  examples: {sorted(example)}\n  book:     {sorted(book)}", file=sys.stderr)
    return 1 if found else 0


if __name__ == "__main__":
    sys.exit(main())
