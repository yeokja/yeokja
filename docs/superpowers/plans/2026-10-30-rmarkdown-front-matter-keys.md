# R Markdown 프런트매터 키 옵션 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rmarkdown` 소스가 `front_matter_keys`로 중첩 프런트매터 값(`summary.sentence`)을 번역 대상으로 내놓게 한다.

**Architecture:** `SourceConfig.front_matter_keys` → `crates/parsers`가 `RmarkdownParser::configured`에 전달 → `yaml_values`가 경로를 들여쓰기 구조로 찾고 `yaml_scalar`가 앵커·따옴표·평문 값을 자른다.

**Tech Stack:** Rust, `serde`, `regex`.

**Spec:** `docs/superpowers/specs/2026-10-30-rmarkdown-front-matter-keys-design.md`

## Global Constraints

- 옵션이 비어 있을 때 기존 `title`·`subtitle`·`description` 세그먼트 모양을 바꾸지 않는다(`projects/loss-data-analytics`의 상태가 그대로여야 한다).
- 테스트를 먼저 실패시키고 구현한다.
- 커밋 메시지: `[core] feat: …` 한국어 요약.

---

### Task 1: 회귀 테스트 (수식 규칙, 이미지 속성)

**Files:** Modify `crates/parser-rmarkdown/src/tests.rs`

- [x] `$FAMOUS_COMPANY on $HYPED_TECHNOLOGY`, `$60m then $800m`이 수식으로 가려지지 않고 산문으로 나오는지.
- [x] `![A caption](x.png){fig-alt="…"}`가 캡션만 내놓고 재구성에 `{…}`가 그대로 남는지.

### Task 2: 설정 옵션

**Files:** Modify `crates/core/src/config.rs`

- [x] `front_matter_keys: Vec<String>` 추가, doc comment(`rmarkdown` only, 점 경로).
- [x] `from_toml`: 다른 파서가 설정하면 거부, 빈 조각 거부. 테스트.

### Task 3: 파서

**Files:** Modify `crates/parser-rmarkdown/src/lib.rs`, `crates/parser-rmarkdown/src/tests.rs`, `crates/parsers/src/lib.rs`

- [x] `RmarkdownParser::configured(patterns, keys)`; `with_patterns`는 유지.
- [x] `yaml_scalar`: 앵커 접두, 따옴표 값, 평문 이어쓰기, `|`/`>`/`*` 제외. 최상위 키 경로도 같은 함수를 쓴다.
- [x] `yaml_values`: 경로를 들여쓰기 구조로 찾아 `BlockType::Paragraph` Literal로 추가, 중복 제거.
- [x] `crates/parsers`에서 옵션 전달, 레지스트리 테스트.
- [x] 테스트: 앵커·여러 줄 큰따옴표·평문·별칭·블록 스칼라, 재구성 시 앵커와 별칭 보존.

### Task 4: 검증

- [x] `cargo test --workspace`, `cargo clippy --workspace --all-targets`.
- [x] `projects/loss-data-analytics`에서 `yeokja status` 세그먼트 수가 상태와 같다.
- [x] `learninpublic/upstream/chapters`에 임시 설정으로 `status`·`coverage`를 돌려 옵션 유무의 세그먼트 수 차이를 확인한다.
