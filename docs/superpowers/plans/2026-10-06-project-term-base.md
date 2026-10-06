# 프로젝트 용어 기준 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 프로젝트 전체에서 반복되는 용어와 짧은 세그먼트의 번역어를 한 번 정해 `terms.toml`에
저장하고, 용어집 기제(요청 주입, 평가, 스냅숏)로 모든 번역에 적용한다.

**Architecture:** 코어의 `Glossary`가 세그먼트 항목(`[whole]`)과 판정 함수 `unmet_terms`를 얻고,
`glossary.toml`과 `terms.toml`을 합쳐 읽는다. translate crate의 새 모듈 `terms`가 결정적 채굴,
모델 선별·결정 프롬프트와 응답 파싱, 스냅숏 받아들이기를 맡는다. CLI는 `terms build`와
`glossary adopt`를 더한다.

**Tech Stack:** Rust 2024 workspace, serde/toml, tokio.

**Spec:** `docs/superpowers/specs/2026-10-06-project-term-base-design.md`

## Global Constraints

- 커밋은 Secretive(Touch ID) SSH 서명이다. 서명을 끄거나 우회하지 않는다. 사람이 없으면 변경을
  커밋하지 않은 채 두고, 계획한 커밋을 순서대로 만드는 스크립트를 스크래치에 남긴다.
- 커밋 메시지 형식은 저장소 관례(`[core] feat: …`, `[translate] feat: …`, `[cli] feat: …`).
- 작업 트리의 `projects/peps/state` 변경은 이 작업과 무관하다. 건드리지도, 스테이징하지도 않는다.

---

### Task 1: 코어 — 세그먼트 항목, `unmet_terms`, 용어 기준 파일

**Files:** Modify `crates/core/src/glossary.rs`, `crates/core/src/config.rs`, `crates/core/src/project.rs`

- [ ] `GlossaryFile.whole`, `Glossary.whole`(정규화 키 → (표기, 번역어)), `pub fn normalize_whole(&str) -> String`.
- [ ] `find_matching_terms`가 세그먼트 항목을 포함하도록 바꾸고, `pub fn whole_match(&self, source) -> Option<(&str, &str)>`.
- [ ] `pub fn unmet_terms(&self, source, translation) -> Vec<(String, String)>`(정렬된 결과).
- [ ] `TermsFile { terms: BTreeMap, whole: BTreeMap, rejected: Rejected { terms: BTreeSet, whole: BTreeSet } }`, `load`/`save`(정렬, 머리 주석).
- [ ] `Glossary::merged(curated, terms) -> Glossary`(curated 우선), `Glossary::load_project(dir, settings)`.
- [ ] `ProjectSettings.terms`(기본 `"terms.toml"`). `ProjectContext::load`가 `load_project`를 쓴다.
- [ ] 단위 테스트(spec 7절의 코어 항목). `cargo test -p yeokja-core`.

### Task 2: 평가기와 서버

**Files:** Modify `crates/translate/src/evaluator_glossary.rs`, `crates/server/src/state.rs`

- [ ] `pub fn glossary_source(markup, source) -> String`(기존 전처리를 함수로 뺀다).
- [ ] 평가기가 `unmet_terms`를 쓰고, 세그먼트 항목에는 "Segment … should be translated as …" 메시지.
- [ ] 서버 상태가 `load_project`로 읽는다.
- [ ] 세그먼트 항목 평가 테스트. `cargo test -p yeokja-translate evaluator_glossary`.

### Task 3: 채굴

**Files:** Create `crates/translate/src/terms.rs`(채굴, 프롬프트, 파싱, 받아들이기), Modify `crates/translate/src/lib.rs`

- [ ] `pub struct Occurrence { file, source, translation: Option<String> }`, `pub struct Corpus`.
- [ ] `prose_chunks(source) -> Vec<Vec<String>>`, 기능어 목록, n-gram 집계, 닫힌 n-gram 걸러내기, 표기 선택.
- [ ] `pub fn mine(corpus, known: &HashSet<String>, min_count, limit) -> Candidates { terms: Vec<TermCandidate>, whole: Vec<WholeCandidate> }`, 예문은 서로 다른 파일 우선 5개.
- [ ] `pub fn collect_corpus(config, glossary, parser_factory) -> Corpus`(`collect_files` + `scan_file`).
- [ ] 단위 테스트(spec 7절의 채굴 항목).

### Task 4: 선별·결정 프롬프트와 실행

- [ ] `screen_prompt`, `parse_screen_reply`, `decide_prompt`, `parse_decide_reply`(번역어 검증).
- [ ] `pub async fn build(llm, corpus, terms_file, opts) -> BuildReport`: 묶음 동시 실행(세마포어), 실패 묶음은 기록하지 않음.
- [ ] 파싱 테스트, 가짜 `LlmProvider`로 `build` 테스트.

### Task 5: 받아들이기와 CLI

**Files:** Create `crates/cli/src/commands/terms.rs`, Modify `crates/cli/src/commands/glossary.rs`, `mod.rs`, `main.rs`

- [ ] `pub fn adopt_file(state, glossary, markup) -> usize`, `pub fn adopt_path(path, config, glossary, factory) -> AdoptReport`(남은 stale 용어별 수).
- [ ] `yeokja glossary adopt [path]`, `yeokja terms build [--dry-run] [--min-count N] [--limit N]`.
- [ ] 받아들이기 테스트. `cargo test --workspace`, `cargo clippy --workspace`.

### Task 6: 문서와 시범 적용

- [ ] `docs/translation-project-checklist.md`에 용어 기준 절차 한 절.
- [ ] putting-the-you-in-cpu에서 `terms build --dry-run`, 전체 실행, `terms.toml`과 남은 stale 검토.
