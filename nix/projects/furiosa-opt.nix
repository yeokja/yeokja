# furiosa-opt: mdbook + mdbook-mermaid(원문 book.toml의 [preprocessor.mermaid]) +
# 빌드 스크립트가 부르는 python3(scripts/*.py — 표준 라이브러리만 사용, venv 불필요).
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [
    pkgs.mdbook
    pkgs.mdbook-mermaid
    pkgs.python3
  ];
}
