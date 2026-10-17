# rust-api-guidelines: mdbook(원문 book.toml에 전처리기 없음) + 빌드 스크립트가
# 부르는 python3(scripts/*.py — 표준 라이브러리만 사용, venv 불필요).
# 원문 CI는 mdbook 버전을 고정하지 않으며, nixpkgs의 mdbook 0.5.x로 빌드합니다.
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
