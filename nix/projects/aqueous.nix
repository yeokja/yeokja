# Bikeshed는 nixpkgs의 python3Packages.bikeshed가 의존성(json-home-client)
# 때문에 broken이라, peps처럼 python3 + uv로 프로젝트의 requirements.txt를
# build/venv에 설치합니다.
{
  pkgs,
  lib,
  system,
}: let
  pythonVenv = import ../lib/python-venv.nix {inherit pkgs;};
in
  pkgs.mkShell {
    packages = [pkgs.python3 pkgs.uv];
    shellHook = pythonVenv {requirements = "requirements.txt";};
  }
