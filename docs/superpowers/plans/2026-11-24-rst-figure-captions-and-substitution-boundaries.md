# reStructuredText 그림 캡션과 치환 참조 경계 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `rst` 파서가 `figure`의 캡션과 범례를 번역 세그먼트로 내고 옵션 들여쓰기와 무관하게 `:alt:`·`:caption:`을 읽으며, `table` 지시문의 제목을 옮기고, RST 형식 수리와 검사가 한국어 단어에 붙은 치환 참조(`|Intro|장에서`)를 `\ `로 떼어 낸다.

**Architecture:** 파서(`parser-rst`의 `directive_header_end`·`directive_option_metadata`, `figure` 분기, `TITLED_PROSE_DIRECTIVES`) → 형식 수리(`translate::evaluator_format`의 `rst_substitutions`·`repair_rst_boundaries`) → 형식 검사(`rst_broken_substitutions`, `line_start_construct`의 RST 줄 블록 판정).

**Tech Stack:** Rust.

**Spec:** `docs/superpowers/specs/2026-11-24-rst-figure-captions-and-substitution-boundaries-design.md`

## Global Constraints

- 기존 `rst` 계열 프로젝트(`cpython-devguide`, `peps`, `pypy`, `mil`)에서 `reconcile --dry-run`의 dropped는 0이고 번역된 세그먼트는 모두 kept다. added는 spec 4절의 표와 같다.
- 테스트를 먼저 실패시키고 구현한다.
- 설정(`crates/core/src/config.rs`)은 바꾸지 않는다.

---

### Task 1: 지시문 머리와 옵션 블록 (`parser-rst`)

**Files:** Modify `crates/parser-rst/src/lib.rs`

- [x] 실패하는 테스트: 4칸 옵션의 `code-block` `:caption:`이 세그먼트다; 빈 줄 뒤 코드 본문의 `:caption:` 줄은 세그먼트가 아니다; 목록 안에 중첩된 `figure`의 8칸 `:alt:`와 그 이어지는 줄이 한 세그먼트다.
- [x] `directive_header_end`(첫 빈 줄까지)와, 머리 안 첫 필드 표시의 열을 옵션 열로 쓰는 `directive_option_metadata`.

### Task 2: `figure`의 캡션과 범례 (`parser-rst`)

**Files:** Modify `crates/parser-rst/src/lib.rs`

- [x] 실패하는 테스트: 7E 꼴(`.. _fig-cloud:` + 4칸 옵션 + 캡션 + 범례)에서 캡션과 범례가 세그먼트이고, 번역 없는 재구성이 원문과 같으며, 번역된 재구성이 지시문 줄과 옵션을 바이트 그대로 두고 캡션을 4칸에 쓴다; 캡션 없이 `..` 주석이 오는 그림과 여러 줄 `:target:`은 원문 그대로다.
- [x] `figure`를 `OPAQUE_DIRECTIVES`에서 빼고, 머리만 불투명 블록으로 낸 뒤 내용은 일반 본문으로 파싱한다. 기존 `image_alt…` 테스트의 그림 본문은 이제 캡션 세그먼트다.

### Task 3: `table` 제목 (`parser-rst`)

**Files:** Modify `crates/parser-rst/src/lib.rs`

- [x] 실패하는 테스트: `.. table:: Common protocol fields.`의 인수가 첫 세그먼트이고, 번역 후 옵션 줄이 그대로다.
- [x] `TITLED_PROSE_DIRECTIVES`에 `table`.

### Task 4: 치환 참조 경계 (`translate::evaluator_format`)

**Files:** Modify `crates/translate/src/evaluator_format.rs`

- [x] 실패하는 테스트: `|Intro|장에서` → `|Intro|\ 장에서`, `참고로|Fed|_도` → `참고로\ |Fed|_\ 도`, `|Fed|.3`은 그대로; 줄 블록 `| 첫 줄`, 원문이 치환으로 쓰지 않은 `a|b|c`, 원문에 없는 이름, 리터럴 안의 막대는 그대로.
- [x] `rst_substitutions`(후보와 인식 여부), `rst_source_substitutions`, `repair_rst_boundaries`의 삽입.

### Task 5: 형식 검사 (`translate::evaluator_format`)

**Files:** Modify `crates/translate/src/evaluator_format.rs`

- [x] 실패하는 테스트: 단어에 붙은 원문 치환은 `FormatLost` 오류이고 메시지에 그 꼴이 있다; 수리한 번역(줄 머리가 `|Intro|\ 장`인 것 포함)은 통과한다; 원문에 없는 줄 블록 `| `로 시작하는 번역은 여전히 거부된다.
- [x] `rst_broken_substitutions`와 오류 메시지, `line_start_construct`의 `Markup::Rst` 줄 블록 판정.

### Task 6: 기존 프로젝트 확인

- [x] 변경 전·후 release 바이너리로 네 프로젝트에서 `reconcile --dry-run`과 `status --check`를 실행하고 spec 4절의 표와 맞춘다. 서브모듈이 없는 작업 트리에서는 메인 체크아웃의 프로젝트에 dry-run만 실행하고, 실행 전·후 state 파일 해시가 같은지 확인한다.
- [x] 코퍼스 테스트(`pep_corpus`, `pypy_corpus`, `mil_corpus`)가 실제 서브모듈로 통과한다.
- [x] 배포 전에 `peps`에서 `reconcile`과 새 세그먼트 45개 번역을 마친다.
