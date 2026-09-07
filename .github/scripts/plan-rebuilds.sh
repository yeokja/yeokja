#!/usr/bin/env bash
#
# 어떤 프로젝트·타깃을 다시 빌드해야 하는지 결정합니다.
#
# 매트릭스 항목마다 build-fingerprint.sh로 현재 지문을 내고, 마지막으로
# 스테이징된 Pages 트리에 기록된 지문(<recorded-dir>/<artifact>)과 비교합니다.
# 지문이 같으면 그 트리에 이미 같은 입력으로 만든 산출물이 들어 있으므로
# 빌드를 건너뛰고, 다르거나 기록이 없거나 --force가 주어지면 매트릭스에
# 넣습니다. 지문은 rebuild 잡의 산출물 유무와 무관하게 전부 <out-dir>에
# 적어 두어, stage-pages.sh가 실제로 덮어쓴 항목의 지문만 트리에 기록하게
# 합니다.
#
# 사용법: plan-rebuilds.sh <projects.json> <recorded-dir> <out-dir> [--force]
#   표준 출력으로 다시 빌드할 항목만 담은 JSON 배열(한 줄)을 냅니다.
#   저장소 루트에서 실행하며, 각 프로젝트의 ko/는 이미 재구성되어 있어야
#   합니다.

set -euo pipefail

if [ "$#" -lt 3 ]; then
  echo "usage: $0 <projects.json> <recorded-dir> <out-dir> [--force]" >&2
  exit 2
fi

projects_json=$1
recorded_dir=$2
out_dir=$3
force=${4:-}

script_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
fingerprint_script="$script_dir/build-fingerprint.sh"

mkdir -p "$out_dir"

selected=()
count=$(jq 'length' "$projects_json")
for ((i = 0; i < count; i++)); do
  entry=$(jq -c ".[$i]" "$projects_json")
  project=$(jq -r '.project' <<<"$entry")
  target=$(jq -r '.target' <<<"$entry")
  artifact=$(jq -r '.artifact' <<<"$entry")

  args=("projects/$project" "$target")
  if [ "$(jq -r '.rebuild_daily == true' <<<"$entry")" = "true" ]; then
    args+=(--daily)
  fi
  current=$("$fingerprint_script" "${args[@]}")
  printf '%s\n' "$current" > "$out_dir/$artifact"

  recorded=""
  if [ -f "$recorded_dir/$artifact" ]; then
    recorded=$(tr -d '[:space:]' < "$recorded_dir/$artifact")
  fi

  if [ "$force" = "--force" ]; then
    echo "$artifact: rebuild (forced; fingerprint $current)" >&2
    selected+=("$artifact")
  elif [ -z "$recorded" ]; then
    echo "$artifact: rebuild (no recorded fingerprint; now $current)" >&2
    selected+=("$artifact")
  elif [ "$recorded" != "$current" ]; then
    echo "$artifact: rebuild (fingerprint $recorded -> $current)" >&2
    selected+=("$artifact")
  else
    echo "$artifact: up to date (fingerprint $current)" >&2
  fi
done

selected_json=$(jq -cn '$ARGS.positional' --args "${selected[@]}")
jq -c --argjson selected "$selected_json" \
  '[.[] | select(.artifact as $a | $selected | index($a) != null)]' \
  "$projects_json"
