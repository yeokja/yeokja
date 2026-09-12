{
  pkgs,
  lib,
  system,
}: let
  # 원문 pages.yml의 latex-chisel 툴체인(apt-get: fonts-dejavu-core
  # fonts-nanum texlive-bibtex-extra texlive-fonts-recommended
  # texlive-lang-korean texlive-latex-extra texlive-xetex)을 미러링합니다.
  tex = pkgs.texlive.combine {
    inherit
      (pkgs.texlive)
      scheme-medium
      collection-langkorean
      collection-fontsrecommended
      collection-latexextra
      collection-bibtexextra
      ;
  };
in
  pkgs.mkShell {
    packages = [
      tex
      pkgs.python3
      pkgs.gnumake
      pkgs.nanum
      pkgs.dejavu_fonts
    ];

    # figures/Makefile가 pdflatex로 각 파형 다이어그램을 렌더링하고,
    # chisel-book.tex 본문은 XeLaTeX + 나눔 글꼴로 한국어를 조판합니다.
    FONTCONFIG_FILE = pkgs.makeFontsConf {
      fontDirectories = [pkgs.nanum pkgs.dejavu_fonts];
    };

    shellHook = ''
      export OSFONTDIR="${pkgs.nanum}/share/fonts//:${pkgs.dejavu_fonts}/share/fonts//"
    '';
  }
