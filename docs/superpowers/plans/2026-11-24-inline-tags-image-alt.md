# 인라인 태그의 이미지 대체 텍스트 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `inline_tags = true`에서 이미지의 alt가 짝 태그 `<gN>…</gN>`로 모델에 가고, 번역된 alt가 목적지·제목·참조를 원문 바이트 그대로 둔 채 돌아온다.

**Architecture:** 인라인 코덱(`translate::inline::markdown`의 `TagKind`·`tagify`·`read`·`Renderer`) → 감사(`inline::audit`의 `NOT_TO_GAIN`·`same_construct`) → 프롬프트(`prompt::INLINE_TAG_RULES`에 `<gN>` 한 구절).

**Tech Stack:** Rust, pulldown-cmark 0.12(변경 없음).

**Spec:** `docs/superpowers/specs/2026-11-24-inline-tags-image-alt-design.md`

## Global Constraints

- 변경은 `crates/translate/src/inline/`과 `crates/translate/src/prompt.rs`(규칙 문장 하나)에 한정한다.
- 세그먼트와 state 상태를 바꾸지 않는다.
- 테스트를 먼저 실패시키고 구현한다.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets` 경고 없음.

---

### Task 1: 태그화 (`inline/markdown.rs`)

- [x] 실패하는 테스트: `![fig one](a.png)` → `<g1>fig one</g1>`; `![](a.png)` → `<x1/>`; `![**x** y](i.png)` → `<g1><b2>x</b2> y</g1>`; `[![alt](i.png)](u)` → `<a1><g2>alt</g2></a1>`; `![alt][ref]`·`![ref]` → `<g1>…</g1>`.
- [x] `TagKind::Image`(글자 `g`, 짝). `Start(Image)` 다음이 곧 `End(Image)`면 빈 태그(모양 `(g)`), 아니면 링크처럼 열고 `End(Image)`에서 꼬리와 단축 레이블을 채운다.

### Task 2: 읽기와 직렬화 (`inline/markdown.rs`)

- [x] 실패하는 테스트: 번역 응답이 `![번역](src "title")`로 돌아온다; 단축 참조가 바뀌면 `![번역][ref]`; 링크 안 이미지; alt의 대괄호가 이스케이프되고 다섯 방언에서 다시 읽은 이미지 수가 1; 빈 `<g1></g1>`은 `read`의 문제; 원문을 그대로 돌리면 바이트가 같다.
- [x] `read`: 이미지는 정확히 한 번; 원문 alt가 있는데 비면 문제.
- [x] `Renderer`: 여는 자리 `![`, 닫는 자리는 링크의 꼬리·레이블 규칙, `first_char`는 `!`·`]`, `links_open`에 포함.
- [x] `read_back`: 이미지는 `(g`…`)`이고 alt는 글자. `construct_name("(g") = "image"`.

### Task 3: 감사와 번역할 글자 판정 (`inline/audit.rs`)

- [x] 실패하는 테스트: 영어 alt를 둔 기존 번역과 alt를 옮긴 번역은 Sound이고 `gained`가 비어 있다; 글자 대괄호로 이미지를 하나 더 만든 번역은 "image 2 where the source has 1"; 이미지 단독 세그먼트는 `nothing_to_translate`가 거짓, `![](x.png)`는 참.
- [x] `NOT_TO_GAIN`의 `g` → `(g`; `same_construct`에 이미지 짝짓기; `describe`.

### Task 4: 프롬프트 (`prompt.rs`)

- [x] `INLINE_TAG_RULES`에 `<gN>…</gN> the description (alt text) of an image`.

### Task 5: 검증

- [x] `cargo test --workspace`, `cargo clippy --workspace --all-targets`.
- [x] 말뭉치 왕복: `YEOKJA_PROJECTS=<projects> cargo test -p yeokja-translate --test inline_roundtrip -- --ignored`에서 실패가 늘지 않는다.
