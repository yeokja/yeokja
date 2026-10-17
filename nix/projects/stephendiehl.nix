# stephendiehl: mdbook + mdbook-katex(글의 `$` 수식을 빌드 시점에 렌더링) +
# 빌드 스크립트가 부르는 python3(scripts/*.py — 표준 라이브러리만 사용, venv
# 불필요). pandoc은 scripts/fetch_site.py가 블로그 HTML을 source/ 원고로 다시
# 변환할 때만 필요하고 Pages 빌드에는 쓰이지 않습니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [
    pkgs.mdbook
    pkgs.mdbook-katex
    pkgs.pandoc
    pkgs.python3
  ];
}
