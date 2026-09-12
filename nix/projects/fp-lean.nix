# Functional Programming in Lean 한국어 번역용 devShell.
#
# 빌드는 elan이 관리하는 Lean/Lake 툴체인으로 진행됩니다 (upstream book과
# examples 서브모듈 각각의 `lean-toolchain` 파일을 elan이 첫 호출 때 읽어
# 필요한 버전을 내려받습니다 — pages.yml의 `lean --version` 스텝과 동일한
# 방식). elan은 사용자의 기본 `~/.elan`을 그대로 쓰므로 ELAN_HOME은 설정하지
# 않습니다 (다른 프로젝트와 툴체인 캐시를 공유). `scripts/update-verso-spans.sh`가
# lake-manifest.json에서 Verso revision을 읽는 데 python3(표준 라이브러리만
# 사용)가 필요합니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [
    pkgs.elan
    pkgs.git
    pkgs.curl
    pkgs.python3
  ];

  # elan/lake가 HTTPS로 툴체인·의존성을 내려받을 때 쓸 CA 번들.
  SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
}
