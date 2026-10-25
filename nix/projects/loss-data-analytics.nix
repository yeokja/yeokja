# loss-data-analytics: bookdown(R Markdown) 교재를 gitbook HTML로 knit합니다.
# R 패키지는 원문 Rmd가 library()/::로 부르는 것 전부를 rWrapper 하나에 모읍니다
# (원문은 버전을 고정하지 않으므로 nixpkgs의 버전을 씁니다). CASdatasets만
# CRAN이 아닌 cas.uqam.ca에 있어 여기서 소스 tarball을 해시로 고정해 빌드합니다.
# 그 밖에 pandoc(bookdown), TeX Live(knitr tikz 엔진: 손해 진전 그림 6개),
# ImageMagick·Ghostscript·poppler(tikz PDF → PNG, magick·pdftools), gifski
# (gganimate 애니메이션), C++ 컴파일러(nimble이 모형을 C++로 컴파일; mkShell의
# stdenv가 제공), python3(scripts/*.py, 표준 라이브러리만)를 둡니다.
{
  pkgs,
  lib,
  system,
}: let
  base = pkgs.rPackages;

  # nixpkgs 기본 빌드가 이 시스템에서 실패하는 두 패키지를 고칩니다(rPackages 안에서
  # 바꿔야 의존하는 패키지도 고친 판을 씁니다).
  # - pdftools 3.9.0: configure가 poppler-data.pc를 찾는데 빌드 입력에 poppler_data가
  #   없습니다(Packages.Rmd가 library("pdftools")를 부름).
  # - sf(transformr가 의존, Packages.Rmd가 library('transformr')를 부름): configure의
  #   PROJ+sqlite3 링크 검사가 proj.pc의 비공개 의존(libtiff-4, libcurl과 그것이 끄는
  #   libunistring·libidn2 …, sqlite3)을 찾지 못해 실패합니다.
  rp = pkgs.rPackages.override {
    overrides = {
      pdftools = base.pdftools.overrideAttrs (old: {
        buildInputs = (old.buildInputs or []) ++ [pkgs.poppler_data];
      });
      sf = base.sf.overrideAttrs (old: {
        buildInputs =
          (old.buildInputs or [])
          ++ (with pkgs; [libtiff sqlite curl libunistring libidn2 libpsl nghttp2 nghttp3 ngtcp2 zstd brotli openssl libssh2 krb5 zlib]);
      });
    };
  };

  CASdatasets = rp.buildRPackage {
    pname = "CASdatasets";
    version = "1.2-0";
    src = pkgs.fetchurl {
      url = "https://cas.uqam.ca/pub/src/contrib/CASdatasets_1.2-0.tar.gz";
      sha256 = "0zn4gy70fmw64ny2s8gp9q9z3gxrk8fvslppi9nf49va4p36bizp";
    };
    propagatedBuildInputs = with rp; [xts survival lattice zoo];
    nativeBuildInputs = with rp; [xts survival lattice zoo];
  };

  R = pkgs.rWrapper.override {
    packages = with rp; [
      CASdatasets
      ChainLadder
      FRAPO
      GB2
      GofCens
      Hmisc
      MASS
      Metrics
      RColorBrewer
      VGAM
      actuar
      alabama
      bookdown
      boot
      copula
      doBy
      dplyr
      epitools
      fitdistrplus
      forecast
      gganimate
      ggplot2
      gifski
      goftest
      gridExtra
      gridGraphics
      htmlTable
      insuranceData
      invgamma
      kableExtra
      knitr
      magick
      mcmcse
      nimble
      numDeriv
      pander
      patchwork
      pdftools
      png
      psych
      rmarkdown
      rpart
      rpart_plot
      scales
      statmod
      survival
      tikzDevice
      transformr
      tseries
      webexercises
    ];
  };

  tex = pkgs.texliveSmall.withPackages (ps:
    with ps; [
      pgf
      preview
      standalone
      xcolor
      amsmath
    ]);
in
  pkgs.mkShell {
    packages = with pkgs; [
      R
      pandoc
      tex
      imagemagick
      ghostscript
      poppler-utils
      gifski
      python3
    ];
  }
