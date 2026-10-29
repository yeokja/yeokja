# Aqueous Archive Format 명세 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bikeshed 명세 `spec/index.bs`를 `bikeshed` 파서로 번역하고, 원문과 번역을 같은 Bikeshed 7.2.0으로 빌드해 골격이 같음을 검사한 한국어 페이지를 Pages 배포 배선까지 갖춘다.

**Architecture:** `upstream` 서브모듈(3d37dae5)을 조립 트리의 바탕으로 두고 `ko/spec/index.bs`를 겹친다. `scripts/build.py`가 번역 파일에 메타데이터 블록(제목·상태·스냅샷 날짜·경고 상자·Translation)을 덧붙여 원문과 함께 Bikeshed로 빌드하고, `scripts/check.py`가 두 HTML의 id·요소·dfn·제목·내부 링크를 대조한 뒤 Bikeshed 보일러플레이트 문자열만 한국어로 바꿔 `site/`를 만든다.

**Tech Stack:** yeokja CLI(`bikeshed` 파서, 인라인 태그, `claude_code` provider, `claude-opus-5-5`), Python 3 표준 라이브러리, PyPI `bikeshed==7.2.0`(python3 + uv venv), Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-10-30-aqueous-korean-translation-design.md`

## Global Constraints

- `upstream/`은 수정하지 않는다. 원문 사본을 오버레이에 두지 않는다(매주 바뀐다).
- 번역하지 않는 것: 메타데이터(Abstract 제외), `<pre>`, 16진 상수, 식별자형 정의, 명령 이름, 레지스트리 이름, CDDL, 저작권 문단.
- README 출처 표기는 AGENTS.md 규칙을 따르고, 모델명은 `state/`에 기록된 번역을 기준으로 확인한다.
- `ko/`, `build/`, `dist/`, `site/`, `__pycache__/`는 커밋하지 않는다(`.gitignore`는 루트 고정 패턴).
- 커밋 메시지: `[aqueous] <type>: <한국어 요약>`.

---

### Task 1: 프로젝트 골격

**Files:** Create `projects/aqueous/{yeokja.toml,glossary.toml,requirements.txt,.gitignore}`, `nix/projects/aqueous.nix`

- [x] `yeokja.toml`: `bikeshed` 소스, `inline_tags`, `derive` 오버레이, `build.html` → `scripts/build.py`, 커스텀 `prompt_template`(합쇼체·코드 스팬·`{inline_tags}`·`|var|`·`[[…]]`·식별자·RFC 2119·문자 참조 규칙), `[translation] concurrency = 6, batch_segments = 32`.
- [x] `glossary.toml`: 저자의 `docs/` 한국어를 우선한 용어, 복수형, 문서 제목.
- [x] `requirements.txt`: `bikeshed==7.2.0`. nix devShell: `python3 + uv`, `pythonVenv { requirements = "requirements.txt"; }`.
- [x] 확인: `yeokja status upstream/spec` → 2,471 세그먼트. venv의 `bikeshed --die-on=warning`이 원문을 약 2초에 빌드(lint 1건, 경고 0건).

### Task 2: 시험 번역(1–399행 발췌)

- [x] 발췌를 scratch 프로젝트에 복사해 `translate`: 203 세그먼트, 27초, 재시도 2건.
- [x] 검토: `[=key=]`·`[=key|text=]` 되돌리기, `lt=` 보강, `|var|` 보존, 합쇼체, 용어집. 복수형("shorthand blocks", "formulas", "entries")이 용어집을 비켜 가 복수형 항목을 더한다. `&lt;&lt;`가 `<<`로 나와 Bikeshed가 CSS 생성 규칙으로 읽는다 → 프롬프트 규칙과 `scripts/entities.py`.
- [x] 발췌에 메타데이터 블록을 덧붙여 Bikeshed로 빌드: 원문 발췌와 메시지 집합이 줄 번호·dfn 글자만 다르고, id 집합·dfn 수(38)·요소 다중집합이 같다. 제목·`편집자 초안`·날짜·경고 상자·Translations가 머리말에 나온다.

### Task 3: 전체 번역과 용어 기준

- [x] `yeokja translate upstream/spec`: 첫 파서로 2,690 세그먼트, 5분 39초, 재시도 44건(최대 재시도 초과 7건, 모두 식별자 세그먼트의 용어집 오탐). 파서가 HTML 블록 안 인라인 태그와 `: `/`:: ` 정의 목록을 다루게 된 뒤 `reconcile`(kept 2,253 / added 218 / dropped 437 / context_hash 186) → 218개 번역(28초, 재시도 1건) → 2,471 세그먼트. `scripts/entities.py check` 0건.
- [x] `evaluate --mechanical-only`: 7건 → 필드 이름 dfn과 용어의 충돌은 `[whole]`·`except`로, `checksum` 단수는 용어집에서 빼서 0건.
- [x] `terms build`(844 후보 → 191 용어, whole 6) → 검토(`Certificates`, 필드 이름과 충돌하는 `[whole.Type]` 삭제, `|var|` 안의 용어에 `except`) → `glossary adopt --prefer HEAD` → 115개 재번역(재시도 6건).
- [x] 확인: `status --check upstream/spec` 통과, `evaluate --mechanical-only` 0건. 상태에서 직접 고친 것: `Issue:` 접두사 2건, 중복 링크 1건, 중복 코드 스팬 1건, 필드 이름 3건.

### Task 4: 빌드와 검사

**Files:** Create `projects/aqueous/scripts/{build.py,check.py,entities.py}`

- [x] `build.py ROOT TREE`: 메타데이터 블록 덧붙이기(`<h1>`이 `Title`과 다르면 실패), 원문·번역 Bikeshed 빌드(`--die-on=warning`, 산출 파일 부재도 실패), `check.py`, 보일러플레이트 치환(횟수 불일치면 실패), 카운터 라벨 CSS, `lang="ko"`, `site/`.
- [x] `check.py EN KO`: 머리말 제외 id 집합·요소 다중집합·dfn 수·수준별 제목 수 동일, 내부 링크 해소, id 중복 없음, `bikeshed:` href 없음. 발췌로 검증.
- [x] `entities.py check|fix`: 원문이 문자 참조를 쓴 세그먼트의 번역을 다시 이스케이프.
- [x] 확인:

```sh
cd projects/aqueous
nix develop path:../../nix#aqueous -c ../../target/release/yeokja build html
# check: ok — 1935 ids, 8713 elements, 326 dfns, 96 headings, 2327 internal links resolve
```

- [x] 브라우저(`dist/site`)로 머리말·경고 상자·Translations, 블록 구조 표, 노트 안 들여쓴 목록, `<sub>`가 있는 알고리즘 목록, `: `/`:: ` 정의 목록, `&lt;&lt;`·`&thinsp;` 자리, dfn 패널("참조된 곳:"), 색인 제목을 확인했다.

### Task 5: README와 배포 배선

- [x] README: 범위, 스냅샷 커밋·비공식 고지, 알려진 한계, 라이선스(공개 도메인/MIT-0), RFC 2119 대응표, 재현, 출처 표기.
- [ ] 호출 측: `.github/pages-projects.json`, `stage-pages.sh`, `rebuild-translations.sh` 분기(`status --check upstream/spec` → `translate upstream/spec`), 루트 `LICENSE.md`, `site/index.html`의 새 주제 "데이터 형식" 카드.
