# Putting the "You" in CPU: Astro 사이트를 Bun으로 빌드하고, scripts/*.py
# (Python)가 UI 문자열 추출/적용과 빌드 준비·검증을 맡습니다.
#
# CI(oven-sh/setup-bun@v2)는 bun 1.3.14로 고정되어 있으나, 이 nixpkgs 핀에는
# 1.3.13만 있습니다(2026-09-10 기준 nixpkgs-unstable). 패치 버전 차이이며
# `bun install --frozen-lockfile`은 lockfile 포맷으로 동작하므로 빌드 결과에
# 영향이 없을 것으로 예상하지만, 실제 검증은 확인하지 못했습니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [pkgs.bun pkgs.nodejs_24 pkgs.python3];
}
