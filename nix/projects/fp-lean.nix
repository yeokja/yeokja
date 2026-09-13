# Functional Programming in Lean 한국어 번역용 devShell.
#
# 빌드는 elan이 관리하는 Lean/Lake 툴체인으로 진행됩니다 (upstream book과
# examples 서브모듈 각각의 `lean-toolchain` 파일을 elan이 첫 호출 때 읽어
# 필요한 버전을 내려받습니다 — pages.yml의 `lean --version` 스텝과 동일한
# 방식). elan은 사용자의 기본 `~/.elan`을 그대로 쓰므로 ELAN_HOME은 설정하지
# 않습니다 (다른 프로젝트와 툴체인 캐시를 공유).
#
# python3: 책이 빌드 중 `python3 inordernumbering.py`를 실행해 그 출력을 본문의
# `commandOut` 블록과 대조합니다(upstream CI는 이 출력을 고정하려고 3.10.4를
# 핀함). nixpkgs에는 3.10이 더 이상 없어 기본 python3(3.14)를 쓰며, 이 셸로
# `yeokja build html`이 통과하는 것을 2026-09-12에 확인했습니다 — 예제의 출력은
# 명시적 `__repr__`라 버전에 따라 달라지지 않습니다. `scripts/update-verso-spans.sh`도
# 같은 python3로 lake-manifest.json을 읽습니다.
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

  # upstream book/expect(Pty.lean)의 네이티브 FFI(pty.c)가 정의하는
  # book_Expect_Pty.so를 Lake가 병렬로 빌드할 때, 해당 .o가 링크되기 전에
  # 의존 모듈이 .so를 로드해 "undefined symbol: expect_pty_spawn"으로
  # 실패하는 경우를 2026-09-13 CI에서 두 차례 결정적으로 재현했습니다(같은
  # 지점에서 재현됨). Lake는 병렬도를 낮추는 공식 플래그가 없고
  # LEAN_NUM_THREADS만 지원하므로, 이 프로젝트에 한해 직렬 빌드로 강제해
  # 경쟁 상태를 피합니다.
  LEAN_NUM_THREADS = "1";
}
