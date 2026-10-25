# chip8-book: 원문은 pandoc용 원고이지만 한국어판은 mdBook HTML로 내놓습니다.
# mdbook + 빌드 스크립트가 부르는 python3(scripts/*.py — 표준 라이브러리만
# 사용, venv 불필요). PDF 타깃을 두지 않으므로 pandoc·TeX는 넣지 않습니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = with pkgs; [
    mdbook
    python3
  ];
}
