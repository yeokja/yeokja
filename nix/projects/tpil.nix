# Theorem Proving in Lean 4 한국어 번역용 devShell.
#
# fp-lean과 같은 방식입니다: elan이 upstream book/examples 서브모듈의
# `lean-toolchain` 파일을 첫 호출 때 읽어 필요한 Lean/Lake 버전을 내려받습니다
# (pages.yml의 `lean --version` 스텝과 동일). 사용자의 기본 `~/.elan`을 그대로
# 쓰므로 ELAN_HOME은 설정하지 않습니다. `scripts/update-verso-spans.sh`가
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
