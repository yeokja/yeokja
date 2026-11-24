# upstream requirements.txt(Sphinx~=8.2.3, sphinx-book-theme~=1.4.0 등)를
# build/venv에 설치합니다. sphinxcontrib-spelling(PyEnchant)과
# sphinx-multiversion도 함께 설치되지만, 한국어 빌드는 scripts/prepare.py가
# conf.py에서 두 확장을 빼므로 libenchant 같은 시스템 라이브러리는 필요 없습니다
# (PyEnchant는 확장이 로드될 때만 libenchant를 찾습니다).
{
  pkgs,
  lib,
  system,
}: let
  pythonVenv = import ../lib/python-venv.nix {inherit pkgs;};
in
  pkgs.mkShell {
    packages = [pkgs.python3 pkgs.uv];
    shellHook = pythonVenv {requirements = "upstream/requirements.txt";};
  }
