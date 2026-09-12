{
  pkgs,
  lib,
  system,
}: let
  pythonVenv = import ../lib/python-venv.nix {inherit pkgs;};
in
  pkgs.mkShell {
    packages = [pkgs.python3 pkgs.uv pkgs.pagefind];
    shellHook = pythonVenv {requirements = "requirements.txt";};
  }
