# rust-forge: mdbook + blacksmith 전처리기(cargo run, RUN_BLACKSMITH=1)를 위한
# Rust 툴체인. blacksmith(projects/rust-forge/upstream/blacksmith)는
# reqwest(기본 native-tls 백엔드)를 쓰며 Cargo.lock에 openssl-sys가 있어
# Linux에서는 pkg-config + openssl이 필요합니다(darwin은 native-tls가
# Security.framework를 쓰므로 불필요 — nix/flake.nix의 default 셸과 같은 패턴).
# 빌드 자체는 네트워크로 static.rust-lang.org에서 릴리스 채널 정보를
# 받아오므로 셸 밖에서 처리할 사항은 없습니다(변경 없음).
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = with pkgs;
    [
      cargo
      rustc
      mdbook
      pkg-config
    ]
    ++ lib.optionals pkgs.stdenv.hostPlatform.isLinux [
      openssl
    ];
}
