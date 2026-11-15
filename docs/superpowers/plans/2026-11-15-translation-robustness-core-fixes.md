# 번역 견고성 코어 수정 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `mkdocs` 파서가 공백 없는 admonition 머리를 읽고, 인라인 태그 직렬화가 글자 대괄호를 링크로 만들지 않으며, 깨진 태그가 든 번역이 저장되지 않고 `evaluate`에 걸리고, 용어 검사가 옮기지 않은 제목 속 용어·복수형·Glossary stale 세그먼트를 다루고, 번역할 글자가 없는 세그먼트가 provider로 가지 않게 한다.

**Architecture:** 파서(`parser-mkdocs`의 `scan_block_header`) → 인라인 코덱(`translate::inline::markdown`의 `read`·`Renderer`) → 파이프라인(`translate::pipeline`의 태그 문제 경로) → 평가(`inline::audit::gained`, `cli evaluate`) → 용어집(`core::glossary`의 `term_uses`·`unmet_terms`, `ProjectSettings.glossary_plurals`) → 오케스트레이터(`translate_file`이 요청을 만들기 전 `settle_untranslatable`로 번역할 글자가 없는 세그먼트를 기록).

**Tech Stack:** Rust, pulldown-cmark(변경 없음), `serde`.

**Spec:** `docs/superpowers/specs/2026-11-15-translation-robustness-core-fixes-design.md`

## Global Constraints

- 기존 프로젝트의 세그먼트와 state 상태를 바꾸지 않는다. 모든 Pages 프로젝트에서 변경 전·후 `status --check`의 출력과 종료 코드가 같아야 한다.
- 테스트를 먼저 실패시키고 구현한다. provider는 가짜로만 부른다.
- 커밋은 결함별로 나눈다. 메시지: `[core] fix: …` 한국어 요약.

---

### Task 1: 공백 없는 admonition 머리 (`parser-mkdocs`)

**Files:** Modify `crates/parser-mkdocs/src/lib.rs`

- [x] 실패하는 테스트: `!!!note`, `!!!tip "제목"`, `???info`, `???+warning "제목"`, `???a`가 머리이고 본문·목록이 따로 나온다; 공백 있는 꼴과 세그먼트가 같다; `!!!!`, `???what is this?`, `!!!"제목"`, `!!!note: text`, `==="Tab"`은 글자다.
- [x] `is_unspaced_header_tail`과 4바이트 머리의 그림자(`-  -`).

### Task 2: 글자 대괄호 (`translate::inline::markdown`)

**Files:** Modify `crates/translate/src/inline/markdown.rs`

- [x] 실패하는 테스트: `[59:56](top-byte ignore 영역)`, `[a][b]`, MkDocs의 `[a] [b]`, 줄 머리 `[a]: b`, 정의된 레이블과 같은 `[label]`, 각주 꼴, 링크 글자 안의 대괄호, 링크 앞의 `!`, 글자 `]` 뒤에 링크 태그가 오는 경우. 방언마다 쓴 결과가 `verified`이고 다시 읽은 링크 수가 태그 수와 같다.
- [x] `Renderer::escape`에 "바로 뒤 글자"와 링크 글자 안 여부를 넘기고 표의 꼴을 막는다.
- [x] `render_noting`이 글자가 쓴 `]`의 위치를 함께 내고 `unsafe_closes`로 확인한다. `Rendered::mismatch`.

### Task 3: 깨진 태그 (`translate::inline::markdown`, `translate::pipeline`, `inline::audit`, `cli evaluate`)

**Files:** Modify `crates/translate/src/inline/markdown.rs`, `crates/translate/src/inline/audit.rs`, `crates/translate/src/pipeline.rs`, `crates/translate/src/orchestrator.rs`, `crates/cli/src/commands/evaluate.rs`

- [x] 실패하는 테스트: `<b>`가 든 응답은 `read`의 문제다(Plain 위치와 원문 산문에 있는 글자는 예외); 확인되지 않은 직렬화는 그 세그먼트만 피드백과 함께 재시도되고 끝까지 남으면 결과에 없다.
- [x] `Token::Bare`, `source_verifies`, 파이프라인의 unverified → 태그 문제 경로(원문 자신이 확인되지 않는 세그먼트는 경고와 함께 저장).
- [x] `audit::gained`와 테스트(태그 잔재, 새 링크, 원문에 있는 `<b>`는 통과).
- [x] `evaluate`가 인라인 태그 파일에서 `InlineStructure`를 낸다.

### Task 4: 원문 그대로 남긴 구간 (`core::glossary`)

**Files:** Modify `crates/core/src/glossary.rs`, `crates/translate/src/evaluator_glossary.rs`

- [x] 실패하는 테스트: 링크 글자·큰따옴표·강조 구간이 번역에 그대로 있으면 그 안의 용어는 충족; 구간이 용어와 같으면 요구; 구간 밖에도 나오면 요구; 구간이 번역에 없으면 요구; 스냅숏 판정은 그대로.
- [x] `term_uses`가 용어와 출현 범위를 돌려주고 `unmet_terms`가 `delimited_spans`·`kept_as_written`으로 거른다.

### Task 5: `evaluate`의 대상 (`cli evaluate`)

**Files:** Modify `crates/cli/src/commands/evaluate.rs`, `crates/core/src/change.rs`

- [x] 실패하는 테스트: `SegmentStatus::has_current_translation`이 `Translated`·`GlossaryStale`·`ContextChanged`에서 참이다.
- [x] `evaluate`가 그 집합을 보고 로그에 `status`, 요약에 stale 수를 적는다.

### Task 6: 복수형 (`core::glossary`, `core::config`)

**Files:** Modify `crates/core/src/glossary.rs`, `crates/core/src/config.rs`, `crates/translate/src/terms.rs`

- [x] 실패하는 테스트: 옵션이 켜지면 `huge page`가 `huge pages`에, `patch`가 `patches`에, `policy`가 `policies`에 맞는다; 꺼져 있으면 지금과 같다; 긴 항목이 먼저다; `C++`은 그대로다.
- [x] `ProjectSettings.glossary_plurals`, `Glossary::load_project`가 넘긴다.
- [x] `terms build`가 켠 프로젝트에서 규칙 복수형 꼴의 항목을 쓰지 않는다(`DecideOptions::plurals`).

### Task 7: 번역할 글자가 없는 세그먼트 (`translate::orchestrator`)

**Files:** Modify `crates/translate/src/orchestrator.rs`, Create `crates/translate/src/untranslatable.rs`

- [x] 실패하는 테스트: `` `foo()` ``, `—`, `4 KB`, `https://…`, `` [`x`](y.md) ``는 번역할 글자가 없다; `foo`, `Yes`, `10 s`, RST의 `` `text`_ ``는 있다; 코드 스팬뿐인 세그먼트는 provider 호출 없이 원문으로 저장된다; 용어집이 맞는 세그먼트는 provider로 간다.
- [x] `settle_untranslatable`: `translate_file`과 `plan_requests`가 그룹을 만들기 전에 기록하고, `translate_file`은 state를 바로 저장한다.

### Task 8: 문서와 검증

**Files:** Modify `docs/translation-project-checklist.md`

- [x] 체크리스트에 `glossary_plurals`, 옮기지 않는 제목과 용어 검사, `evaluate`의 대상과 `InlineStructure`, 공백 없는 admonition 머리를 적는다.
- [x] `cargo test --workspace`, `cargo clippy --workspace --all-targets`.
- [x] 모든 프로젝트에서 변경 전·후 바이너리의 `status --check` 출력과 종료 코드가 같다.
- [x] `mkdocs`·인라인 태그 프로젝트의 `evaluate --mechanical-only` 전후 비교.
