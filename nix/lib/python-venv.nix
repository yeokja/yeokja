# nix.dev의 scripts/build-site.sh가 쓰던 stamp 방식(요구사항 파일의 sha256을
# venv 안에 남겨 두고, 파일이 바뀌지 않으면 재설치하지 않음)을 devShell
# shellHook에서 공용으로 쓰기 위한 헬퍼입니다. shellHook은 `nix develop`을
# 호출한 디렉터리(즉 프로젝트 디렉터리)를 cwd로 실행되며, 그 시점에는
# $YEOKJA_ROOT가 아직 없으므로 경로는 모두 $PWD 기준 상대 경로로 받습니다.
#
# 사용: pythonVenv { requirements = "upstream/requirements.txt"; }
{pkgs}: {
  requirements,
  venvDir ? "build/venv",
}: ''
  requirements=$PWD/${requirements}
  venv=$PWD/${venvDir}
  if [ -f "$requirements" ]; then
    stamp="$venv/.requirements.stamp"
    want=$(${pkgs.coreutils}/bin/sha256sum "$requirements" | ${pkgs.coreutils}/bin/cut -d' ' -f1)
    if [ ! -x "$venv/bin/python" ] || [ "$(cat "$stamp" 2>/dev/null)" != "$want" ]; then
      echo "python-venv: installing $requirements into $venv" >&2
      rm -rf "$venv"
      python3 -m venv "$venv"
      uv pip install --python "$venv/bin/python" -r "$requirements"
      printf '%s\n' "$want" > "$stamp"
    fi
    export PATH="$venv/bin:$PATH"
  else
    echo "python-venv: $requirements 가 없습니다 (cwd=$PWD) — 프로젝트 디렉터리에서 nix develop을 실행했고 upstream 서브모듈이 받아져 있는지 확인하세요" >&2
  fi
''
