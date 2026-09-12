# Zero to Nix: upstream을 Astro(Node)로 빌드합니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [pkgs.nodejs_24];
}
