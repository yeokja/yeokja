# nix/

yeokja 모노레포의 Nix 툴체인 관리자입니다. **빌드 시스템이 아니라 개발 환경
(devShell) 제공자**입니다 — 각 번역 프로젝트가 필요로 하는 언어 런타임과 시스템
패키지를 선언할 뿐, upstream의 의존성 잠금(`package-lock.json`,
`requirements.txt`, `Cargo.lock` 등)은 다시 선언하지 않습니다.

flake는 저장소 루트가 아니라 이 디렉터리(`nix/`)에 있습니다. `nix develop`은
호출마다 flake가 속한 소스 트리 전체를 Nix 스토어에 복사하는데, 저장소 루트에는
`target/`, `build/`, 서브모듈까지 포함되어 수백 MB에 달하기 때문입니다. 항상
`path:` 스킴으로 참조하세요 — git 스킴은 미추적 파일을 보지 못하고 하위
디렉터리를 가리켜도 저장소 전체를 가져옵니다.

## 사용법

저장소 루트에서 yeokja 자체 Rust 툴체인이 필요하면:

```sh
nix develop path:nix -c cargo build --release -p yeokja-cli
```

특정 번역 프로젝트 디렉터리에서 그 프로젝트의 devShell을 쓰려면:

```sh
cd projects/mil
nix develop path:../../nix#mil -c ../../target/release/yeokja build html
```

## `nix/projects/<name>.nix` 파일 계약

이 디렉터리의 각 `.nix` 파일 하나가 devShell 하나에 대응합니다. 파일명(확장자
제외)이 그대로 `devShells.<system>.<name>`이 됩니다. 파일은 다음 형태의 함수여야
합니다.

```nix
{ pkgs, lib, system }: pkgs.mkShell {
  packages = [ pkgs.python3 ];
}
```

`nix/flake.nix`가 `builtins.readDir`로 이 디렉터리를 자동 스캔하므로, 새 파일을
추가하기만 하면 flake 수정 없이 devShell이 생깁니다. `nix/projects/` 디렉터리는
git이 빈 디렉터리를 추적하지 않으므로 저장소에는 파일이 하나도 없을 때 아예
존재하지 않을 수 있습니다 — `flake.nix`는 이 경우도 정상 동작하도록 만들어져
있습니다(그때는 `default` 셸만 노출됩니다).

## `default` 셸

`nix develop path:nix`(이름 없이)로 들어가는 기본 셸은 yeokja CLI 자체를 빌드하는
데 필요한 Rust 툴체인(`cargo`, `rustc`)입니다.
