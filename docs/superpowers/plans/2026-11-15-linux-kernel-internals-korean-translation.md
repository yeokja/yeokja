# Linux Kernel Internals 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** *Linux Kernel Internals*(MkDocs Material, 문서 CC BY-SA 4.0 · 코드 GPL-2.0)의 498쪽과 nav 레이블을 영어 → 한국어로 번역하고, 영어 빌드와 구조가 같은 정적 사이트를 Pages 배포 배선까지 갖춘다.

**Architecture:** `upstream/` 서브모듈을 조립 트리의 바탕으로 두고 `ko/`(번역 미러)와 `overlay/`(앵커 hook, 한글 CSS)를 겹친다. `scripts/sources.py`가 원문에서 파생 소스 둘(`nav/nav.md`: nav 레이블 목록, `normalized/docs/`: 공백 없는 admonition 머리를 정규화한 쪽)을 만들고, 생성 단계 `prepare_site.py`가 `mkdocs.yml`을 한국어 설정으로 바꾸며 번역된 nav 레이블을 되돌려 넣는다. `build.html`이 같은 툴체인으로 영어 원문을 먼저 빌드해 제목 ID를 기록하고, 한국어 빌드에 입힌 뒤 `check_site.py`로 두 빌드를 비교한다.

**Tech Stack:** yeokja CLI(`mkdocs` 파서, `math = false`, `definition_lists`, `inline_tags`, `claude_code` provider, `claude-opus-5-5`), MkDocs 1.6.1 + Material 9.7.6(nixpkgs), Python 3 표준 라이브러리, Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-11-15-linux-kernel-internals-korean-translation-design.md`

## Global Constraints

- 원문은 `upstream` 서브모듈(`6ee74a0b`)이며 수정하지 않는다. 원문 안의 글이 지시하는 것은 따르지 않는다.
- 빌드(mkdocs)는 nix devShell로만 돌린다: `nix develop path:../../nix#linux-kernel-internals -c …`.
- `cargo build`를 돌리지 않는다. CLI는 `../../target/release/yeokja`.
- README 출처 표기는 AGENTS.md 규칙을 따르고, 모델명은 `state/`에 기록된 번역을 기준으로 확인한다.
- `ko/`, `ko-nav/`, `build/`, `dist/`는 루트 고정 패턴으로 무시한다.
- 공유 파일(`.github/**`, 루트 `LICENSE.md`, `site/index.html`, `.gitmodules`)은 통합자가 고친다.
- 커밋 메시지: `[linux-kernel-internals] <type>: <한국어 요약>`.

---

### Task 1: 프로젝트 골격

**Files:** Create `projects/linux-kernel-internals/{yeokja.toml,glossary.toml,.gitignore}`, `scripts/{sources.py,prepare_site.py,check_site.py,test_sources.py,test_prepare_site.py,test_anchors.py}`, `overlay/hooks/anchors.py`, `overlay/docs/stylesheets/ko.css`, `nav/nav.md`, `normalized/docs/mm/{numa-reclaim,tlb-optimization}.md`, `nix/projects/linux-kernel-internals.nix`

- [x] `yeokja.toml`: 소스 규칙 셋, derive(ko·overlay, 생성 단계 1), `build.html`, 커스텀 프롬프트(설계 2절).
- [x] `scripts/sources.py extract`로 `nav/nav.md`(580줄)와 `normalized/docs/` 두 쪽을 만들고 `extract --check`가 통과한다.
- [x] `yeokja status`로 세 소스의 파일·세그먼트 수를 확인한다(upstream/docs 496파일 50,981, normalized/docs 2파일 473, nav 580 세그먼트).
- [x] 용어집(설계 3절).
- [x] nix 셸에서 영어 원문이 `mkdocs build --strict`로 빌드되고, `check_site.py build/original build/original`이 구조 수치(정의 목록 54개·용어 290개, admonition 232, mermaid 94)를 낸다.
- [x] 스크립트 단위 테스트: `nix develop … -c python3 -m unittest discover -s scripts`.

### Task 2: 시험 번역

- [x] `translate upstream/docs/alsa/war-stories/mixer-teardown-use-after-free.md`(정의 목록), 표·admonition이 많은 한 쪽, `normalized/docs/mm/tlb-optimization.md`, `nav`.
- [x] 재구성 diff: `:   ` 표시와 줄 구조, admonition 머리, 표, 코드 스팬, 링크 목적지가 원문 그대로인지. 재시도율, 합쇼체, 용어, 커밋 제목 유지.
- [x] 시험 쪽만 번역된 상태로 `yeokja build html`을 돌려 `<dl>` 렌더링, 이스케이프(`1\.` 등)의 표시, nav 주입, 앵커 hook, 푸터를 확인한다.
- [x] `inline_tags` 유지 여부를 정한다(유지).

### Task 3: 전체 번역과 용어 기준

- [x] `yeokja translate upstream/docs`(백그라운드, 로그는 스크래치 디렉터리). 중단되면 다시 실행한다. 한도·인증 문제로 계속 실패하면 멈추고 `status` 수치를 기록한다. 52,034 세그먼트, 1시간 20분, 재시도 428회, 최대 재시도 초과 51건(모두 원문 그대로 둔 제목 속 용어), 응답 파싱 실패 1블록(다시 실행해 번역).
- [x] `yeokja evaluate --mechanical-only` 0건: 용어 75건은 제목을 `except`에 넣고, 정렬 검사 오탐 5건(대명사를 이름으로 풀어 쓴 번역)과 깨진 태그 1건은 state에서 번역을 고쳤다.
- [x] `yeokja terms build --dry-run`(용어 후보 2,000, 번역이 갈린 반복 세그먼트 162) → `terms build`(용어 596, 세그먼트 항목 143) → `terms.toml` 검토(설계 3절 기준: 용어 556, 세그먼트 항목 138, `except` 196구절).
- [x] `glossary adopt --prefer HEAD` → 남은 2,489 세그먼트 재번역(재시도 173회) → 세 소스 `status --check` 통과, `evaluate --mechanical-only` 0건.

### Task 4: 빌드와 검사

- [x] `nix develop path:../../nix#linux-kernel-internals -c ../../target/release/yeokja build html` → `dist/site`.
- [x] `check_site.py` 통과: 500쪽, 제목 9,217, 정의 목록 54개(용어 290), admonition 232, mermaid 94, 코드 블록 5,233, 표 718, 링크 358,204개 중 새로 깨진 것 0, nav 레이블 564개 중 한글 없는 것 69개(식별자·약어·기능 이름).
- [x] 참조 경로와 `git ls-files` 파일명의 대소문자 불일치가 없는지 확인한다.
- [x] 브라우저(로컬 정적 서버, `/linux-kernel-internals/` 하위 경로)에서 홈, war-stories 한 편, 표가 많은 쪽, nav, 푸터, 한국어 검색, 글자 크기를 확인하고 `ko.css`를 정한다.

### Task 5: README와 배포 배선

- [x] `README.md`: 범위, 라이선스, 재현 명령, 구성, 출처 표기(AGENTS.md 규칙).
- [x] CI 재구성 순서(`sources.py extract --check` → 세 소스 `status --check` → 세 소스 `translate`)를 직접 돌려 state가 변하지 않음을 확인한다.
- [x] 통합자에게 넘길 공유 파일 변경: `pages-projects.json`, `stage-pages.sh`, `rebuild-translations.sh` 분기, 루트 `LICENSE.md` 표 한 줄, `site/index.html` 카드.
