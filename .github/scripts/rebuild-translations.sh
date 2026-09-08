#!/usr/bin/env bash
#
# 커밋된 state/에서 한 프로젝트의 번역 출력(ko/)을 재구성하고, 미번역
# 세그먼트가 남아 있으면 실패합니다. 프로젝트마다 source 규칙이 달라
# (devguide는 upstream 디렉터리 전체, PEP는 upstream/peps, zero-to-nix는
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
  devguide|learn-fpga)
    "$yeokja" translate upstream
    "$yeokja" status --check upstream
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
  *)
    for s in $(find state -name '*.yeokja.json'); do
      src="${s#state/}"; src="${src%.yeokja.json}"
      "$yeokja" translate "$src"
      "$yeokja" status --check "$src"
    done
    ;;
esac
