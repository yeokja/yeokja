# technicalpm: 빌드 스크립트(scripts/build_epub.py, build_site.py, check.py)는
# 표준 라이브러리만 쓰는 python3 하나로 돕니다. epubcheck 비교는 무거운 JDK를
# 끌어오므로 CI 셸에 넣지 않고 README의 로컬 검증 절차로 둡니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [pkgs.python3];
}
