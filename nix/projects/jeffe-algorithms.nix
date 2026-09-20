# jeffe-algorithms: 원서 PDF 추출(PyMuPDF, poppler)과 LuaLaTeX 조판.
# 원서 글꼴과 같은 계열(XCharter, Roboto, Inconsolata; 수식 XCharter-Math)을
# TeX Live에서, 한국어는 나눔 글꼴을 씁니다(hott 프로젝트와 같은 글꼴 설정 방식).
{
  pkgs,
  lib,
  system,
}: let
  tex = pkgs.texlive.combine {
    inherit
      (pkgs.texlive)
      scheme-medium
      collection-luatex
      collection-langkorean
      collection-fontsrecommended
      collection-fontsextra
      collection-latexextra
      collection-mathscience
      latexmk
      ;
  };
in
  pkgs.mkShell {
    packages = [
      tex
      pkgs.nanum
      pkgs.poppler-utils
      (pkgs.python3.withPackages (ps: [ps.pymupdf]))
    ];
    FONTCONFIG_FILE = pkgs.makeFontsConf {fontDirectories = [pkgs.nanum];};
    # source/luaotfload.conf limits the LuaTeX font database to TeX Live and
    # $OSFONTDIR; a project-specific TEXMFVAR keeps that database from
    # replacing the shared one other LuaTeX projects rely on.
    shellHook = ''
      export OSFONTDIR="${pkgs.nanum}/share/fonts//"
      export TEXMFVAR="''${XDG_CACHE_HOME:-$HOME/.cache}/yeokja/jeffe-algorithms/texmf-var"
    '';
  }
