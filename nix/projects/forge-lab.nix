# forge-lab: Hugo 블로그(Microsoft Security FORGE Lab 연구 노트의 한국어 번역)와
# 조립·검사 스크립트(scripts/*.py, 표준 라이브러리만 사용)용 환경. 원문은 Hugo
# extended 0.158.0 이상을 요구하며(blog/README.md), nixpkgs(flake.lock 고정)의
# hugo(extended)로 빌드합니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [
    pkgs.hugo
    pkgs.python3
  ];
}
