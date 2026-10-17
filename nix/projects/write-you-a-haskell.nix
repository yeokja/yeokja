# write-you-a-haskell: 원문 book.toml이 요구하는 mdBook 전처리기 체인과 빌드
# 스크립트가 부르는 python3.
#
# 원문 BUILDING.md는 mdBook 0.5.3, mdbook-katex 0.10.0, mdbook-bib 0.5.3(저자
# 패치), 저자의 mdbook-include-rs 포크(고정 커밋), pandoc 3.12.1을 `.tools/`에
# cargo install로 고정합니다. nixpkgs에 있는 것은 그대로 쓰고(mdbook 0.5.4,
# mdbook-katex 0.10.0, mdbook-bib 0.5.3, pandoc 3.7.x — design spec 표 참고),
# 없는 것만 여기서 소스로 빌드합니다.
#
# - mdbook-include-rs: nixpkgs에 없으므로 저자 포크를 scripts/bootstrap.py가
#   고정한 커밋에서 buildRustPackage로 빌드합니다. 포크는 mdBook 0.4 API를
#   쓰며, 원문의 scripts/include-rs.py가 0.5 JSON 봉투로 감싸서 호출합니다.
# - mdbook-bib: nixpkgs 0.5.3에 scripts/bootstrap.py와 같은 한 줄 패치
#   (저자 없이 편집자만 있는 항목 — refs.bib의 haskell2010 — 은 편집자를
#   저자로 쓴다)를 적용합니다. 패치 없이는 그 항목의 인용이 비어 렌더링됩니다.
# - pandoc: scripts/bibliography.py가 참고문헌 페이지를 `pandoc --citeproc`로
#   렌더링합니다(PDF 빌드가 아니라 HTML 빌드의 전처리기입니다).
{
  pkgs,
  lib,
  system,
}: let
  mdbook-include-rs = pkgs.rustPlatform.buildRustPackage {
    pname = "mdbook-include-rs";
    version = "0.2.0-unstable-2025-08-15";
    src = pkgs.fetchFromGitHub {
      owner = "sdiehl";
      repo = "mdbook-include-rs";
      rev = "32e5190a5418367059851a2ce5055dceab79b510";
      hash = "sha256-Wgxe9xLFpvwcIX96rhZbqf7Q7MmM8M/SH5ARGz+jBlI=";
    };
    cargoHash = "sha256-WJW/Se2fc7mqU+qhG4529XdSNlm/lnq8kDpPx659pAQ=";
    # insta 스냅샷 테스트는 저장소의 예제 트리에 의존하므로 생략합니다.
    doCheck = false;
    meta.mainProgram = "mdbook-include-rs";
  };

  mdbook-bib = pkgs.mdbook-bib.overrideAttrs (old: {
    postPatch =
      (old.postPatch or "")
      + ''
        substituteInPlace src/parser/hayagriva_parser.rs \
          --replace-fail 'let persons = entry.authors();' \
                         'let persons = entry.authors().or_else(|| entry.editors());'
      '';
  });
in
  pkgs.mkShell {
    packages = [
      pkgs.mdbook
      pkgs.mdbook-katex
      mdbook-bib
      mdbook-include-rs
      pkgs.pandoc
      pkgs.python3
    ];
  }
