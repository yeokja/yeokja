# algorithmica: Hugo 정적 사이트(러시아어판 Algorithmica의 한국어 번역)와 조립·검사
# 스크립트(scripts/*.py, 표준 라이브러리만 사용)용 환경. 원문 Netlify 빌드는
# Hugo 0.96.0 extended이지만 nixpkgs(flake.lock 고정)의 hugo(extended, SASS 포함)로
# 빌드하며, 버전 차이로 필요한 설정은 overlay/config.yaml이 맞춥니다.
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
