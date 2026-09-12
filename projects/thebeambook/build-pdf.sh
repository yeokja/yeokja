#!/bin/sh
# 조립된 트리 안에서 실행됩니다 (`yeokja build pdf`가 cwd를 트리로 잡습니다).
#
# 툴체인(asciidoctor-pdf, ditaa용 jre 등)은 Nix devShell(nix/projects/thebeambook.nix)이
# 제공합니다. `nix develop path:../../nix#thebeambook`로 들어온 셸에서 실행합니다.
#
# `make pdf-a4`를 부르지 않는 이유: 그 타깃의 선행조건
# chapters/opcodes_doc.asciidoc이 erl과 genop.tab 네트워크 다운로드를
# 요구합니다. 영문 빌드에도 들어가지 않는 장이라, Makefile의 레시피만
# 그대로 옮겨 직접 실행합니다.
set -e

# 테마는 패치된 online-book.asciidoc의 :pdf-theme:이 한국어판
# (style/pdf-online-ko-theme.yml, 한글 fallback 폰트 포함)을 가리킵니다.
asciidoctor-pdf -r asciidoctor-diagram \
    -r ./style/custom-pdf-converter.rb \
    -r ./style/custom-admonition-block.rb \
    -a config=./style/ditaa.cfg \
    -a pdf-fontsdir=./style/fonts \
    -a source-highlighter=rouge \
    -a rouge-style=pastie \
    -a rouge-linenums-mode=table \
    online-book.asciidoc -o beam-book-ko-a4.pdf
