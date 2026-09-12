{pkgs, ...}: let
  # graphviz 확장이 다이어그램에 한국어 라벨을 그릴 때 CJK 글리프가 필요합니다.
  notoSansCjk = pkgs.noto-fonts-cjk-sans;
in
  pkgs.mkShell {
    packages = [
      pkgs.pypy27
      pkgs.graphviz
    ];

    FONTCONFIG_FILE = pkgs.makeFontsConf {
      fontDirectories = [notoSansCjk];
    };

    # 공식 빌드 환경이 Python 2.7입니다(.readthedocs.yaml) — conf.py의
    # pypyconfig 확장이 py2 문법인 pypy.config/rpython.config를 임포트하기
    # 때문입니다. nixpkgs의 pypy27은 `pypy27.withPackages`/`pypy27Packages.*`가
    # 오랫동안 깨져 있어(예: NixOS/nixpkgs#39356 "pypy.withPackages can't
    # import packages", 2018년부터 열려 있고 이 핀 시점에도 미해결) 그 경로로는
    # pip을 넣을 수 없습니다. 대신 bare `pypy27` 인터프리터만 받아 브루
    # 절차(README 참고)를 그대로 재현합니다: ensurepip --user로 pip을
    # build/pypy-bootstrap에 넣고, 그 pip으로 구버전 virtualenv(16.7.12 — 최신 virtualenv는
    # Python>=3.7만 지원해 pypy27Packages.virtualenv 자체가 존재하지 않음)를
    # 같은 곳에 설치한 뒤, 그 virtualenv로 PyPy2 venv를 만들어 Sphinx
    # 툴체인을 넣습니다. requirements.txt의 해시를 도장으로 남겨 재설치를
    # 건너뜁니다(nix-dev/scripts/build-site.sh와 같은 패턴).
    shellHook = ''
      requirements="$PWD/requirements.txt"
      if [ -f "$requirements" ]; then
        venv="$PWD/build/venv"
        stamp="$venv/.requirements.stamp"
        want=$(${pkgs.pypy27}/bin/pypy -c 'import hashlib, sys; print(hashlib.sha256(open(sys.argv[1], "rb").read()).hexdigest())' "$requirements")
        if [ ! -x "$venv/bin/pip" ] || [ "$(cat "$stamp" 2>/dev/null)" != "$want" ]; then
          echo "pypy devShell: PyPy2 Sphinx venv를 $venv 에 부트스트랩합니다" >&2
          rm -rf "$venv"
          mkdir -p "$(dirname "$venv")"
          # 부트스트랩용 pip/virtualenv는 ~/.local이 아니라 build/ 아래의
          # 전용 user site에 넣어 프로젝트 밖을 건드리지 않습니다. nixpkgs의
          # pypy는 user site를 sys.path에 올리지 않으므로(ENABLE_USER_SITE=False)
          # PYTHONPATH로 직접 가리킵니다.
          bootstrap="$PWD/build/pypy-bootstrap"
          rm -rf "$bootstrap"
          PYTHONUSERBASE="$bootstrap" ${pkgs.pypy27}/bin/pypy -m ensurepip --user
          bootstrap_site="$bootstrap/lib/pypy2.7/site-packages"
          PYTHONUSERBASE="$bootstrap" PYTHONPATH="$bootstrap_site" ${pkgs.pypy27}/bin/pypy -m pip install --user --quiet 'virtualenv==16.7.12'
          PYTHONPATH="$bootstrap_site" ${pkgs.pypy27}/bin/pypy -m virtualenv --python="${pkgs.pypy27}/bin/pypy" "$venv"
          "$venv/bin/pip" install --quiet -r "$requirements"
          printf '%s\n' "$want" > "$stamp"
        fi
        export PATH="$venv/bin:$PATH"
      fi
    '';
  }
