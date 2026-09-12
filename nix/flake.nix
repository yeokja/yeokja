{
  # yeokja 모노레포의 Nix 툴체인 관리자.
  #
  # 이 flake는 저장소 루트가 아니라 `nix/` 하위에 있습니다. `nix develop`은
  # 매 호출마다 flake가 속한 소스 트리 전체를 Nix 스토어로 복사하는데, 저장소
  # 루트에는 `target/`, `build/`, 서브모듈까지 포함되어 수백 MB에 달합니다.
  # `nix/`만 복사되면 KB 단위입니다. 항상 `path:` 스킴으로 참조하세요
  # (`path:nix`, 또는 프로젝트 디렉터리 기준 `path:../../nix`). git 스킴은
  # 미추적 파일을 보지 못하고, 하위 디렉터리를 가리켜도 저장소 전체를 가져와
  # 이 최적화가 무의미해집니다.
  #
  # 프로젝트별 devShell 계약: `nix/projects/<name>.nix` 파일 하나가
  # devShell 하나에 대응합니다. 각 파일은 다음 형태의 함수여야 합니다.
  #
  #   { pkgs, lib, system }: pkgs.mkShell { ... }
  #
  # 파일은 `builtins.readDir`로 자동 발견되며, 파일명(`.nix` 제외)이 그대로
  # `devShells.<system>.<name>`이 됩니다. `nix/projects/` 디렉터리가 없거나
  # 비어 있어도 flake는 정상 동작합니다(그 경우 `default` 셸만 노출됩니다).

  description = "yeokja 모노레포 Nix 툴체인 관리자 (빌드 시스템 아님)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    systems = ["aarch64-darwin" "x86_64-linux" "aarch64-linux"];
    lib = nixpkgs.lib;

    forEachSystem = f: lib.genAttrs systems f;

    # nix/projects/<name>.nix -> { <name> = <파일 경로>; ... }
    projectsDir = ./projects;
    projectFiles =
      if builtins.pathExists projectsDir
      then
        lib.filterAttrs
        (name: type: type == "regular" && lib.hasSuffix ".nix" name)
        (builtins.readDir projectsDir)
      else {};
    projectNames = map (lib.removeSuffix ".nix") (builtins.attrNames projectFiles);
  in {
    devShells = forEachSystem (
      system: let
        pkgs = import nixpkgs {inherit system;};

        # yeokja 자체 Rust 워크스페이스 빌드에 필요한 최소 툴체인.
        # reqwest(기본 native-tls 백엔드)가 openssl-sys에 의존하므로
        # Linux에서는 pkg-config + openssl이 필요합니다. darwin은 Nix
        # stdenv가 Apple SDK를 기본으로 제공하므로 별도 항목이 없습니다.
        defaultShell = pkgs.mkShell {
          packages = with pkgs;
            [
              cargo
              rustc
              pkg-config
            ]
            ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [
              openssl
            ];
        };

        projectShells = builtins.listToAttrs (map
          (name: {
            inherit name;
            value = import (projectsDir + "/${name}.nix") {
              inherit pkgs lib system;
            };
          })
          projectNames);
      in
        projectShells // {default = defaultShell;}
    );

    formatter = forEachSystem (system: nixpkgs.legacyPackages.${system}.alejandra);
  };
}
