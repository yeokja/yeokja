#!/usr/bin/env bash

set -euo pipefail

# 사용법: stage-pages.sh <artifacts-dir> <site-dir> <landing-dir> [<fingerprints-dir>]
#
# <artifacts-dir>에 내려받은 dist-* 산출물을 마지막으로 배포된 <site-dir>
# 트리 위에 덮어씁니다. 산출물이 없는 프로젝트는 이전 배포분을 그대로
# 보존합니다 — 빌드가 실패했거나, 지문이 같아 plan 잡이 빌드를 건너뛴 경우
# 둘 다 여기에 해당합니다.
#
# <fingerprints-dir>가 주어지면(plan-rebuilds.sh가 적어 둔 <artifact> 파일들),
# 실제로 덮어쓴 산출물의 지문만 <site-dir>/build-fingerprints/<artifact>에
# 기록합니다. 다음 실행의 plan 잡은 이 기록과 현재 지문을 비교해 빌드를
# 건너뛸지 정하므로, 지문은 반드시 그 산출물이 트리에 들어간 뒤에만 기록해야
# 합니다. 기록이 트리와 함께 캐시·배포되므로 둘이 어긋날 일이 없습니다.

if [ "$#" -lt 3 ] || [ "$#" -gt 4 ]; then
  echo "usage: $0 <artifacts-dir> <site-dir> <landing-dir> [<fingerprints-dir>]" >&2
  exit 2
fi

artifacts_dir=$1
site_dir=$2
landing_dir=$3
fingerprints_dir=${4:-}

fail() {
  echo "error: $1" >&2
  exit 1
}

warn_preserved() {
  echo "warning: preserving published $1; $2 is unavailable" >&2
}

record_fingerprint() {
  artifact_name=$1
  if [ -z "$fingerprints_dir" ]; then
    return
  fi
  if [ ! -f "$fingerprints_dir/$artifact_name" ]; then
    echo "warning: no fingerprint for $artifact_name; it will be rebuilt next run" >&2
    return
  fi
  mkdir -p "$site_dir/build-fingerprints"
  cp "$fingerprints_dir/$artifact_name" "$site_dir/build-fingerprints/$artifact_name"
}

overlay_site() {
  artifact_name=$1
  source_name=$2
  destination_name=$3
  source_path="$artifacts_dir/$artifact_name/$source_name"

  if [ ! -d "$source_path" ]; then
    warn_preserved "$destination_name" "$artifact_name"
    return
  fi

  rm -rf "${site_dir:?}/${destination_name:?}"
  cp -R "$source_path" "$site_dir/$destination_name"
  record_fingerprint "$artifact_name"
}

overlay_pypy() {
  artifact_path="$artifacts_dir/dist-pypy"
  if [ ! -d "$artifact_path/site" ] || [ ! -d "$artifact_path/rpython-site" ]; then
    warn_preserved "pypy and rpython" "dist-pypy"
    return
  fi

  rm -rf "$site_dir/pypy" "$site_dir/rpython"
  cp -R "$artifact_path/site" "$site_dir/pypy"
  cp -R "$artifact_path/rpython-site" "$site_dir/rpython"
  record_fingerprint "dist-pypy"
}

overlay_download() {
  artifact_name=$1
  file_name=$2
  destination_name=${3:-napkin}
  source_path="$artifacts_dir/$artifact_name/$file_name"

  if [ ! -f "$source_path" ]; then
    warn_preserved "$destination_name/$file_name" "$artifact_name"
    return
  fi

  mkdir -p "$site_dir/$destination_name"
  cp "$source_path" "$site_dir/$destination_name/$file_name"
  record_fingerprint "$artifact_name"
}

test -s "$site_dir/index.html" || \
  fail "published Pages baseline is missing index.html"
test -s "$landing_dir/index.html" || fail "landing page is missing index.html"
test -s "$landing_dir/favicon.svg" || fail "landing page is missing favicon.svg"

overlay_site "dist-thebeambook" "site" "theBeamBook"
overlay_pypy
overlay_site "dist-fp-lean" "site" "fp-lean"
overlay_site "dist-tpil" "site" "tpil"
overlay_site "dist-mil" "site" "mil"
overlay_site "dist-peps" "site" "peps"
overlay_site "dist-napkin-html" "site" "napkin"
overlay_download "dist-napkin-pdf" "Napkin-ko.pdf"
overlay_download "dist-napkin-epub" "Napkin-ko.epub"
overlay_download "dist-chisel-book-pdf" "Digital-Design-with-Chisel-ko.pdf" "chisel-book"
overlay_site "dist-devguide" "site" "devguide"
overlay_site "dist-rust-forge" "site" "rust-forge"
overlay_site "dist-learn-fpga" "site" "learn-fpga"

cp "$landing_dir/index.html" "$site_dir/index.html"
cp "$landing_dir/favicon.svg" "$site_dir/favicon.svg"

# 필수 프로젝트는 산출물이 아니라 최종 트리를 기준으로 확인합니다 — plan 잡이
# 빌드를 건너뛴 경우 산출물은 없지만 보존된 트리에 이미 들어 있습니다.
test -s "$site_dir/devguide/index.html" || \
  fail "required devguide site is missing from the staged tree"
test -s "$site_dir/chisel-book/Digital-Design-with-Chisel-ko.pdf" || \
  fail "required chisel-book PDF is missing from the staged tree"
