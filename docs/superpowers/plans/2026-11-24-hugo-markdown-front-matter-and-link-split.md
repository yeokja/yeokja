# Hugo 파서와 링크 안 문장 분리 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Hugo 콘텐츠 파일을 `parser = "hugo"`로 읽어 TOML/YAML 프런트매터와 단축 코드 줄을 세그먼트에서 빼고, 고른 프런트매터 값만 번역한다. Markdown 계열 문장 분리기는 링크·이미지 안에서 자르지 않는다.

**Architecture:** `parser-utils::split_markdown_sentences`(링크 범위 가림) → `parser-markdown-dialect`가 `make_markdown_segments`로 블록을 나눔. 프런트매터 도우미는 `parser-markdown-dialect::front_matter`로 모아 `rmarkdown`과 `hugo`가 함께 쓴다. `HugoParser`는 `parser-markdown` 크레이트에 있고 그림자에서 프런트매터와 단축 코드 줄을 지운다.

**Tech Stack:** Rust, pulldown-cmark.

**Spec:** `docs/superpowers/specs/2026-11-24-hugo-markdown-front-matter-and-link-split-design.md`

## Global Constraints

- `markdown`·`mkdocs`·`mdx`·`myst`·`rmarkdown` 등 기존 파서의 세그먼트는 R3 말고는 바뀌지 않는다.
- R3로 바뀌는 기존 프로젝트 세그먼트 수를 프로젝트별로 센다.
- 테스트를 먼저 실패시키고 구현한다.
- `crates/parser-rst`, `crates/translate/src/evaluator_format.rs`는 건드리지 않는다.

---

### Task 1: 링크 안 문장 분리 (R3)

**Files:** Modify `crates/parser-utils/src/sentence.rs`, `crates/parser-utils/src/lib.rs`, `crates/parser-markdown-dialect/src/lib.rs`, `crates/parser-rmarkdown/src/lib.rs`

- [x] 실패 테스트: 여러 문장 그림 설명, 링크 글자 안 문장, 참조 링크 `[…][ref]`, 목적지 안 괄호, 코드 스팬 안 대괄호, 닫히지 않은 `[`, `] (` 사이 공백.
- [x] `split_markdown_sentences`: 코드 스팬 밖의 `[`/`![`에서 링크 끝을 찾아 그 범위를 통째로 현재 문장에 붙인다.
- [x] `make_markdown_segments` 추가, dialect의 `flush_run`·`push_extra`와 rmarkdown `pieces`가 쓴다.
- [x] 파서 단위 테스트(`markdown`): 그림 설명이 한 세그먼트.

### Task 2: 프런트매터 모듈

**Files:** Create `crates/parser-markdown-dialect/src/front_matter.rs`; Modify `crates/parser-rmarkdown/src/lib.rs`

- [x] YAML 도우미를 옮기고 `rmarkdown`이 가져다 쓴다(기존 테스트 통과).
- [x] 실패 테스트 후 TOML: `toml_front_matter`, `toml_values`(표 머리, 점 키, 따옴표 키, basic/literal, 여러 줄·배열 제외), `quote_toml`(basic 이스케이프, literal → basic 승격).

### Task 3: `HugoParser`

**Files:** Create `crates/parser-markdown/src/hugo.rs`; Modify `crates/parser-markdown/src/lib.rs`

- [x] 실패 테스트: TOML 프런트매터 블랭크와 바이트 보존, 키 값 번역과 이스케이프, YAML 프런트매터 키, 단축 코드 줄(한 줄, 여러 줄, 펜스 안 제외, 붙은 본문 분리), 인라인 단축 코드 유지.
- [x] 구현: 그림자, Extra, 재구성 스플라이스.

### Task 4: 설정과 등록

**Files:** Modify `crates/core/src/config.rs`, `crates/parsers/src/lib.rs`, `docs/translation-project-checklist.md`

- [x] `PARSER_NAMES`·`INLINE_TAG_PARSERS`에 `hugo`. `front_matter_keys`는 `rmarkdown`·`hugo`에 허용.
- [x] `select_parser`가 `HugoParser::configured(keys)`를 만든다. 레지스트리 테스트.
- [x] 체크리스트에 Hugo 항목.

### Task 5: 검증

- [x] `cargo test --workspace`, `cargo clippy --workspace --all-targets` 경고 없음.
- [x] `cargo build --release -p yeokja-cli`, Markdown 계열 파서를 쓰는 모든 프로젝트에서 `reconcile --dry-run`과 `status --check`로 바뀌는 세그먼트 수를 센다(메인 체크아웃은 읽기만).
- [x] forge-lab 원문 두 편으로 세그먼트와 재구성 확인.
