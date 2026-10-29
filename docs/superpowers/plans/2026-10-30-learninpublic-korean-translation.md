# The Coding Career Handbook 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** *The Coding Career Handbook*(Quarto book, CC BY 4.0)의 장 49개와 루트 문서 4개를 영어 → 한국어로 번역하고, 하위 경로(`/learninpublic/`)에서 동작하는 정적 Quarto 웹 판을 Pages 배포 배선까지 갖춘다.

**Architecture:** `upstream/` 서브모듈을 조립 트리의 바탕으로 두고 `ko/`(번역 미러)와 `overlay/`(한국어 `_quarto.yml`·`_quarto-web.yml`, 한글 SCSS, 쪽 클래스 스크립트)를 겹친다. 생성 단계 `prepare_site.py`가 권리 미확인 그림을 원문 링크 안내로 바꾸고 오디오북 목록을 정적으로 만들며, `book_map.py`가 번역된 프런트매터로 책 지도를 다시 만든다. `build.html`이 같은 Quarto로 영어 원문을 함께 빌드해 `check.py`로 비교한다.

**Tech Stack:** yeokja CLI(`rmarkdown` 파서, `inline_tags`, `front_matter_keys`, `claude_code` provider, `claude-opus-5-5`), Quarto 1.10.18(nixpkgs), Python 3 표준 라이브러리, Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-10-30-learninpublic-korean-translation-design.md`

## Global Constraints

- 원문은 `upstream` 서브모듈(`a87982e3`)이며 수정하지 않는다. 원문의 스크립트·`npm install`은 실행하지 않는다.
- `assets/rights.json`에서 `third-party-permission-required`·`unresolved`인 그림은 산출물에 들어가면 안 된다.
- 원문의 JavaScript 셸·플레이어·PWA 포함 파일은 쓰지 않는다(설계 3절).
- README 출처 표기는 AGENTS.md 규칙을 따르고, 모델명은 `state/`에 기록된 번역을 기준으로 확인한다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 루트 고정 패턴으로 무시한다(`overlay/ko/`가 가려지지 않게).
- `cargo build`를 돌리지 않는다. CLI는 `../../target/release/yeokja`.
- 공유 파일(`.github/**`, 루트 `LICENSE.md`, `site/index.html`, `.gitmodules`)은 통합자가 고친다.
- 커밋 메시지: `[learninpublic] <type>: <한국어 요약>`.

---

### Task 1: 프로젝트 골격

**Files:** Create `projects/learninpublic/{yeokja.toml,glossary.toml,.gitignore}`, `overlay/{_quarto.yml,_quarto-web.yml,styles/korean.scss,assets/ko/page-classes.html}`, `patches/magazine-fonts.patch`, `scripts/{prepare_site.py,book_map.py,check.py}`, `nix/projects/learninpublic.nix`

- [x] `yeokja.toml`: 소스 규칙 하나(`upstream`, `*.qmd`, `publishing/**` 제외, `front_matter_keys`), derive(ko·overlay, 생성 단계 2, 패치 1), `build.html`.
- [x] 용어집 24개(설계 2절).
- [x] `yeokja status upstream`으로 53개 파일과 8,708 세그먼트를 확인하고, `coverage`의 건너뛴 긴 구간이 코드·원시 HTML·단축코드뿐인지 본다.
- [x] `book_map.py`를 원문 `book-map.html`과 견준다: 영어 원문 트리에서 돌렸을 때 경로 접두(`/` → 상대)만 다르고 나머지가 같다.
- [x] `nix develop path:../../nix#learninpublic -c quarto --version`이 1.10.18.

### Task 2: 시험 번역

- [x] `yeokja translate upstream/chapters/principles-learn-in-public.qmd`: 재시도율, 합쇼체, 용어(공개적으로 배우기), `@sec-…` 뒤 조사 처리(`@{sec-x}를`), 강조·링크·`{fig-alt}` 보존을 `ko/` 재구성 diff로 본다.
- [x] 전체 번역 뒤 장마다 원문과 번역의 URL 집합을 비교해 `chapters/bibliography.qmd`의 "레이블: <URL>" 목록을 포함한 모든 URL이 보존됨을 확인.
- [x] 시험 번역 뒤 `yeokja assemble`과 Quarto 렌더링으로 생성 단계·패치·Quarto 설정을 확인(그림 22곳 대체, 책 지도 구조 일치, pandoc 3.10 필요).

### Task 3: 전체 번역과 용어 기준

- [x] `yeokja translate upstream`: 8,708 세그먼트, 1시간 22분, 재시도 17회, 최대 재시도 초과 1건(용어 어간 검사 오탐).
- [x] `yeokja evaluate --mechanical-only upstream` 0건(남는 건 state에서 번역을 지우고 다시 번역).
- [x] `yeokja terms build --dry-run` → `terms build` → `terms.toml` 검토(잘못 정해진 번역어 수정·삭제, `except`).
- [x] `glossary adopt --prefer HEAD`(816 수용) → 358 세그먼트 재번역(재시도 32회) → `status --check` 통과, `evaluate --mechanical-only` 0건.

### Task 4: 빌드와 검사

- [x] `nix develop path:../../nix#learninpublic -c ../../target/release/yeokja build html` → `dist/site`.
- [x] `check.py`: 53쪽, 제목 390개, 링크 8,418개 검사 통과(원문 빌드에만 있는 깨진 루트 절대 링크 106개는 보고만).
- [x] 브라우저(`/learninpublic/` 하위 경로로 서빙)에서 홈·책 지도, 장 페이지의 그림 대체 안내와 교차 참조, 오디오북 정적 목록, 퀵스타트를 확인.

### Task 5: README와 배포 배선

- [x] `README.md`: 범위, 제외한 것(그림·JavaScript 기능), 라이선스, 재현 명령, 구성, 출처 표기(AGENTS.md 규칙).
- [ ] 통합자에게 넘길 공유 파일 변경: `pages-projects.json`, `stage-pages.sh`, `rebuild-translations.sh` 분기(`status --check upstream` → `translate upstream`), 루트 `LICENSE.md` 표 한 줄, `site/index.html` 카드(`essays`).
