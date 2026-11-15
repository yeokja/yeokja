# MkDocs 정의 목록과 수식 옵션 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `mkdocs` 소스가 `definition_lists = true`로 Python-Markdown 정의 목록을 용어·정의별 세그먼트로 내놓고, `math = false`로 `$`를 글자로 읽으며, 그런 소스에 `inline_tags`를 켤 수 있게 한다.

**Architecture:** `SourceConfig.math`/`definition_lists` → `crates/parsers`가 `MkdocsParser::configured`에 전달 → `scan`이 수식 범위를 건너뛰고 `scan_definition_lists`가 그림자의 `:`를 목록 표시로, 용어 줄을 `Extra`로 바꾼다. 수식을 끈 파서는 `Markup::MkDocsNoMath`를 알리고, 번역 쪽 평가·프롬프트·인라인 코덱(`Dialect::MkDocs`)이 그것을 따른다.

**Tech Stack:** Rust, `serde`, pulldown-cmark(`parser-markdown-dialect`, 변경 없음).

**Spec:** `docs/superpowers/specs/2026-11-15-mkdocs-def-list-and-math-option-design.md`

## Global Constraints

- 옵션을 쓰지 않는 소스의 세그먼트를 바꾸지 않는다(모든 프로젝트에서 `status --check` 결과가 같아야 한다). `parser-markdown-dialect`의 파싱 코드는 건드리지 않는다.
- 테스트를 먼저 실패시키고 구현한다.
- 커밋 메시지: `[core] feat: …` 한국어 요약.

---

### Task 1: 파서 옵션과 정의 목록 (`parser-mkdocs`)

**Files:** Modify `crates/parser-mkdocs/src/lib.rs`, `crates/parser-mkdocs/examples/roundtrip.rs`

- [x] `MkdocsParser`를 `math`·`definition_lists` 필드를 가진 구조체로 바꾸고 `Default`(수식 켬, 정의 목록 끔)와 `configured(math, definition_lists)`를 둔다.
- [x] 실패하는 테스트: 용어와 정의가 따로 나오고 `:   `와 줄이 남는다, 한 용어의 여러 정의, 빈 줄 없이 이어지는 다음 용어, 여러 줄 용어, 들여쓴 이어지는 줄·문단·펜스, 빈 줄 뒤 정의, admonition 안, 콜론 뒤 공백 1칸·5칸 이상, 정의가 아닌 콜론 줄(목록·인용·펜스·`:::`), 옵션이 꺼져 있으면 지금과 같다.
- [x] `scan`이 줄 종류(`LineKind`)를 기록하고, 수식을 채운 뒤 `scan_definition_lists`를 부른다.
- [x] `roundtrip` 예제가 `--no-math`, `--definition-lists`를 받는다.

### Task 2: 수식 끄기

**Files:** Modify `crates/parser-mkdocs/src/lib.rs`, `crates/core/src/parser.rs`, `crates/translate/src/{evaluator_format,evaluator_link,evaluator_glossary,prompt}.rs`

- [x] 실패하는 테스트: `…10000$](…)`로 끝나는 이웃한 두 목록 항목이 두 세그먼트다, `$$`뿐인 문단도 산문이다, `markup()`이 `MkDocsNoMath`다.
- [x] `math`가 꺼지면 `math_spans_from`을 부르지 않는다.
- [x] `Markup::MkDocsNoMath`: Format은 수식 검사와 가리기를 건너뛰고 Jinja·느슨한 강조는 유지, 링크·용어집·코드 스팬 규칙은 `MkDocs`와 같게, 프롬프트 닫는 규칙은 `$`를 글자로 말한다. 평가·프롬프트 테스트.

### Task 3: 설정과 파서 선택

**Files:** Modify `crates/core/src/config.rs`, `crates/parsers/src/lib.rs`

- [x] `math: Option<bool>`, `definition_lists: Option<bool>`와 접근자 `mkdocs_math()`, `mkdocs_definition_lists()`.
- [x] `from_toml`: 다른 파서가 적으면 거부. `SourceConfig`에 `deny_unknown_fields`. 테스트(기본값, 다른 파서, 오타).
- [x] `select_parser`가 `mkdocs` 소스에 옵션을 넘긴다. 레지스트리 테스트.

### Task 4: 인라인 태그

**Files:** Modify `crates/core/src/config.rs`, `crates/translate/src/inline/markdown.rs`, `crates/translate/src/orchestrator.rs`, `crates/translate/tests/inline_roundtrip.rs`

- [x] 실패하는 테스트: `mkdocs` + `inline_tags`는 `math = false`일 때만 통과한다, `Dialect::MkDocs`는 `~~`를 글자로 읽고 `~`·`<`·`&` 앞에 역슬래시를 쓰지 않되 원문이 쓴 것은 되돌린다.
- [x] `SourceConfig::supports_inline_tags`; 설정 검증, `inline_tags_for`, `audit_file`, 코퍼스 왕복 테스트가 그것으로 묻는다.
- [x] `Dialect::MkDocs`: 취소선·MathJax 가리기 없음, 이스케이프 규칙.

### Task 5: 문서와 검증

**Files:** Modify `docs/translation-project-checklist.md`

- [x] 체크리스트에 `mkdocs` 소스 옵션과 인라인 태그 조건을 적는다.
- [x] `cargo test --workspace`, `cargo clippy --workspace --all-targets`.
- [x] 모든 프로젝트에서 변경 전·후 바이너리의 `status --check` 출력과 종료 코드가 같다.
- [x] Linux Kernel Internals 원문에 임시 설정으로 `status`를 돌려 옵션 유무의 세그먼트 수를 비교하고, `roundtrip` 예제의 가짜 번역 출력 차이가 정의 목록 줄과 `$` 목록 항목뿐인지, 인라인 태그 코퍼스 왕복이 같은 HTML을 내는지 확인한다.
