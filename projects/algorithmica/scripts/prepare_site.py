#!/usr/bin/env python3
"""조립 트리를 Hugo 빌드에 맞게 손질합니다.

사용법:
  python3 scripts/prepare_site.py korean TREE    # 번역 사이트(content/korean)
  python3 scripts/prepare_site.py russian TREE   # 비교용 원문 사이트(content/russian)

두 모드 공통:
- 영어판(content/english)과 reveal-hugo 테마를 지웁니다. 영어판은 이용 허락이 없어
  번역 범위 밖이고, 슬라이드 테마는 번역 사이트에서 쓰지 않습니다.
- reveal 슬라이드 묶음(dl/aux)을 지웁니다(scripts/scope.py 참고).
- front matter의 `published:` 줄과 `date: {}` 줄을 지웁니다. 원문이 쓰던 Hugo 0.96은
  이를 받아들였지만 nixpkgs의 Hugo는 날짜로 읽다가 빌드를 멈춥니다.

korean 모드:
- content/russian의 그림 등 Markdown이 아닌 파일을 content/korean으로 복사합니다.
- 번역 범위 밖의 초안(draft: true)을 원문 그대로 복사합니다. Hugo는 초안을 내놓지
  않지만, 초안인 _index.md가 없으면 그 섹션을 기본 제목으로 새로 만들어 버리므로
  섹션 구조를 원문 사이트와 같게 두려면 필요합니다.
- 번역 범위의 모든 문서에 번역본이 있는지 확인합니다.
- `*.ru.md`를 `*.md`로 바꿉니다(한국어 한 언어 사이트에서 `.ru` 접미사는 다른
  언어로 읽혀 문서가 사라집니다). URL은 바뀌지 않습니다.
- front matter `part:`를 PARTS 사전으로 옮기고(파서는 title만 번역 대상으로 냅니다),
  원문 경로를 `originalPath:`로 덧붙여 원문 보기 버튼이 씁니다.
- content/russian을 지우고, 남은 심볼릭 링크를 실제 파일로 바꿉니다(Hugo는 링크인
  파일을 건너뜁니다).
"""

import re
import shutil
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from scope import is_draft  # noqa: E402

# 원문 front matter `part:`(사이드바·목차의 부 제목) → 한국어
PARTS = {
    "Анализ алгоритмов": "알고리즘 분석",
    "Структуры данных": "자료 구조",
    "Графы": "그래프",
    "Динамическое программирование": "동적 계획법",
    "Строки": "문자열",
    "Геометрия": "기하",
    "Математика": "수학",
    "Разное": "기타",
}

FRONT_MATTER = re.compile(r"\A---\n(.*?\n)---\n", re.S)


def write(path: Path, text: str) -> None:
    """조립 트리의 파일은 upstream·ko·overlay를 가리키는 심볼릭 링크이므로, 링크를
    끊고 새 파일로 씁니다(링크를 따라 쓰면 원본이 바뀝니다)."""
    if path.is_symlink():
        path.unlink()
    path.write_text(text, encoding="utf-8")


def edit_front_matter(path: Path, original_path: str | None) -> None:
    text = path.read_text(encoding="utf-8")
    match = FRONT_MATTER.match(text)
    if not match:
        if original_path is None:
            return
        # front matter가 없는 옛 위키 문서도 원문 보기 버튼을 갖게 합니다.
        write(path, f"---\noriginalPath: {original_path}\n---\n\n{text}")
        return
    lines = []
    for line in match.group(1).splitlines():
        if line.startswith("published:") or line.strip() == "date: {}":
            continue
        if original_path is not None and line.startswith("part:"):
            value = line[len("part:"):].strip()
            if value not in PARTS:
                raise SystemExit(f"{path}: no Korean for part {value!r}; add it to PARTS")
            line = f"part: {PARTS[value]}"
        lines.append(line)
    if original_path is not None:
        lines.append(f"originalPath: {original_path}")
    write(path, "---\n" + "\n".join(lines) + "\n---\n" + text[match.end():])


def materialize(tree: Path) -> None:
    """Hugo는 심볼릭 링크인 파일(테마·본문)을 읽지 않으므로 조립 트리의 링크를 실제
    파일로 바꿉니다."""
    for path in sorted(tree.rglob("*")):
        if path.is_symlink() and path.is_file():
            data = path.read_bytes()
            path.unlink()
            path.write_bytes(data)


def common(tree: Path) -> None:
    shutil.rmtree(tree / "content" / "english", ignore_errors=True)
    shutil.rmtree(tree / "themes" / "reveal-hugo", ignore_errors=True)
    shutil.rmtree(tree / "content" / "russian" / "dl" / "aux", ignore_errors=True)


def russian(tree: Path) -> None:
    common(tree)
    for path in (tree / "content" / "russian").rglob("*.md"):
        edit_front_matter(path, None)


def korean(tree: Path) -> None:
    common(tree)
    ru = tree / "content" / "russian"
    ko = tree / "content" / "korean"
    if not ko.is_dir():
        raise SystemExit(f"{ko} is missing; run `yeokja translate` first")
    missing = []
    for src in sorted(ru.rglob("*")):
        if not src.is_file():
            continue
        rel = src.relative_to(ru)
        dst = ko / rel
        if src.suffix != ".md":
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)
        elif is_draft(src):
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)
        elif not dst.is_file():
            missing.append(rel.as_posix())
    if missing:
        raise SystemExit(f"no translation for {len(missing)} documents: {missing[:10]}")
    for path in sorted(ko.rglob("*.md")):
        rel = path.relative_to(ko).as_posix()
        if not (ru / rel).is_file():
            raise SystemExit(f"{path}: translated document has no original")
        draft = is_draft(path)
        edit_front_matter(path, None if draft else rel)
        if path.name.endswith(".ru.md"):
            path.rename(path.with_name(path.name[: -len(".ru.md")] + ".md"))
    shutil.rmtree(ru)
    materialize(tree)


def main() -> int:
    if len(sys.argv) != 3 or sys.argv[1] not in ("korean", "russian"):
        print(__doc__, file=sys.stderr)
        return 2
    tree = Path(sys.argv[2])
    korean(tree) if sys.argv[1] == "korean" else russian(tree)
    return 0


if __name__ == "__main__":
    sys.exit(main())
