# Write You a Haskell (한국어 번역)

[sdiehl/write-you-a-haskell](https://github.com/sdiehl/write-you-a-haskell)의
mdBook 책 *Write You a Haskell*(`book/`)을 한국어로 옮긴 비공식 번역입니다.
[yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의
`claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로
작업하였습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `4333ed8c95f016470ca30c950e1744873f004bab`
  (2026-10-13, `main`)
- 번역 대상: `upstream/book/src/**/*.md` (25개 장과 부록 5개, `SUMMARY.md` 포함)
- 코드 블록, `{{#include}}`로 책에 끼워 넣어지는 `crates/`의 Rust 소스,
  `$`/`$$` 수식, `[@key]` 인용 키, 인라인 코드, URL은 원문을 유지합니다.
- mdbook-bib가 생성하는 참고문헌 페이지와 저장소의 README, `BUILDING.md`,
  `CHANGELOG.md` 등 책 밖 문서는 제외합니다.
- 책이 만드는 언어 이름 Fun, IR 이름 Core/ANF/Flat, Prelude, LLVM/JIT/REPL은
  영문으로 둡니다. 용어집은 `glossary.toml`, 프로젝트 용어 기준은 `terms.toml`에
  있습니다.

## 라이선스

원문의 산문과 원본 그림은 CC BY-SA 4.0(`upstream/LICENSE-PROSE`), 코드는
MIT(`upstream/LICENSE`)입니다. 이 번역은 원문 산문을 개작한 2차적 저작물로서
같은 CC BY-SA 4.0으로 제공하며, 배포 사이트 루트에 원문의 두 라이선스 파일을
함께 싣고 모든 본문 페이지 하단에 저작자(Stephen Diehl), 원문 링크, 번역·수정
사실을 표시합니다. 책에 끼워 넣어진 코드는 번역하지 않으므로 MIT 그대로입니다.
저자의 공식 번역이 아닙니다.

## 재현

```sh
cd projects/write-you-a-haskell
../../target/release/yeokja translate upstream/book/src      # 누락 세그먼트 번역 및 ko/ 재구성
../../target/release/yeokja status --check upstream/book/src
nix develop path:../../nix#write-you-a-haskell -c ../../target/release/yeokja build html   # dist/site
```

번역하려면 인증된 Claude Code CLI가 필요합니다. `state/`는 번역의 진실의
원천이므로 커밋합니다. `ko/`, `build/`, `dist/`는 `state/`와 원문에서 언제든
재생성되므로 커밋하지 않습니다.

Nix devShell(`nix/projects/write-you-a-haskell.nix`)이 원문 `book.toml`이
요구하는 mdbook, mdbook-katex, mdbook-bib(저자 패치 적용), 저자의
mdbook-include-rs 포크, pandoc(참고문헌 페이지 렌더링용), python3를 제공합니다.

빌드는 번역문의 수식·`{{#include}}`·인용 키가 원문과 파일별로 같은지 먼저
확인하고(`scripts/check_markers.py`), 영어 원본과 한국어판을 함께 만든 뒤
원문의 영어 제목 앵커를 한국어 페이지에 보존합니다(`scripts/preserve_anchors.py`).
인쇄용 `print.html`은 한국어 개별 페이지를 목차 순서대로 다시
합치고(`scripts/build_print.py`), 원문에 없던 깨진 로컬 링크가 생기면
`scripts/check_links.py`가 빌드를 실패시킵니다. Deploy Pages 워크플로는 커밋된
번역 상태로 `ko/`를 재구성하고 `upstream/book/src` 전체의 미번역 여부를 검사한
뒤 빌드해, `dist-write-you-a-haskell` 산출물을 Pages의 `write-you-a-haskell/`에
배치합니다.
