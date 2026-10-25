# ila: Interactive Linear Algebra (MathBook XML). 원 빌드(SCons + pretex.py +
# 패치된 Inkscape 0.92 + mathbox gulp3)는 재현할 수 없어, 원 XSLT(xsltproc)로
# HTML을 만들고 scripts/render_latex.py가 수식·TikZ 스니펫을 latex → dvisvgm으로
# SVG화합니다. CSS는 mathbook-assets의 SCSS를 compass로 컴파일합니다
# (Ruby 3.4에서 지워진 File.exists?는 scripts/compass-shim.rb가 채웁니다).
{
  pkgs,
  lib,
  system,
}: let
  # 스니펫 전문(src/latex/*.sty, pretex 프리앰블)이 쓰는 패키지: amsart,
  # mathdesign(Charter), tikz와 라이브러리(collection-pictures), kotex
  # (collection-langkorean), dvisvgm.
  tex = pkgs.texliveBasic.withPackages (ps:
    with ps; [
      collection-latexrecommended
      collection-pictures
      collection-langkorean
      collection-fontsrecommended
      amscls
      mathdesign
      charter
      helvetic
      dvisvgm
    ]);
in
  pkgs.mkShell {
    packages = [
      tex
      pkgs.libxslt
      pkgs.libxml2
      pkgs.python3
      pkgs.compass
      pkgs.ghostscript
      # build_html.py가 원문 커밋을 git으로 읽습니다.
      pkgs.git
    ];
  }
