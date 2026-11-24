#!/usr/bin/env python3
"""overlay/가 손으로 옮긴 원문 파일이 바뀌지 않았는지 확인합니다.

사용법: python3 scripts/check_upstream.py

overlay/의 템플릿과 설정은 원문 파일을 복사해 문구를 한국어로 옮기고 그림·로고·
방문 통계를 뺀 것입니다. 원문 서브모듈을 올려 그 원문 파일이 바뀌면 번역 사이트는
옛 템플릿으로 조용히 빌드되므로, 여기 적힌 SHA-256과 다르면 실패합니다. 그때는
바뀐 내용을 overlay 사본에 옮긴 뒤 해시를 갱신합니다.

또한 upstream/blog/layouts의 파일 목록이 LAYOUTS와 같아야 합니다. 새 템플릿이 생기면
영어 문구가 번역 사이트에 섞이거나, prepare_site.py가 지운 파일을 부를 수 있습니다.
"""

import hashlib
import sys
from pathlib import Path

BLOG = Path(__file__).resolve().parent.parent / "upstream" / "blog"

# 원문 파일 → 그 파일을 대신하거나 덧씌우는 overlay 파일
MIRRORED = {
    "hugo.toml": ("overlay/hugo.toml", "cd0424f22cc27864cb8f45b6e231d1536203404f6523669a80e69ac07d207873"),
    "layouts/_default/baseof.html": ("overlay/layouts/_default/baseof.html", "63209af159c35dd57723d4a5a650bd12d022a35baee24ac16888dcf7b9b7130a"),
    "layouts/_default/single.html": ("overlay/layouts/_default/single.html", "662887660fd5f552346c060a7394ee2d025c5c604f2e8601fd8184a3e612d2cc"),
    "layouts/_default/list.html": ("overlay/layouts/_default/list.html", "e178bf3808272cde6ba133a9baa57ee2f30679f3e14c1deb22b055c357922670"),
    "layouts/index.html": ("overlay/layouts/index.html", "bf1f12a6deadb70732d95bb387cfea76ef0295bb719d12142054bc9eed907d80"),
    "layouts/partials/post-list.html": ("overlay/layouts/partials/post-list.html", "87016d693dcd5589182951c86434e8f2d709e97dbfa576b73cfc0790eaab0cba"),
    "layouts/partials/social-meta.html": ("overlay/layouts/partials/social-meta.html", "5231d2bb6fd795449d6e53dbdb7c606794c3487ea1713f85d403a06fb09de480"),
    # 번역 사이트는 main.css를 그대로 쓰고 overlay/assets/css/translation.css로 글꼴
    # 변수와 첫 화면 배치를 덮어씁니다. 변수 이름이나 배치가 바뀌면 다시 맞춥니다.
    "assets/css/main.css": ("overlay/assets/css/translation.css", "63f8db0ff2b3eba177893adc67a868662135aeffb89801ca320d1c2fefe0c451"),
}

# upstream/blog/layouts의 모든 파일. MIRRORED에 없는 것은 문구가 없어 그대로 쓰거나
# (shortcodes/note.html) prepare_site.py가 지우는 파일입니다.
LAYOUTS = {
    "layouts/_default/baseof.html",
    "layouts/_default/list.html",
    "layouts/_default/single.html",
    "layouts/index.html",
    "layouts/partials/analytics.html",
    "layouts/partials/artwork-image.html",
    "layouts/partials/kernel-repair.html",
    "layouts/partials/microsoft-logo.html",
    "layouts/partials/post-list.html",
    "layouts/partials/rust-inspection.html",
    "layouts/partials/social-meta.html",
    "layouts/privacy/single.html",
    "layouts/shortcodes/note.html",
}


def main() -> int:
    failures = []
    for rel, (copy, expected) in MIRRORED.items():
        path = BLOG / rel
        if not path.is_file():
            failures.append(f"upstream/blog/{rel} is gone; drop {copy} or port it to the new layout")
            continue
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        if actual != expected:
            failures.append(f"upstream/blog/{rel} changed ({actual}); port the change to {copy} and update the hash")
    layouts = {p.relative_to(BLOG).as_posix() for p in (BLOG / "layouts").rglob("*") if p.is_file()}
    for rel in sorted(layouts - LAYOUTS):
        failures.append(f"new upstream template upstream/blog/{rel}; translate it in overlay/ or remove it in prepare_site.py")
    for rel in sorted(LAYOUTS - layouts):
        failures.append(f"upstream template upstream/blog/{rel} is gone; update LAYOUTS and prepare_site.py")
    for failure in failures:
        print("FAIL", failure)
    if not failures:
        print(f"upstream templates unchanged: {len(MIRRORED)} mirrored files, {len(LAYOUTS)} layouts")
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
