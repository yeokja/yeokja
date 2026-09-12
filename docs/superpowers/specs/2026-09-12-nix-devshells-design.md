# 번역 프로젝트 툴체인의 Nix devShell 통일 설계

**날짜**: 2026-09-12
**상태**: 승인됨 (구현 plan: `docs/superpowers/plans/2026-09-12-nix-devshells.md`)

## 문제

20개 번역 프로젝트의 빌드 **명령**은 각 `projects/<name>/yeokja.toml`의
`[build.*]`에 있지만, 그 명령이 요구하는 **툴체인**(언어 런타임, mdbook, TeX,
폰트, graphviz 등)의 선언은 `.github/workflows/pages.yml`의 `if: matrix.toolchain == …`
스텝 15여 개에만 있습니다. 로컬 재현 방법은 README 주석(brew, asdf, apt)과 빌드
스크립트의 macOS 전용 PATH 분기(`thebeambook/build-*.sh`), 빌드 명령 안의
`GITHUB_ACTIONS` 분기(`learn-fpga`, `nix-dev`)로 흩어져 있습니다. 결과적으로 "이
프로젝트를 빌드하려면 무엇이 필요한가"의 단일 출처가 없고, CI가 그 역할을 대신합니다.

## 결정

Nix를 **빌드 시스템이 아니라 툴체인 관리자**로 씁니다.

- 각 프로젝트는 `nix/projects/<name>.nix`에 devShell 하나를 선언합니다. 내용은
  언어 런타임·빌드 도구·시스템 패키지(TeX, 폰트, graphviz, pandoc, Java 등)뿐입니다.
- upstream의 의존성 잠금(`package-lock.json`, `requirements.txt`, `lean-toolchain`, `Cargo.lock`)은
  Nix로 재선언하지 않고, devShell 안에서 upstream의 패키지 매니저(`npm ci`, `uv`/`pip`, `lake`, `cargo`)가
  그대로 처리합니다. 순수 derivation(`nix build`)은 목표가 아닙니다 —
  `docs/nix-build-migration.md`가 정리한 장벽(네트워크, elan, Python 2.7)은 devShell에서는
  문제가 되지 않습니다.
- 빌드 명령은 로컬과 CI에서 동일하게 `nix develop path:nix#<name> -c yeokja build <target>`로
  실행합니다. `GITHUB_ACTIONS` 분기와 macOS 전용 PATH 분기는 제거합니다.

## flake 배치

flake는 저장소 루트가 아니라 `nix/` 하위 디렉터리에 둡니다.

- 루트 flake는 `nix develop` 때마다 flake 소스 트리 전체(git 트리 또는 `path:`의 경우
  `target/`, `build/`, 서브모듈까지)를 Nix 스토어로 복사합니다. 이 저장소는 그 크기가
  수백 MB라 매 호출이 느려집니다. `nix/`만 복사되면 KB 단위입니다.
- 참조는 항상 `path:` 스킴(`path:nix`, 또는 `"path:$YEOKJA_ROOT/../../nix"`)을
  씁니다. git 스킴은 미추적 파일을 보지 못하고, 하위 디렉터리를 가리켜도 저장소 전체를
  가져옵니다.
- flake는 `nix/projects/*.nix`를 `builtins.readDir`로 발견해 파일명을 devShell 이름으로
  씁니다. 각 파일은 `{ pkgs, lib, system }: pkgs.mkShell { … }` 형태의 함수입니다.
- `devShells.default`는 yeokja 자체의 Rust 툴체인(cargo, rustc)입니다.
- nixpkgs 핀은 하나(`nix/flake.lock`)입니다. 시스템은 `aarch64-darwin`, `x86_64-linux`, `aarch64-linux`.

## Python 프로젝트 규약

nixpkgs의 Python 패키지 버전은 upstream `requirements.txt`의 핀과 다르므로, Python
프로젝트의 devShell은 `python3`와 `uv`만 제공하고, `shellHook`에서
`$YEOKJA_ROOT/build/venv`(프로젝트 디렉터리 기준 `build/venv`)에 requirements를 설치한 뒤
PATH 앞에 얹습니다. requirements 파일의 해시를 stamp로 남겨 바뀌지 않으면 재설치하지
않습니다(`nix-dev/scripts/build-site.sh`가 쓰던 방식을 `nix/lib/python-venv.nix`로 공용화). devguide는 upstream
Makefile이 uv로 venv를 만들므로 uv만 제공합니다.

## 프로젝트별 툴체인 (CI 스텝 → devShell)

| 프로젝트 | devShell 내용 | 비고 |
|---|---|---|
| rust-forge | cargo, rustc, mdbook | blacksmith 전처리기는 cargo가 빌드, 네트워크 필요(변경 없음) |
| rustc-dev-guide | mdbook, mdbook-mermaid, python3 | nixpkgs mdbook 0.5.4 (CI는 0.5.2 핀이었음) |
| component-docs | mdbook, mdbook-tabs(nixpkgs에 없음 — 셸 파일 안에서 `fetchCrate` + `buildRustPackage`로 1.0.1 빌드), python3 | |
| zero-to-nix | nodejs_24 | |
| webgpufundamentals | nodejs_24, python3, Linux에서는 chromium + `PUPPETEER_EXECUTABLE_PATH` | darwin은 puppeteer가 Chrome을 내려받음 |
| putting-the-you-in-cpu | bun, nodejs_24, python3 | |
| devguide | python3, uv | contributors 표는 unshallow 유지(CI) |
| peps, mil | python3, uv + venv 규약 | |
| nix-dev | python3, uv + venv 규약, pagefind | `GITHUB_ACTIONS`·venv 분기와 npx 호출 제거 |
| learn-fpga | python3, pandoc | `GITHUB_ACTIONS` apt-get 분기 제거 |
| raytracing | python3 | |
| chisel-book, hott | texlive(필요 collection 조합), nanum, 폰트 설정, python3, git | `FONTCONFIG_FILE`로 폰트 노출 |
| napkin | 기존 `assets/flake.nix`의 devShell 내용을 옮김 | pdf 타깃의 `nix build`(upstream flake + assets 오버레이)는 그대로 |
| fp-lean, tpil | elan, python3, git | Lean 툴체인은 lake가 `lean-toolchain`대로 내려받음(CI와 동일) |
| thebeambook | asciidoctor-with-extensions(asciidoctor, -diagram, -pdf, rouge), jre(ditaa), graphviz, noto-fonts-cjk, rsync | PDF도 같은 셸에서 가능해짐 |
| pypy | pypy27, graphviz, noto-fonts-cjk | `pypy27.withPackages`가 깨져 있어 shellHook이 `ensurepip`으로 `build/pypy-bootstrap`에 pip·virtualenv 16.7을 넣고, 그 virtualenv로 `build/venv`에 Python 2 핀(`requirements.txt`)을 설치 |

pypy-eu-reports는 배포 타깃이 없고 소스가 커밋되지 않으므로 대상에서 뺍니다.

## CI

- `rebuild` 잡의 툴체인 설치 스텝을 모두 제거하고, `cachix/install-nix-action` +
  `nix-community/cache-nix-action` + `nix develop path:nix#<project> -c … yeokja build <target>`
  세 스텝으로 통일합니다.
- 캐시 키: `nix-${runner.os}-${project}-${target}-${hashFiles('nix/flake.lock', 'nix/projects/<project>.nix', 'projects/<project>/assets/flake.lock', 'projects/<project>/upstream/flake.lock')}`.
  `hashFiles`는 없는 파일을 무시하므로 napkin 전용 `nix_store` 필드는 없어집니다. 타깃까지
  키에 넣는 이유는 napkin pdf(upstream flake `nix build`)와 html/epub(devShell)의 스토어
  경로 집합이 달라 한 키를 공유하면 먼저 저장된 부분집합만 남기 때문입니다.
- `pages-projects.json`의 `toolchain` 필드는 더 이상 워크플로가 읽지 않으므로 제거합니다.
  `unshallow`, `rebuild_daily`는 유지합니다.
- 지문 게이트(`plan-rebuilds.sh`)는 변경하지 않습니다.
- `pr.yml`은 변경하지 않습니다(빌드하지 않음).

## 범위 밖

- yeokja CLI 자체를 Nix로 빌드하는 것(cargoHash 관리가 늘어남). CI는 기존 캐시된 바이너리를 씁니다.
- 순수 derivation으로의 이주.
- 프로젝트 빌드 결과의 변경. 산출물은 기존과 동일해야 합니다(mdbook 버전 통일에 따른
  사소한 마크업 차이는 허용하되 검증 스크립트를 통과해야 합니다).

