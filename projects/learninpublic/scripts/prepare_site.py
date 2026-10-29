#!/usr/bin/env python3
"""조립 트리의 번역된 문서를 하위 경로·정적 사이트에 맞춥니다.

사용법: python3 scripts/prepare_site.py TREE

- 그림 권리: 원문 assets/rights.json에서 class가 third-party-permission-required(ARR)
  또는 unresolved인 파일은 싣지 않습니다. 본문의 `![캡션](../assets/images/…){…}`를
  들여쓰기를 유지한 채 원문 사이트 링크가 든 안내 스팬(.figure-omitted)으로 바꾸고,
  index.qmd의 표지 `<img>`를 안내 스팬(.book-cover-omitted)으로 바꾸며, 그림 파일을
  트리에서 지웁니다. 개방형 라이선스 그림과 저자 소유 그림은 속성(fig-alt)까지 그대로
  둡니다.
- audiobook.qmd: 스크립트가 채우던 `::: {#audiobook-playlist}`를 audiobook/manifest.json
  에서 만든 정적 목록(원문 사이트의 MP3 링크, 번역된 장 링크)으로, 오프라인 저장
  관리자 div를 한 줄 안내로 바꿉니다.
- reading-trail.qmd: 앱 루트 div 앞에 읽기 기록 노트가 원문 사이트에서만 동작한다는
  안내를 둡니다.
- 렌더링에 쓰지 않는 디렉터리(docs, publishing, tests, worker, 오디오북 자막·챕터 데이터)를
  지우고, 남은 심볼릭 링크를 실제 파일로 바꿉니다. Quarto는 링크인 자원(assets/fonts 등)의
  실제 경로를 프로젝트 밖으로 풀어 복사에 실패합니다.

조립 트리의 파일은 원본을 가리키는 심볼릭 링크이므로 링크를 끊고 새 파일을 씁니다.
"""

import json
import posixpath
import re
import shutil
import sys
from pathlib import Path

ORIGIN = "https://learninpublic.org"
RESTRICTED_CLASSES = {"third-party-permission-required", "unresolved"}

# Pandoc 그림: 한 문단을 이루는 `![캡션](경로){속성}`. 캡션에는 인라인 마크업이 올 수 있고
# 대괄호는 오지 않습니다. 앞의 들여쓰기(목록 항목 안)는 유지합니다.
FIGURE = re.compile(r"^(?P<indent>[ \t]*)!\[(?P<caption>[^\]\n]*)\]\((?P<src>[^)\s]+)\)(?P<attrs>\{[^}\n]*\})?[ \t]*$", re.M)
IMG_TAG = re.compile(r'<img\b[^>]*\bsrc="(?P<src>[^"]+)"[^>]*/?>')
PLAYLIST = re.compile(r"^::: \{#audiobook-playlist\}\n.*?\n:::$", re.S | re.M)
OFFLINE = re.compile(r"^::: \{#audiobook-offline-manager[^}]*\}\n.*?\n:::$", re.S | re.M)
TRAIL_ROOT = re.compile(r"^:{4,} \{\.reading-trail-app[^}]*\}$", re.M)


def write(path: Path, text: str) -> None:
    if path.is_symlink():
        path.unlink()
    path.write_text(text, encoding="utf-8")


def restricted_images(tree: Path) -> set[str]:
    rights = json.loads((tree / "assets" / "rights.json").read_text(encoding="utf-8"))
    return {path for path, entry in rights.items() if entry.get("class") in RESTRICTED_CLASSES}


def resolve(document: Path, tree: Path, src: str) -> str:
    """문서 안의 상대 경로를 트리 기준 경로로. 트리의 파일은 심볼릭 링크이므로 링크를
    따라가지 않고 어휘적으로만 정규화합니다."""
    base = document.relative_to(tree).parent.as_posix()
    return posixpath.normpath(posixpath.join(base, src))


def figure_note(caption: str, path: str) -> str:
    url = f"{ORIGIN}/{path}"
    title = f"그림 「{caption.strip()}」은" if caption.strip() else "이 그림은"
    return (
        f"[{title} 저작권자의 별도 허락이 필요하거나 출처가 확인되지 않아 번역판에 싣지 않습니다. "
        f"[원문 사이트에서 그림 보기]({url})]{{.figure-omitted}}"
    )


def cover_note(path: str) -> str:
    url = f"{ORIGIN}/{path}"
    return (
        '<span class="book-cover book-cover-omitted">표지 그림은 권리가 확인되지 않아 번역판에 싣지 않습니다. '
        f'<a href="{url}">원문 사이트에서 보기</a></span>'
    )


def replace_images(tree: Path, restricted: set[str]) -> int:
    replaced = 0
    for document in sorted(list((tree / "chapters").glob("*.qmd")) + list(tree.glob("*.qmd"))):
        text = document.read_text(encoding="utf-8")

        def figure(match: re.Match) -> str:
            nonlocal replaced
            path = resolve(document, tree, match.group("src"))
            if path not in restricted:
                return match.group(0)
            replaced += 1
            return match.group("indent") + figure_note(match.group("caption"), path)

        def img(match: re.Match) -> str:
            nonlocal replaced
            path = resolve(document, tree, match.group("src"))
            if path not in restricted:
                return match.group(0)
            replaced += 1
            return cover_note(path)

        new = FIGURE.sub(figure, text)
        new = IMG_TAG.sub(img, new)
        for path in restricted:
            # 안내문 속 원문 사이트 URL(learninpublic.org/assets/…)은 참조가 아닙니다.
            if re.search(r"(?<!learninpublic\.org/)" + re.escape(path), new):
                raise SystemExit(f"{document}: still references restricted image {path}")
        if new != text:
            write(document, new)
    return replaced


def remove_restricted_files(tree: Path, restricted: set[str]) -> None:
    for path in sorted(restricted):
        file = tree / path
        if file.is_symlink() or file.exists():
            file.unlink()


def audiobook_list(tree: Path) -> str:
    manifest = json.loads((tree / "audiobook" / "manifest.json").read_text(encoding="utf-8"))
    tracks = sorted(manifest["tracks"], key=lambda t: t["sequence"])
    lines = ["::: {#audiobook-playlist .audiobook-static-list}", ""]
    for track in tracks:
        page = track["page"].lstrip("/")
        source = tree / re.sub(r"\.html$", ".qmd", page)
        chapter = f" · [번역된 장 읽기]({page})" if source.is_file() else ""
        lines.append(
            f"{track['sequence']}. [{track['displayTitle']}]({ORIGIN}{track['audio']}) "
            f"({track['duration']}, MP3, 영어){chapter}"
        )
    lines += ["", ":::"]
    return "\n".join(lines)


def prepare_audiobook(tree: Path) -> None:
    document = tree / "audiobook.qmd"
    text = document.read_text(encoding="utf-8")
    new, n = PLAYLIST.subn(lambda m: audiobook_list(tree), text, count=1)
    if n != 1:
        raise SystemExit(f"{document}: playlist div not found")
    note = (
        "::: {#audiobook-offline-manager .audiobook-offline-manager}\n"
        "브라우저에 챕터를 저장하는 오프라인 재생은 [원문 사이트의 오디오북 페이지]"
        f"({ORIGIN}/audiobook.html)에서만 됩니다. 위 목록의 MP3 링크로 파일을 직접 내려받을 수 있습니다.\n"
        ":::"
    )
    new, n = OFFLINE.subn(lambda m: note, new, count=1)
    if n != 1:
        raise SystemExit(f"{document}: offline manager div not found")
    write(document, new)


def prepare_reading_trail(tree: Path) -> None:
    document = tree / "reading-trail.qmd"
    text = document.read_text(encoding="utf-8")
    note = (
        "::: {.callout-note appearance=\"simple\"}\n"
        "읽기 기록 노트(책갈피·강조·메모)는 원문 사이트의 리더 스크립트가 제공하는 기능이라 번역판에서는 "
        f"동작하지 않습니다. [원문 사이트의 My Reading Trail]({ORIGIN}/reading-trail.html)에서 쓸 수 있습니다.\n"
        ":::\n\n"
    )
    new, n = TRAIL_ROOT.subn(lambda m: note + m.group(0), text, count=1)
    if n != 1:
        raise SystemExit(f"{document}: reading trail root div not found")
    write(document, new)


UNUSED_DIRS = ["docs", "publishing", "tests", "worker", "audiobook/transcripts", "audiobook/chapters"]


def materialize(tree: Path) -> None:
    for name in UNUSED_DIRS:
        shutil.rmtree(tree / name, ignore_errors=True)
    for path in sorted(tree.rglob("*")):
        if path.is_symlink() and path.is_file():
            data = path.read_bytes()
            path.unlink()
            path.write_bytes(data)


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    tree = Path(sys.argv[1])
    restricted = restricted_images(tree)
    replaced = replace_images(tree, restricted)
    remove_restricted_files(tree, restricted)
    prepare_audiobook(tree)
    prepare_reading_trail(tree)
    materialize(tree)
    print(f"prepare_site: {len(restricted)} restricted images, {replaced} references replaced")
    return 0


if __name__ == "__main__":
    sys.exit(main())
