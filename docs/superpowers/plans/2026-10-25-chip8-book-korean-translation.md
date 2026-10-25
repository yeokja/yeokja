# Chip-8 Emulation in Rust 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** pandoc용 원고인 `aquova/chip8-book`을 mdBook 책으로 내놓으며 한국어로 번역하고, Pages 배포 배선까지 갖춘다.

**Architecture:** 원문 서브모듈을 조립 트리의 바탕으로 두고 `ko/src` 번역 미러와 `overlay/ko`(한국어 `SUMMARY.md`, `metadata.yaml`)를 겹친다. 빌드는 영어 사본(`overlay/en`의 `SUMMARY.md`)과 조립 트리 양쪽에 같은 전처리(`prepare_book.py`)를 적용해 두 mdBook 빌드를 비교한다.

**Tech Stack:** yeokja CLI(`markdown` 파서, `inline_tags = true`, `claude_code` provider, `claude-opus-5-5`), mdBook 0.5, Python 3 표준 라이브러리, Nix devShell(mdbook, python3), GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-10-25-chip8-book-korean-translation-design.md`

## Global Constraints

- `upstream/`은 `e0b3888bed29d5e7201e26c8a8a7cbc3d6becc8f`에 고정하며 수정하지 않는다.
- 번역 대상은 `upstream/src/*.md` 9개 장(928 세그먼트).
- 전처리는 빌드 직전 사본에서만 한다. 원문과 `ko/`는 그대로 둔다.
- 4-methods의 콜론 빠진 각주는 원문대로 둔다. PDF 타깃은 만들지 않는다.
- `roms/`는 배포하지 않는다.
- README 출처 표기는 AGENTS.md 규칙을 따르고, 모델명은 `state/`에 기록된 번역을 기준으로 확인한다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 커밋 메시지: `[chip8-book] <type>: <한국어 요약>`.

---

### Task 1: 프로젝트 골격

**Files:** Create `projects/chip8-book/{yeokja.toml,glossary.toml,.gitignore,overlay/en/{book.toml,src/SUMMARY.md},overlay/ko/{book.toml,src/metadata.yaml}}`

- [x] `overlay/{en,ko}/book.toml`: 원문 `book.toml`에서 mdBook 0.5가 거부하는 `multilingual` 키를 뺀 사본(한국어판은 `language = "ko"`, 한국어 제목).
- [x] `yeokja.toml`: `upstream/src` `*.md` markdown 소스(`inline_tags = true`), `derive` 오버레이 `ko`(require_base)와 `overlay/ko`, `build.html`, provider·evaluation·translation 설정.
- [x] `glossary.toml`: spec의 용어 절.
- [x] 확인: `yeokja status upstream/src` → 9개 파일, 928 세그먼트.

### Task 2: 빌드 전처리와 opcode 검사

**Files:** Create `projects/chip8-book/scripts/{prepare_book.py,check_opcodes.py}`

- [x] `prepare_book.py <src>`: 코드 펜스 밖 단독 줄 `\newpage` 삭제, 헤딩 `{#id}` 매핑으로 파일 간 `](#id)` → `](file.md#id)`. 정의되지 않은 ID, 중복 ID, 줄 가운데 `\newpage`는 오류.
- [x] `check_opcodes.py <orig> <ko>`: 표 행과 opcode 제목의 기계 토큰 다중 집합 비교.
- [x] 확인: 원문 사본에 `prepare_book.py` → `\newpage` 10줄, 링크 4개, ID 3개. `check_opcodes.py upstream/src upstream/src` → 79행, 0건.

### Task 3: 시험 번역(5-instr)

- [x] `yeokja translate upstream/src/5-instr.md`(210 세그먼트) → 문체, 용어, 코드·`{#id}`·`](#id)` 보존, `check_opcodes.py` 0건 확인. 재시도 0회, 79행 중 5-instr 42행 0건.

### Task 4: 전체 번역과 용어 기준

- [x] `yeokja translate upstream/src`: 928개 전부, 재시도 3회.
- [x] `yeokja evaluate --mechanical-only upstream/src` → 0건. 남은 항목은 체크리스트 6절대로 state에서 지우고 다시 번역한다.
- [x] `check_opcodes.py upstream/src ko/src` → 0건.
- [x] `yeokja terms build --dry-run` → `yeokja terms build` → `terms.toml` 검토 → `yeokja glossary adopt --prefer HEAD` → 남은 세그먼트 `yeokja translate` → `status --check`, `evaluate --mechanical-only` 0건.
- [x] 결과: 후보 429개 중 55개 채택, 기존 번역 410개 수용, 6개 재번역. 이어 `font` → 폰트("글꼴"과 섞임), `guide` → 가이드("안내서"와 섞임)를 용어집에 더해 `glossary adopt` → 2개 재번역.

### Task 5: 목차와 HTML 빌드

**Files:** Create `projects/chip8-book/overlay/ko/src/SUMMARY.md`, `projects/chip8-book/scripts/{preserve_anchors.py,build_print.py,check_links.py}`(rust-api-guidelines 사본), `nix/projects/chip8-book.nix`

- [x] 한국어 `SUMMARY.md`: 항목을 `ko/src/*.md`의 첫 제목과 글자까지 같게 쓴다.
- [x] devShell: `mdbook`, `python3`.
- [x] 확인:

```sh
cd projects/chip8-book
nix develop path:../../nix#chip8-book -c ../../target/release/yeokja build html
```

  mdBook 경고 0, `check_links.py` `New: 0`, `dist/site/2-basics.html`에 `id="eb"`, `dist/site/5-instr.html`의 opcode 제목에 영어 제목 ID(`id="8xy4---vx--vy"` 등) 보존, `\newpage` 글자 없음.
- [x] 브라우저로 첫 장, 5-instr, opcode 표, 장 간 링크를 확인한다.

### Task 6: README와 배포 배선

**Files:** Create `projects/chip8-book/README.md`; Modify(호출 측) `.github/pages-projects.json`, `.github/scripts/{stage-pages.sh,rebuild-translations.sh}`, `site/index.html`, `LICENSE.md`

- [x] README: 범위, 고정 커밋, CC0·`roms/` 제외, 재현 명령, 빌드 설명, 출처 표기.
- [x] 배포 배선은 spec의 배포 절.
- [x] `bash .github/scripts/rebuild-translations.sh chip8-book`이 provider 호출 없이 통과하는지 확인.
