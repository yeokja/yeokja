# upstream Makefile이 uv로 자체 venv(./venv)를 만들어 requirements.txt를
# 설치하므로(ensure-venv 타겟), 여기서는 python3 + uv만 제공합니다. 공유
# venv 훅은 필요 없습니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [pkgs.python3 pkgs.uv];
}
