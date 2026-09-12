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
