# nix.dev 한국어판 Sphinx 설정. 조립된 트리에서 upstream의 source/conf.py를
# 대신합니다: 먼저 upstream 설정을 그대로 실행한 뒤 한국어판에 필요한 값만
# 덧붙이므로 upstream이 확장이나 테마 옵션을 바꿔도 여기서는 따라갈 것이
# 없습니다. 이 파일은 projects/nix-dev/overrides/source/conf.py이고, 빌드
# 스크립트가 조립 트리의 source/로 복사합니다. 프로젝트 디렉터리는 yeokja
# build가 주는 $YEOKJA_ROOT로 찾습니다.

import os
import sys
from pathlib import Path

_project = Path(os.environ["YEOKJA_ROOT"]).resolve()
_upstream_conf = _project / "upstream" / "source" / "conf.py"
exec(compile(_upstream_conf.read_text(encoding="utf-8"), str(_upstream_conf), "exec"))

# ---- 한국어판 설정 ----

language = "ko"

# 푸터의 "By %(author)s"가 ko 로케일에서 "으로 …"로 찍히므로 저자 문구를
# 한국어 어순에 맞게 통째로 씁니다. 원문 저자 표기는 그대로 담습니다.
author = (
    '<a href="https://nixos.org/community/teams/documentation">Nix 문서 팀</a>과 '
    "기여자들이 작성했습니다. 한국어판은 yeokja 기계 번역입니다"
)

# 사이트는 https://yeokja.moreal.dev/nix-dev/ 아래에 배포됩니다. sitemap.xml은
# html_baseurl을 따르고(sitemap_url_scheme = "{link}"), 404 페이지의 자산
# 경로는 notfound_urls_prefix를 따릅니다.
html_baseurl = "https://yeokja.moreal.dev/nix-dev/"
notfound_urls_prefix = "/nix-dev/"

html_theme_options = {
    **html_theme_options,
    "announcement": (
        '이 사이트는 <a href="https://nix.dev/">nix.dev</a>의 비공식 한국어 기계 번역입니다. '
        '원문과 번역 모두 <a href="https://creativecommons.org/licenses/by-sa/4.0/deed.ko">'
        "CC BY-SA 4.0</a>에 따라 배포되며, 정확한 내용은 원문을 확인하세요."
    ),
}

# 번역된 제목에도 원문의 영어 앵커(#slug)를 유지합니다. scripts/ko_slugs.py 참조.
sys.path.append(str(_project / "scripts"))
extensions.append("ko_slugs")
ko_slugs_reference = str(_project / "upstream" / "source")
