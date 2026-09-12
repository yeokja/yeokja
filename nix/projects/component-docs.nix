# component-docs: mdbook + mdbook-tabs(언어별 탭 전처리기) + 빌드 스크립트가
# 부르는 python3(scripts/*.py — 표준 라이브러리만 사용, venv 불필요).
#
# mdbook-tabs(https://crates.io/crates/mdbook-tabs, CI가 고정한 1.0.1)는
# nixpkgs pkgs/by-name에 이 이름으로 없습니다(nixpkgs에는 다른 crate/저장소인
# RustForWeb/mdbook-plugins 기반 "mdbook-plugins" 0.3.4가 있으나 별개 패키지이며
# CI가 쓰는 crate가 아닙니다). crates.io에서 직접 buildRustPackage로 빌드합니다.
#
# 버전을 올릴 때는 아래 두 해시를 `lib.fakeHash`로 바꾸고 `nix develop`이
# 보고하는 got: 값으로 다시 채웁니다.
{
  pkgs,
  lib,
  system,
}: let
  mdbook-tabs = pkgs.rustPlatform.buildRustPackage {
    pname = "mdbook-tabs";
    version = "1.0.1";

    src = pkgs.fetchCrate {
      pname = "mdbook-tabs";
      version = "1.0.1";
      hash = "sha256-4m5eAOszbIJ5i6ghte8KKP2n0TYUhYSbpHdp8+MmZSk=";
    };

    cargoHash = "sha256-WDfTXcDrJaAjkmUvGhiU5GwzduuOMnqSq8hp9xd0sdQ=";

    meta = {
      description = "mdBook preprocessor for rendering content in tabs";
      mainProgram = "mdbook-tabs";
    };
  };
in
  pkgs.mkShell {
    packages = [
      pkgs.mdbook
      mdbook-tabs
      pkgs.python3
    ];
  }
