# WebGPU Fundamentals: 원본 lesson-builder(Node)가 Puppeteer로 Chrome을
# 구동해 정적 페이지를 렌더링하고, scripts/*.py(Python)가 소스 검증과 빌드
# 준비/후처리를 맡습니다. git은 GIT_DIR/GIT_WORK_TREE로 upstream 서브모듈
# 커밋 이력을 읽어 게시 날짜를 만드는 데 필요합니다.
#
# Linux에서는 nixpkgs의 chromium을 Puppeteer의 실행 파일로 지정해 매 빌드마다
# Chrome을 내려받지 않게 합니다(PUPPETEER_EXECUTABLE_PATH가 설정되어 있으면
# yeokja.toml의 build.html 명령이 이미 install.mjs 호출을 건너뜁니다). darwin은
# 기존 동작대로 Puppeteer가 직접 Chrome을 내려받도록 둡니다(nixpkgs chromium은
# darwin에서 별도 이슈가 있었고, 기존 CI에도 macOS 경로가 없었습니다).
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages =
    [pkgs.nodejs_24 pkgs.python3 pkgs.git]
    ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [pkgs.chromium];

  shellHook = lib.optionalString pkgs.stdenv.hostPlatform.isLinux ''
    export PUPPETEER_EXECUTABLE_PATH=${pkgs.chromium}/bin/chromium
  '';
}
