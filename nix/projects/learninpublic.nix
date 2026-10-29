# learninpublic: Quarto book(The Coding Career Handbook)의 웹 HTML 판을 만드는 환경.
# 원문은 Quarto 1.10.18을 고정하며(.quarto-version) nixpkgs(flake.lock)의 quarto도
# 같은 판입니다. 다만 nixpkgs의 quarto는 nixpkgs의 pandoc(3.7.0.2)을 QUARTO_PANDOC으로
# 묶는데, Quarto 1.10은 pandoc 3.10의 옵션(`syntax-highlighting`)을 쓰므로 그 pandoc으로는
# 렌더링이 실패합니다. 그래서 pandoc 3.10 공식 릴리스 바이너리를 시스템별로 해시로
# 고정해 받아 QUARTO_PANDOC으로 넘깁니다. 원문의 npm 스크립트(아이콘·소셜 카드·EPUB·
# Typst·epubcheck)는 쓰지 않고, 조립·검사 스크립트(scripts/*.py)는 python3 표준
# 라이브러리만 씁니다.
{
  pkgs,
  lib,
  system,
}: let
  pandocVersion = "3.10";
  release = {
    x86_64-linux = {
      asset = "linux-amd64.tar.gz";
      sha256 = "0i1jj9ck3q4zfm5pkpk0riy0jfnsbmnzxkjvm8mx4rzjs1iazy70";
      dir = "pandoc-${pandocVersion}";
    };
    aarch64-linux = {
      asset = "linux-arm64.tar.gz";
      sha256 = "197lfvm8fa760861qys9vgrlhj78fcgqz1gy84b8dv0s1kxksham";
      dir = "pandoc-${pandocVersion}";
    };
    aarch64-darwin = {
      asset = "arm64-macOS.zip";
      sha256 = "0d7f9abvgygmb1cz891frkr5yk1ymnqmpi68r06llxxfjqfx1jnr";
      dir = "pandoc-${pandocVersion}-arm64";
    };
    x86_64-darwin = {
      asset = "x86_64-macOS.zip";
      sha256 = "0q9i99jmqvdyqhqvykkj55nmys07apx5dbfzc7kkg7kwmzcz8d33";
      dir = "pandoc-${pandocVersion}-x86_64";
    };
  }.${system};

  pandocBin = pkgs.stdenvNoCC.mkDerivation {
    pname = "pandoc-bin";
    version = pandocVersion;
    src = pkgs.fetchurl {
      url = "https://github.com/jgm/pandoc/releases/download/${pandocVersion}/pandoc-${pandocVersion}-${release.asset}";
      inherit (release) sha256;
    };
    nativeBuildInputs = lib.optionals (lib.hasSuffix ".zip" release.asset) [pkgs.unzip];
    sourceRoot = release.dir;
    dontBuild = true;
    dontFixup = true;
    installPhase = ''
      mkdir -p $out/bin
      cp bin/pandoc $out/bin/pandoc
      chmod +x $out/bin/pandoc
    '';
  };
in
  pkgs.mkShell {
    packages = [
      pkgs.quarto
      pkgs.python3
    ];
    QUARTO_PANDOC = "${pandocBin}/bin/pandoc";
  }
