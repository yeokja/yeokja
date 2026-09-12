# rustc-dev-guide: mdbook + mdbook-mermaid 전처리기, 빌드 스크립트가 부르는
# python3(scripts/preserve_anchors.py, scripts/build_print.py — 표준 라이브러리만
# 사용, venv 불필요). CI는 mdbook 0.5.2/mdbook-mermaid 0.17.0을 cargo install로
# 고정했지만, nixpkgs에는 mdbook 0.5.4가 있고 mdbook-mermaid는 CI와 같은
# 0.17.0입니다(design spec 표 참고).
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = with pkgs; [
    mdbook
    mdbook-mermaid
    python3
  ];
}
