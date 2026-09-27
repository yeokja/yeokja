#!/usr/bin/env python3
"""번역된 Verso 기술 용어 역할에 원문의 참조 키를 붙입니다.

Verso의 {tech}와 {deftech}는 표시 문구를 기본 참조 키로 씁니다. 표시 문구를
한국어로 옮기면 정의와 참조가 끊기므로, 아래 표에 따라 원문의 키를
`(key := "...")`로 명시합니다. 표에 없는 한국어 {tech} 표시 문구를 만나면
실패해, 새로 번역된 참조가 조용히 끊긴 링크가 되지 않게 합니다. 참조되지
않는 {deftech}는 키가 없어도 끊길 링크가 없으므로 알리기만 합니다.

사용법: add_verso_term_keys.py <book 디렉터리>
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

KEYS = {
    "명제-타입 대응": "propositions-as-types",
    "명제-타입 대응(propositions-as-types)": "propositions-as-types",
    "비서술적": "impredicative",
    "비서술적(impredicative)": "impredicative",
    "증명 무관성": "proof irrelevance",
    "증명 무관한": "proof irrelevance",
    "커리-하워드 동형": "Curry-Howard isomorphism",
    "커리-하워드 동형(Curry-Howard isomorphism)": "Curry-Howard isomorphism",
}

# 키가 없는 역할과 그 뒤의 표시 문구: [..], *..*, _.._
ROLE = re.compile(r"\{(tech|deftech)\}(\[[^\]\n]*\]|\*[^*\n]*\*|_[^_\n]*_)")
HANGUL = re.compile(r"[가-힣]")


def add_keys(text: str, path: str, unknown: list[str], unkeyed: list[str]) -> str:
    def replace(match: re.Match[str]) -> str:
        role, display = match.groups()
        words = display[1:-1]
        key = KEYS.get(words)
        if key is None:
            if HANGUL.search(words):
                (unknown if role == "tech" else unkeyed).append(f"{path}: {{{role}}}{display}")
            return match.group(0)
        return f'{{{role} (key := "{key}")}}{display}'

    return ROLE.sub(replace, text)


def main() -> int:
    book = Path(sys.argv[1])
    unknown: list[str] = []
    unkeyed: list[str] = []
    for path in sorted(book.rglob("*.lean")):
        text = path.read_text(encoding="utf-8")
        keyed = add_keys(text, str(path), unknown, unkeyed)
        if keyed != text:
            # 조립된 트리의 파일은 ko/나 upstream을 가리키는 링크이므로, 링크를
            # 끊고 실파일로 써야 원본을 건드리지 않습니다.
            path.unlink()
            path.write_text(keyed, encoding="utf-8")
    for entry in unkeyed:
        print(f"note: translated definition keeps its Korean key: {entry}", file=sys.stderr)
    for entry in unknown:
        print(f"no reference key for translated term: {entry}", file=sys.stderr)
    return 1 if unknown else 0


if __name__ == "__main__":
    sys.exit(main())
