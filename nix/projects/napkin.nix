{
  pkgs,
  lib,
  system,
}: let
  # projects/napkin/assets/flake.nix의 devShell과 동일한 정의입니다. pdf
  # 타깃은 그 flake의 defaultPackage(assembled tree 위에서 nix build)를 그대로
  # 쓰고, html/epub 타깃은 이 devShell 안에서 직접 명령을 실행합니다.
  notoSansCjk = pkgs.noto-fonts-cjk-sans.override {static = true;};
  notoSerifCjk = pkgs.noto-fonts-cjk-serif.override {static = true;};
  tex = pkgs.texlive.combined.scheme-full;
in
  pkgs.mkShell {
    packages = with pkgs; [
      asymptote
      biber
      ghostscript
      mathjax
      notoSansCjk
      notoSerifCjk
      pandoc
      perlPackages.LaTeXML
      python3
      tex
      epubcheck
    ];
    FONTCONFIG_FILE = pkgs.makeFontsConf {
      fontDirectories = [notoSansCjk notoSerifCjk];
    };
    shellHook = ''
      export OSFONTDIR="${notoSansCjk}/share/fonts//:${notoSerifCjk}/share/fonts//"
      export NAPKIN_MATHJAX_DIR="${pkgs.mathjax}/lib/node_modules/mathjax"
    '';
  }
