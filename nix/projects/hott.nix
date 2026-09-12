{
  pkgs,
  lib,
  system,
}: let
  # 원문 pages.yml의 latex-hott 툴체인(apt-get: fonts-nanum fonts-texgyre
  # latexmk texlive-xetex texlive-latex-extra texlive-fonts-recommended
  # texlive-science texlive-lang-korean)을 미러링합니다.
  tex = pkgs.texlive.combine {
    inherit
      (pkgs.texlive)
      scheme-medium
      collection-langkorean
      collection-fontsrecommended
      collection-latexextra
      collection-mathscience
      latexmk
      ;
  };
in
  pkgs.mkShell {
    packages = [
      tex
      pkgs.python3
      pkgs.git
      pkgs.nanum
      pkgs.gyre-fonts
    ];

    # hott-online.tex은 XeLaTeX + 나눔 글꼴로 한국어를 조판하고, 원문의
    # TeX Gyre 글꼴(gyre-fonts)도 함께 필요합니다.
    FONTCONFIG_FILE = pkgs.makeFontsConf {
      fontDirectories = [pkgs.nanum pkgs.gyre-fonts];
    };

    shellHook = ''
      export OSFONTDIR="${pkgs.nanum}/share/fonts//:${pkgs.gyre-fonts}/share/fonts//"
    '';
  }
