# principles-finance: CNXML 모듈을 정적 HTML로 내는 자체 렌더러
# (scripts/build_site.py)와 검사(scripts/check.py)가 python3 + lxml로 돕니다.
# 공식 빌드 도구(enki: Docker, Prince, Node, Java, Ruby)는 쓰지 않습니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [(pkgs.python3.withPackages (ps: [ps.lxml]))];
}
