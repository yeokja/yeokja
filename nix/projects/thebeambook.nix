{
  pkgs,
  lib,
  system,
}: let
  notoSansCjk = pkgs.noto-fonts-cjk-sans;
in
  pkgs.mkShell {
    packages = with pkgs; [
      asciidoctor-with-extensions
      jre
      graphviz
      notoSansCjk
      rsync
      git
      gnumake
    ];

    FONTCONFIG_FILE = pkgs.makeFontsConf {
      fontDirectories = [notoSansCjk];
    };
  }
