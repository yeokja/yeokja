#!/bin/sh
# 조립된 트리 안에서 실행됩니다 (`yeokja build`가 cwd를 트리로 잡습니다).
#
# 툴체인(asciidoctor, ditaa용 jre 등)은 Nix devShell(nix/projects/thebeambook.nix)이
# 제공합니다. `nix develop path:../../nix#thebeambook`로 들어온 셸에서 실행합니다.
set -e

make html

# Makefile의 `rsync -R code/*/*.png site`는 심링크를 건너뜁니다(-l 없음).
# 트리에서는 코드 그림이 전부 심링크라 -L로 관통 복사를 한 번 더 합니다.
rsync -R -L code/*/*.png site
