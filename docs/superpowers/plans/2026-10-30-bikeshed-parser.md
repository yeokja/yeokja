# Bikeshed 파서 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bikeshed 명세(`index.bs`)를 번역할 수 있는 `bikeshed` 파서를 추가하고, HTML 블록 안 들여쓴 산문을 잃는 `parser-markdown-dialect`의 결함을 고친다.

**Architecture:** `crates/parser-markdown-dialect`가 산문 덩어리의 공통 들여쓰기를 벗겨 읽고(`prose_extras`), `crates/parser-bikeshed`가 자동 링크·절 참조를 `bikeshed:` 스킴 링크로 고쳐 쓴 텍스트를 그림자와 함께 `parse_with`에 주고 재구성 뒤 되돌린다. 번역 쪽은 `Markup::Bikeshed`와 `BikeshedEvaluator`.

**Tech Stack:** Rust, pulldown-cmark(`parser-markdown-dialect`).

**Spec:** `docs/superpowers/specs/2026-10-30-bikeshed-parser-design.md`

## Global Constraints

- 기존 Markdown 계열 프로젝트의 세그먼트를 바꾸지 않는다(`status --check` 변경 0).
- 픽스처는 index.bs 발췌만 넣는다(MIT, lifthrasiir/aqueous).
- 테스트를 먼저 쓰고 구현한다.

---

### Task 1: 공통 들여쓰기 (`parser-markdown-dialect`)

**Files:** Modify `crates/parser-markdown-dialect/src/lib.rs`

- [x] 실패하는 테스트: 2칸 노트 안의 4칸 목록이 제공되고 무번역 재구성이 같다.
- [x] `dedent`(줄별 오프셋 표)와 `prose_extras`, `push_html_block`이 그것을 쓴다.
- [x] `html_opaque_ranges` 공개.

### Task 2: `bikeshed` 파서

**Files:** Create `crates/parser-bikeshed/{Cargo.toml,src/lib.rs,tests/fixtures/index-excerpt.bs}`; Modify `Cargo.toml`

- [x] 리터럴 영역, 요소 스택, 들여쓴 코드 분류(`analyze`).
- [x] `rewrite`/`restore`와 퍼센트 인코딩, 무번역 왕복.
- [x] 그림자(메타데이터 지움, 요소 안 들여쓴 코드 지움)와 extras(Abstract Literal, 산문).
- [x] `<dfn>` `lt=` 보강.
- [x] 픽스처 왕복 테스트.

### Task 3: 등록·Markup·평가기

**Files:** Modify `crates/core/src/{config,parser}.rs`, `crates/parsers/{Cargo.toml,src/lib.rs}`, `crates/translate/src/{orchestrator,evaluator_format,evaluator_glossary,evaluator_link,pipeline,prompt,lib}.rs`; Create `crates/translate/src/evaluator_bikeshed.rs`

- [x] `Markup::Bikeshed`, `PARSER_NAMES`·`INLINE_TAG_PARSERS`, `.bs` 자동 감지.
- [x] Markdown 계열 분기에 `Bikeshed` 추가, `closing_rule` 문장.
- [x] `BikeshedEvaluator`(참조·변수 다중집합) 등록과 테스트.
- [x] 코덱: 고쳐 쓴 링크 왕복 테스트.

### Task 4: 검증

- [x] `cargo test --workspace`, `cargo clippy --workspace --all-targets`.
- [x] index.bs `status`/`coverage`, 세 노트 제공 확인.
- [x] 기존 프로젝트 `status --check` 변경 0.
