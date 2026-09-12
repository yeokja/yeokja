# scripts/의 빌드·검증 스크립트는 표준 라이브러리만 임포트하므로(argparse,
# json, pathlib, re, subprocess, unittest 등) python3만 제공합니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [pkgs.python3];
}
