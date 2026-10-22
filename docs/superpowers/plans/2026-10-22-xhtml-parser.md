# XHTML 파서 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** EPUB 콘텐츠 문서(XHTML)를 파일 단위로 읽고 인라인 태그로 번역을 주고받는 `xhtml` 파서를 추가한다.

**Architecture:** 새 크레이트 `crates/parser-xhtml`이 바이트 범위를 보존하는 XML 판독기(`xml.rs`)와 번역 단위 추출·문장 분할·범위 재구성(`lib.rs`)을 가진다. 번역 쪽은 `inline/xhtml.rs`가 태그화·직렬화를 맡고, 판독은 `markdown::read`를 `Dialect::Xhtml`로 재사용한다.

**Tech Stack:** Rust(외부 XML 크레이트 없음), yeokja 인라인 태그 전송.

**Spec:** `docs/superpowers/specs/2026-10-22-xhtml-parser-design.md`

## Global Constraints

- 다른 파서와 기존 프로젝트의 세그먼트 모양을 바꾸지 않는다. Markdown 계열 코덱은 `Dialect::Xhtml` 분기 외에 손대지 않는다.
- 엔티티 표는 Python `html.entities.name2codepoint`(HTML 4, 252개)에서 생성한다.
- 시험 말뭉치는 테스트에 넣지 않는다. `YEOKJA_XHTML_CORPUS`로 경로를 받는 테스트만 둔다. 단위 테스트의 예문은 직접 지은 짧은 문장이다.
- 커밋 메시지: `[parser-xhtml] <type>: <한국어 요약>` 또는 `[translate] …`.

---

### Task 1: XML 판독기와 엔티티 표

**Files:** Create `crates/parser-xhtml/src/{xml.rs,entities.rs}`, `crates/parser-xhtml/Cargo.toml`; Modify `Cargo.toml`(workspace member)

- [x] 시작·끝 태그, 속성 값, 텍스트, 주석·PI·DOCTYPE·CDATA의 바이트 범위를 남기는 판독기. 짝 안 맞는 태그, 따옴표 없는 속성, 맨 `&`·`<`, 모르는 엔티티는 위치와 함께 오류.
- [x] `decode_each`(문자와 원문 바이트 범위), `decode`, `entity`.
- [x] 테스트: 범위·중첩, DTD 엔티티, 잘못된 중첩, 선언부.

### Task 2: 번역 단위와 재구성

**Files:** Create `crates/parser-xhtml/src/{lib.rs,tests.rs}`, `crates/parser-xhtml/tests/corpus.rs`; Modify `crates/core/src/parser.rs`(`Markup::Xhtml`)

- [x] 블록 요소 안의 인라인 구간을 번역 단위로, 블록 자식에서 끊기. `head`의 `title`, `alt`·`title` 속성(Literal 블록), 표 셀 위치, 건너뛸 하위 트리(`pre`, `script`, `math`, `translate="no"` 등).
- [x] 보이는 텍스트로 문장 분할한 뒤 열린 인라인 요소 안의 분할점을 버리기.
- [x] 재구성: 구간 갈아 끼우기, 번역된 속성은 시작 태그째 바꾸고 번역된 구간 안의 같은 태그 바이트도 바꾸기.
- [x] `skeleton()`.
- [x] 검증:

```sh
cargo test -p yeokja-parser-xhtml
YEOKJA_XHTML_CORPUS=<EPUB을 푼 OEBPS> cargo test -p yeokja-parser-xhtml --test corpus -- --nocapture
# 30 documents, 4073 blocks, 8858 segments, 바이트 단위 왕복
```

### Task 3: 레지스트리와 설정

**Files:** Modify `crates/parsers/{Cargo.toml,src/lib.rs}`, `crates/core/src/config.rs`, `crates/translate/src/orchestrator.rs`

- [x] 이름 `xhtml`, 확장자 `.xhtml` 자동 감지.
- [x] `INLINE_TAG_PARSERS`에 `xhtml`, `SourceConfig::uses_inline_tags`(xhtml은 언제나 참), 설정 검사와 `inline_tags_for`가 이를 쓰기. `audit_file`은 xhtml을 건너뛰기.
- [x] 테스트: `xhtml_sources_always_use_inline_tags`.

### Task 4: 인라인 코덱

**Files:** Create `crates/translate/src/inline/xhtml.rs`; Modify `crates/translate/src/inline/{mod.rs,markdown.rs}`, `crates/translate/Cargo.toml`, `crates/translate/src/{prompt.rs,evaluator_format.rs,evaluator_link.rs}`

- [x] `Dialect::Xhtml`, `Tag::verbatim`, `tagify`·`render`·`prose_of` 분기.
- [x] 태그화: `em`류 `<iN>`, `strong`류 `<bN>`, `a` `<aN>`, 그 밖의 짝 요소 `<hN>`, 빈 요소·각주 참조·글자 없는 요소 `<xN/>`, 코드는 백틱. 산문의 백틱은 `Untaggable`.
- [x] 직렬화: 원문 태그 복원, `&`·`<`·`>`·U+00A0 이스케이프, 속성 값은 따옴표까지. 모델이 글자로 쓴 엔티티(`&nbsp;`)는 문자로 읽기.
- [x] `closing_rule`·`backtick_spans`·`constrained_marks`의 `Xhtml` 분기, 링크 평가기는 XHTML 엔티티를 읽은 뒤 URL 추출.
- [x] 검증:

```sh
cargo test --workspace
```
