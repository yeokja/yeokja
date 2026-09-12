#!/usr/bin/env bash
#
# nix.dev 한국어판 HTML 빌드. `yeokja build html`이 조립된 트리(upstream 위에
# ko/와 overrides/를 덮은 심링크 묶음) 안에서 실행하며, $YEOKJA_ROOT는
# projects/nix-dev입니다. 결과는 트리의 site/에 남기고 yeokja가 dist/로
# 복사합니다.
#
# 1. Python 툴체인: nix devShell(nix/projects/nix-dev.nix)이 python3와 uv를
#    제공하고, 그 shellHook이 requirements.txt를 build/venv에 설치한 뒤 venv의
#    python3를 PATH 앞에 얹습니다. 이 스크립트는 그 PATH의 python3를 그대로
#    씁니다(nix develop이 셸을 준비하지 않은 경우도 대비해 NIX_DEV_PYTHON으로
#    다른 python3를 지정할 수 있습니다).
# 2. 실파일 source/: 조립 트리의 파일은 upstream(또는 ko/)을 가리키는
#    심링크인데, myst는 `[…](./other.md)` 같은 문서 링크를 realpath로 풀어
#    srcdir 바깥이라며 깨뜨리고(myst.xref_missing), 아래 버전 치환도 서브모듈
#    파일을 고쳐 버립니다. source/만 심링크를 관통 복사한 실파일로 바꿉니다
#    (600 KB 남짓).
# 3. 버전 자리 표시자: upstream은 source/reference/nix-manual.md의 @nix-latest@
#    등을 Nix로 계산해 치환합니다(default.nix 참조). scripts/nix-releases.py가
#    같은 값을 JSON 고정본에서 구해 치환합니다.
# 4. Sphinx: 한국어판 설정은 overrides/source/conf.py가 upstream conf.py를
#    실행한 뒤 덧붙이므로 sphinx-build에 -D를 넘길 것이 없습니다. -W는 쓰지
#    않지만(upstream도 번역과 무관한 경고가 생길 수 있음) 오류가 있으면
#    sphinx-build 자체가 실패합니다.
# 5. Pagefind: upstream의 search.html 템플릿이 Sphinx 검색 대신 Pagefind UI를
#    쓰므로 색인(site/pagefind/)이 없으면 검색 페이지가 비어 버립니다. nix
#    devShell이 pagefind 바이너리를 PATH에 제공하므로 이 스크립트는 npx 없이
#    pagefind를 직접 부릅니다. 색인 생성이 실패하면 빌드도 실패시킵니다.
#    언어는 pagefind.yml의 en 대신 ko로 강제합니다.

set -euo pipefail

root=${YEOKJA_ROOT:?YEOKJA_ROOT가 필요합니다 — yeokja build html로 실행하세요}
python=${NIX_DEV_PYTHON:-python3}

rm -rf source.real
cp -RL source source.real
rm -rf source
mv source.real source

manual=source/reference/nix-manual.md
"$python" "$root/scripts/nix-releases.py" substitute "$root/upstream" "$root/nixpkgs-nix-versions.json" \
  "$manual" "$manual.substituted"
mv "$manual.substituted" "$manual"

rm -rf site
"$python" -m sphinx -b html -j auto -d build/doctrees source site

pagefind --site site --force-language ko
test -f site/pagefind/pagefind-ui.js
