# Rust API 가이드라인 한국어 번역

[rust-lang/api-guidelines](https://github.com/rust-lang/api-guidelines)의
mdBook 책 *Rust API Guidelines*(`src/**/*.md`) 전체를
[yeokja](https://github.com/yeokja/yeokja)의 `claude_code` provider로 한국어로
번역합니다. Anthropic 사의 `claude-opus-5-5` 모델을 활용하여 번역되었으며
학습을 모두 비허용한 상태로 작업하였습니다.
원문은 `upstream` 서브모듈 커밋 `97a0969cb07fe4cabb0eed8a56234053f47d83dc`로
고정하고, 번역문은 `state/`에 저장합니다. `ko/`, `build/`, `dist/`는 다시
생성할 수 있는 산출물입니다.

## 범위

- 번역 대상: `upstream/src/**/*.md` 15개 파일(`SUMMARY.md` 포함). 저장소
  루트의 `README.md`, `CONTRIBUTING.md`는 책 사이트에 나오지 않으므로
  제외합니다.
- 코드 블록, 인라인 코드, URL, 링크 참조 정의, `<a id="c-xxx">` 앵커는 원문을
  보존합니다. 가이드라인 식별자(`C-CASE`, `C-CONV` 등)는 제목과 체크리스트에서
  그대로 둡니다.
- Rust, Cargo, rustdoc, Serde 같은 이름과 newtype, unsafe 같은 Rust 관용구
  이름은 영문으로 둡니다. 용어집은 `glossary.toml`, 프로젝트 용어 기준은
  `terms.toml`에 있습니다.

## 번역과 검증

저장소 루트에서 실행합니다. 번역하려면 인증된 Claude Code CLI가 필요합니다.

```sh
git submodule update --init projects/rust-api-guidelines/upstream
cargo build --release -p yeokja-cli
target/release/yeokja -C projects/rust-api-guidelines translate upstream/src
target/release/yeokja -C projects/rust-api-guidelines status --check upstream/src
target/release/yeokja -C projects/rust-api-guidelines evaluate --mechanical-only upstream/src
```

## HTML 빌드와 Pages

Nix devShell이 mdbook과 python3 툴체인을 제공합니다.

```sh
cd projects/rust-api-guidelines
nix develop path:../../nix#rust-api-guidelines -c ../../target/release/yeokja build html
```

HTML은 `dist/site/`에 생성됩니다. 빌드는 영어 원본과 한국어판을 함께 만들고,
원문의 영어 제목 앵커를 한국어 페이지에 보존합니다
(`scripts/preserve_anchors.py`). 인쇄용 `print.html`은 장마다 앵커 이름을
구분해 합치고(`scripts/build_print.py`), 원문에 없던 깨진 로컬 링크가 생기면
`scripts/check_links.py`가 빌드를 실패시킵니다.

Deploy Pages 워크플로는 커밋된 번역 상태로 `ko/`를 재구성하고 `upstream/src`
전체의 미번역 여부를 검사한 뒤 빌드합니다. `dist-rust-api-guidelines` 산출물을
Pages의 `rust-api-guidelines/`에 배치합니다.

## 라이선스

원문과 번역문에는 원문의 MIT 또는 Apache-2.0 라이선스가 적용됩니다. 라이선스
전문은 `upstream/LICENSE-MIT`, `upstream/LICENSE-APACHE`에 있으며 배포
사이트 루트에도 함께 싣습니다.
