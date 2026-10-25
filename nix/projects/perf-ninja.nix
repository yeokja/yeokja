{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [pkgs.python3 pkgs.pandoc];
}
