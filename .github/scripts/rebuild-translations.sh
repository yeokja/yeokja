#!/usr/bin/env bash
#
# 커밋된 state/에서 한 프로젝트의 번역 출력(ko/)을 재구성하고, 미번역
# 세그먼트가 남아 있으면 실패합니다. 프로젝트마다 source 규칙이 달라
# (cpython-devguide는 upstream 디렉터리 전체, PEP는 upstream/peps, zero-to-nix는
# upstream/src/content, 나머지는 state 파일 하나당 원본 하나) 그 분기를 한 곳에
# 모아 두고, plan 잡과 rebuild 잡이 같은 스크립트를 부릅니다.
#
# 사용법: rebuild-translations.sh <project-name> [yeokja-binary]
#   프로젝트 디렉터리는 projects/<project-name>이며, 저장소 루트에서 실행합니다.

set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "usage: $0 <project-name> [yeokja-binary]" >&2
  exit 2
fi

project=$1
yeokja=$(realpath "${2:-target/release/yeokja}")
project_dir="projects/$project"

if [ ! -d "$project_dir" ]; then
  echo "error: no such project directory: $project_dir" >&2
  exit 1
fi

cd "$project_dir"

case "$project" in
  raytracing)
    python3 scripts/prepare_index.py extract --check
    "$yeokja" status --check upstream/books
    "$yeokja" status --check ui
    "$yeokja" translate upstream/books
    "$yeokja" translate ui
    ;;
  putting-the-you-in-cpu)
    "$yeokja" status --check upstream/src/content/chapters
    "$yeokja" status --check ui
    "$yeokja" translate upstream/src/content/chapters
    "$yeokja" translate ui
    python3 scripts/prepare_ui.py
    ;;
  hott)
    # Check every configured source before reconstruction; CI must not call a provider.
    python3 scripts/index_terms.py extract
    "$yeokja" status --check index
    "$yeokja" status --check upstream
    "$yeokja" translate index
    "$yeokja" translate upstream
    "$yeokja" status --check upstream
    ;;
  cpython-devguide|learn-fpga|perf-ninja)
    "$yeokja" translate upstream
    "$yeokja" status --check upstream
    ;;
  webassembly-component-docs)
    "$yeokja" status --check upstream/component-model/src
    "$yeokja" translate upstream/component-model/src
    "$yeokja" status --check upstream/component-model/src
    ;;
  rustc-dev-guide|rust-api-guidelines)
    # src/ 디렉터리 전체를 한 번에 확인해 upstream이 새로 추가한 문서가
    # state 없이 빠지는 경우도 배포를 막습니다.
    "$yeokja" translate upstream/src
    "$yeokja" status --check upstream/src
    ;;
  furiosa-opt)
    # docs/ mdBook 책 전체를 한 번에 확인해 upstream이 새로 추가한 문서가
    # state 없이 빠지는 경우도 배포를 막습니다.
    "$yeokja" translate upstream/docs/src
    "$yeokja" status --check upstream/docs/src
    ;;
  zero-to-nix)
    # Astro 사이트의 MDX 본문만 번역 대상이므로 [[sources]] 경로 전체를
    # 한 번에 확인해 새 문서가 state 없이 빠지는 경우도 배포를 막습니다.
    "$yeokja" translate upstream/src/content
    "$yeokja" status --check upstream/src/content
    ;;
  peps)
    # PEP는 루트 문서와 appendices를 여러 source 규칙으로 다룹니다. 프로젝트
    # 전체를 한 번에 확인해야 새 PEP가 state 없이 빠지는 경우도 배포를 막습니다.
    "$yeokja" translate upstream/peps
    "$yeokja" status --check upstream/peps
    ;;
  write-you-a-haskell)
    # book/src mdBook 책 전체를 한 번에 확인해 upstream이 새로 추가한 장이
    # state 없이 빠지는 경우도 배포를 막습니다.
    "$yeokja" translate upstream/book/src
    "$yeokja" status --check upstream/book/src
    ;;
  jeffe-algorithms)
    # 복원한 영어 원고(source/) 전체를 확인한 뒤 ko/를 재구성합니다.
    "$yeokja" status --check source
    "$yeokja" translate source
    ;;
  stephendiehl)
    # 블로그 HTML에서 복원한 영어 원고(source/: 글과 홈 소개)를 한 번에 확인한
    # 뒤 ko/를 재구성합니다. 재구성 전에 확인하므로 CI가 provider를 부르지 않습니다.
    "$yeokja" status --check source
    "$yeokja" translate source
    ;;
  chip8-book)
    # src/의 9개 장 전체를 재구성 전에 확인해 CI가 provider를 부르지 않게 하고,
    # upstream이 새로 추가한 장이 state 없이 빠지는 경우도 배포를 막습니다.
    "$yeokja" status --check upstream/src
    "$yeokja" translate upstream/src
    ;;
  ila)
    # src/의 PreTeXt 파일 전체를 재구성 전에 확인해 CI가 provider를 부르지 않게 하고,
    # upstream이 새로 추가한 장이 state 없이 빠지는 경우도 배포를 막습니다.
    "$yeokja" status --check upstream/src
    "$yeokja" translate upstream/src
    ;;
  principles-finance)
    # CNXML 모듈과 컬렉션(두 source 규칙)을 upstream 전체로 한 번에 확인해
    # upstream이 새로 추가한 모듈이 state 없이 빠지는 경우도 배포를 막습니다.
    # 재구성 전에 확인하므로 CI가 번역 provider를 부르지 않습니다.
    "$yeokja" status --check upstream
    "$yeokja" translate upstream
    ;;
  loss-data-analytics)
    # Gloss 표시어 사전이 원문과 맞는지 먼저 보고, Rmd·GlossFct.csv(upstream)와 표시어(gloss)를
    # 재구성 전에 확인해 CI가 provider를 부르지 않게 하고 새 문서가 state 없이 빠지는 것도 막습니다.
    python3 scripts/gloss_labels.py extract --check
    "$yeokja" status --check upstream
    "$yeokja" status --check gloss
    "$yeokja" translate upstream
    "$yeokja" translate gloss
    ;;
  technicalpm)
    # EPUB 콘텐츠 문서 전체(upstream/OEBPS)를 재구성 전에 확인해 CI가 provider를
    # 부르지 않게 하고, 원문을 다시 내려받아 생긴 새 문서가 state 없이 빠지는
    # 경우도 배포를 막습니다.
    "$yeokja" status --check upstream/OEBPS
    "$yeokja" translate upstream/OEBPS
    ;;
  cp-algorithms)
    # 문서 디렉터리 전체와 홈 본문(README → src/index_body)을 함께 확인해
    # upstream이 추가한 문서가 state 없이 빠지는 경우도 배포를 막습니다.
    # 재구성 전에 확인하므로 CI가 번역 provider를 부르지 않습니다.
    "$yeokja" status --check upstream/src
    "$yeokja" status --check upstream/README.md
    "$yeokja" translate upstream/src
    "$yeokja" translate upstream/README.md
    ;;
  nix-dev)
    # nix.dev의 문서는 모두 upstream/source 아래의 MyST Markdown입니다. 디렉터리
    # 전체를 확인해야 upstream이 새로 추가한 문서가 state 없이 빠지는 경우도
    # 배포를 막습니다.
    "$yeokja" translate upstream/source
    "$yeokja" status --check upstream/source
    ;;
  fp-lean)
    # anchorEqSteps가 예제 모듈과 원고의 같은 문장을 한 줄씩 비교하므로, 한쪽만
    # 고친 번역은 Lean 빌드가 한참 돈 뒤에야 실패합니다. 재구성 전에 거릅니다.
    python3 scripts/check_example_pairs.py
    for s in $(find state -name '*.yeokja.json'); do
      src="${s#state/}"; src="${src%.yeokja.json}"
      "$yeokja" translate "$src"
      "$yeokja" status --check "$src"
    done
    ;;
  *)
    for s in $(find state -name '*.yeokja.json'); do
      src="${s#state/}"; src="${src%.yeokja.json}"
      "$yeokja" translate "$src"
      "$yeokja" status --check "$src"
    done
    ;;
esac
