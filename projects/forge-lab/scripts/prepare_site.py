#!/usr/bin/env python3
"""조립 트리(또는 비교용 원문 사본)를 Hugo 빌드에 맞게 손질합니다.

사용법:
  python3 scripts/prepare_site.py korean TREE     # 번역 사이트(derive generate 단계)
  python3 scripts/prepare_site.py original TREE   # 비교용 원문 사이트

두 모드 공통:
- 개인정보 처리 방침 쪽(content/privacy, layouts/privacy)을 지웁니다. 원문 사이트의
  Azure Application Insights 방문 통계를 설명하는 쪽인데, 번역 사이트는 방문 통계를
  싣지 않습니다. 원문 비교 빌드에서도 지워 두 빌드의 쪽 목록을 맞춥니다.

korean 모드:
- 번역 범위(content/posts/**/*.md, overlay가 더한 _index.md 제외)의 모든 글이 ko/의
  번역본인지 확인합니다. 번역이 없으면 원문 영어 글이 조용히 섞여 나가므로 실패합니다.
- 번역 사이트가 싣지 않는 파일을 지웁니다.
  - Microsoft 로고(partials/microsoft-logo.html)와 본문의 "Microsoft" 낱말에 로고를
    붙이는 스크립트(js/brand-wordmarks.js). 원문 README는 수정본이 Microsoft의 후원을
    암시하지 않도록 요구합니다.
  - 방문 통계(partials/analytics.html, js/analytics.js).
  - 이용 허락이 분명하지 않은 장식 그림: 첫 화면·글 머리 그림(assets/images,
    글 묶음의 cover*.webp)과 그 움직임 스크립트·스타일, 공유 미리보기 그림
    (static/images), 파비콘. 원문 LICENSE(CC BY 4.0)는 README가 "Documentation and
    other written content"에 적용한다고 밝히고, 그림은 어느 라이선스에도 들지 않습니다.
    본문의 SVG 도표는 글의 일부(문서)로 보아 싣습니다.
- 남은 심볼릭 링크를 실제 파일로 바꿉니다(Hugo는 링크인 파일을 읽지 않습니다).
"""

import os
import shutil
import sys
from pathlib import Path

COMMON_REMOVE = ["content/privacy", "layouts/privacy"]

KOREAN_REMOVE = [
    "layouts/partials/microsoft-logo.html",
    "layouts/partials/analytics.html",
    "layouts/partials/artwork-image.html",
    "layouts/partials/kernel-repair.html",
    "layouts/partials/rust-inspection.html",
    "assets/js",
    "assets/css/kernel-repair.css",
    "assets/css/rust-inspection.css",
    "assets/images",
    "static/images",
    "static/favicon.ico",
    "static/favicon.svg",
    "tests",
]


def remove(tree: Path, rel: str) -> None:
    path = tree / rel
    if path.is_dir() and not path.is_symlink():
        shutil.rmtree(path)
    elif path.exists() or path.is_symlink():
        path.unlink()
    else:
        raise SystemExit(f"{rel} is missing from the tree; update scripts/prepare_site.py")


def check_translations(tree: Path, root: Path) -> None:
    ko = (root / "ko").resolve()
    missing = []
    for path in sorted((tree / "content" / "posts").rglob("*.md")):
        rel = path.relative_to(tree).as_posix()
        if path.name == "_index.md":
            continue
        if not path.is_symlink() or not path.resolve().is_relative_to(ko):
            missing.append(rel)
    if missing:
        raise SystemExit(f"no translation for {missing}; run `yeokja translate` first")


def materialize(tree: Path) -> None:
    for path in sorted(tree.rglob("*")):
        if path.is_symlink() and path.is_file():
            data = path.read_bytes()
            path.unlink()
            path.write_bytes(data)


def original(tree: Path) -> None:
    for rel in COMMON_REMOVE:
        remove(tree, rel)


def korean(tree: Path) -> None:
    root = Path(os.environ.get("YEOKJA_ROOT", Path(__file__).resolve().parent.parent))
    check_translations(tree, root)
    for rel in COMMON_REMOVE + KOREAN_REMOVE:
        remove(tree, rel)
    for cover in sorted((tree / "content" / "posts").rglob("cover*.webp")):
        cover.unlink()
    materialize(tree)


def main() -> int:
    if len(sys.argv) != 3 or sys.argv[1] not in ("korean", "original"):
        print(__doc__, file=sys.stderr)
        return 2
    tree = Path(sys.argv[2])
    korean(tree) if sys.argv[1] == "korean" else original(tree)
    return 0


if __name__ == "__main__":
    sys.exit(main())
