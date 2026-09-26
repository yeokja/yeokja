{
  pkgs,
  lib,
  system,
}: let
  notoSansCjk = pkgs.noto-fonts-cjk-sans;

  # asciidoctor-diagram 3.x의 ditaa 래퍼는 ditaamini 포크의 API
  # (org.stathissideris.ditaa.core)를 부릅니다. nixpkgs의 `ditaa`(0.11,
  # org.stathissideris.ascii2image)와는 호환되지 않고 nixpkgs에 ditaamini
  # 젬도 없으므로, rubygems의 젬에서 JAR만 꺼내 씁니다.
  ditaamini = pkgs.runCommand "ditaamini-1.0.3.jar" {
    src = pkgs.fetchurl {
      url = "https://rubygems.org/downloads/asciidoctor-diagram-ditaamini-1.0.3.gem";
      sha256 = "a630b2a80ee049a35ed39bc93ba33b571996381215583cc73f0c1ebcdc2a068e";
    };
  } ''
    tar -xOf $src data.tar.gz \
      | tar -xzO lib/asciidoctor-diagram/ditaa/ditaamini-1.0.3.jar > $out
  '';
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

    # asciidoctor-diagram은 ditaa 본체를 번들하지 않습니다. 이 변수가 없으면
    # ditaa 블록이 전부 "Could not load Ditaa"로 실패합니다.
    DIAGRAM_DITAA_CLASSPATH = "${ditaamini}";

    FONTCONFIG_FILE = pkgs.makeFontsConf {
      fontDirectories = [notoSansCjk];
    };
  }
