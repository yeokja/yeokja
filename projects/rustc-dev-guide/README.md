# Rust 컴파일러 개발 가이드 한국어 번역

[rust-lang/rustc-dev-guide](https://github.com/rust-lang/rustc-dev-guide)의
`src/**/*.md` 전체와 HTML 블록 안의 경고문·접기 제목을 yeokja의 `claude_code` provider로 한국어로 번역합니다.
원문은 `upstream` 서브모듈 커밋으로 고정하고, 번역문은 `state/`에 저장합니다.
`ko/`, `build/`, `dist/`는 다시 생성할 수 있는 산출물입니다.

## 번역과 검증

저장소 루트에서 실행합니다. 번역하려면 인증된 Claude Code CLI가 필요합니다.

```sh
git submodule update --init projects/rustc-dev-guide/upstream
cargo build --release -p yeokja-cli
target/release/yeokja -C projects/rustc-dev-guide translate upstream/src
target/release/yeokja -C projects/rustc-dev-guide status --check upstream/src
target/release/yeokja -C projects/rustc-dev-guide coverage upstream/src
```

용어집은 `glossary.toml`에 있습니다. 코드, 파일 경로, 링크 주소는 보존하며
원문 제목의 앵커는 HTML 빌드 시 원문 렌더링 결과에서 가져와 유지합니다.

## HTML 빌드와 Pages

원문의 CI와 같은 도구 버전을 사용합니다.

```sh
cargo install --locked mdbook --version 0.5.2
cargo install --locked mdbook-mermaid --version 0.17.0
target/release/yeokja -C projects/rustc-dev-guide build html
```

HTML은 `dist/site/`에 생성됩니다. 기존 Deploy Pages 워크플로는 커밋된
번역 상태로 `ko/`를 재구성하고, `upstream/src` 전체의 미번역 여부를 검사한
뒤 빌드합니다. `dist-rustc-dev-guide` 산출물을 Pages의 `rustc-dev-guide/`에
배치합니다. Mermaid 도표를 위한 JavaScript와 원문 정적 자산도 함께 배포합니다.

원문과 번역문에는 원문의 MIT 또는 Apache-2.0 라이선스가 적용됩니다.
라이선스 전문은 `upstream/LICENSE-MIT`, `upstream/LICENSE-APACHE`를 참조하세요.

개별 HTML 페이지는 한국어 제목 앵커와 원문 제목 앵커를 함께 제공합니다.
인쇄용 `print.html`은 한국어 개별 페이지를 목차 순서대로 합칩니다. 장마다
앵커 이름을 구분하고 링크·이미지 경로를 함께 바꿔, 다른 장의 같은 이름을 가진
절이나 용어집 항목으로 잘못 연결되지 않게 합니다.
