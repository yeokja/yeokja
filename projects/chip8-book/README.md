# Rust로 배우는 Chip-8 에뮬레이션 입문 (한국어 번역)

[aquova/chip8-book](https://github.com/aquova/chip8-book)의 *An Introduction to
Chip-8 Emulation using the Rust Programming Language*(aquova)를 한국어로 옮긴
비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic
사의 `claude-opus-5-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한
상태로 작업하였습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `e0b3888bed29d5e7201e26c8a8a7cbc3d6becc8f`
  (2023-08-13, `master`)
- 번역 대상: `upstream/src/*.md` 9개 장(928 세그먼트)
- 코드 블록, 인라인 코드, URL, opcode 표기(`00E0`, `DXYN`), 레지스터 이름
  (V0–VF, I, PC, SP, DT, ST), 16진수 리터럴은 원문을 유지합니다. 이미지 대체
  텍스트는 영어로 남습니다. 용어집은 `glossary.toml`, 프로젝트 용어 기준은
  `terms.toml`에 있습니다.
- 원문은 pandoc으로 PDF/EPUB을 만드는 원고이며 mdBook 목차가 없습니다. 이
  번역은 mdBook HTML로 내놓으며, 원문에 없는 목차(`SUMMARY.md`)와 mdBook
  0.5용 `book.toml`, 한국어 pandoc 표제 정보(`metadata.yaml`)를 `overlay/`에
  손으로 써 둡니다(`overlay/en`은 비교용 영어 빌드에만 씁니다).
- 저장소의 `README.md`, `code/`, `roms/`는 책에 나오지 않으므로 번역하지
  않습니다.

## 라이선스

원문은 CC0 1.0(`upstream/LICENSE`)이며 이 번역도 CC0 1.0으로 제공합니다.
배포 사이트 루트에 원문 `LICENSE`를 함께 싣습니다. 원문 저장소의 `roms/`는
저작자를 알 수 없는 제3자 ROM이라 배포하지 않습니다(mdBook은 `src/`만
내놓습니다). 저자의 공식 번역이 아닙니다.

## 재현

```sh
cd projects/chip8-book
../../target/release/yeokja translate upstream/src      # 누락 세그먼트 번역 및 ko/ 재구성
../../target/release/yeokja status --check upstream/src
../../target/release/yeokja evaluate --mechanical-only upstream/src
nix develop path:../../nix#chip8-book -c ../../target/release/yeokja build html   # dist/site
```

번역하려면 인증된 Claude Code CLI가 필요합니다. `state/`는 번역의 진실의
원천이므로 커밋합니다. `ko/`, `build/`, `dist/`는 `state/`와 원문에서 언제든
재생성되므로 커밋하지 않습니다.

Nix devShell(`nix/projects/chip8-book.nix`)이 mdbook과 python3를 제공합니다.
빌드는 다음 순서로 진행합니다.

1. `scripts/check_opcodes.py`: opcode 표의 행과 `### 8XY4 - VX += VY` 같은
   opcode 제목마다 opcode·레지스터·16진수·연산자 토큰이 원문과 같은지 확인합니다.
2. `scripts/prepare_book.py`: pandoc 원고를 mdBook에 맞춥니다. 단독 줄
   `\newpage`를 지우고, 다른 장의 `{#id}` 제목을 가리키는 `](#id)` 링크를
   `](file.md#id)`로 바꿉니다. 원문 사본과 번역 트리 양쪽에 똑같이 적용합니다.
3. 영어 원본과 한국어판을 mdBook으로 빌드하고(경고가 있으면 실패), 원문의 영어
   제목 앵커를 한국어 페이지에 보존합니다(`scripts/preserve_anchors.py`).
4. 인쇄용 `print.html`을 장마다 앵커 이름을 구분해 다시 합치고
   (`scripts/build_print.py`), 영어 빌드에 없던 깨진 로컬 링크가 생기면
   `scripts/check_links.py`가 빌드를 실패시킵니다.

Deploy Pages 워크플로는 커밋된 번역 상태로 `ko/`를 재구성하고 `upstream/src`
전체의 미번역 여부를 검사한 뒤 빌드해, `dist-chip8-book` 산출물을 Pages의
`chip8-book/`에 배치합니다.
