# XML 어휘 프로필 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `xhtml` 파서를 XML 어휘 프로필로 일반화해 `cnxml`(OpenStax)과 `pretext`(PreTeXt, MathBook XML) 파서를 더한다. `xhtml`의 세그먼트와 출력은 바뀌지 않는다.

**Architecture:** `crates/parser-xhtml/src/vocab.rs`의 `Vocabulary`가 요소 분류 질문에 답하고, 파서(`lib.rs`)와 인라인 태그 코덱(`crates/translate/src/inline/xhtml.rs`)이 같은 답을 쓴다. 파서 이름이 어휘를 고른다.

**Tech Stack:** Rust, 기존 XML 판독기(`xml.rs`), 인라인 태그 전송.

**Spec:** `docs/superpowers/specs/2026-10-25-xml-vocabulary-profiles-design.md`

## Global Constraints

- technicalpm 원문의 블록 4,073개·세그먼트 8,858개(번역 대상 8,637개)와 무변환 왕복, `status --check`, 저장된 번역으로 다시 쓴 출력이 그대로여야 한다.
- 픽스처는 두 사본의 짧은 발췌이고 머리 주석에 출처·라이선스를 적는다. 전체 사본은 `YEOKJA_XHTML_CORPUS`로 경로를 받는 테스트로만 돈다.
- 커밋 메시지: `[parser-xhtml] <type>: <한국어 요약>`, `[translate] …`, `[core] …`.

---

### Task 1: 어휘 프로필과 블록 순회

**Files:** Create `crates/parser-xhtml/src/vocab.rs`; Modify `crates/parser-xhtml/src/lib.rs`

- [x] `Vocabulary { Xhtml, Cnxml, Pretext }`: 블록(이름, 블록을 품은 요소), 건너뛰기, 코드, 번역 속성, 인라인 종류, 제목 컨테이너, 섬, 표 구조, 절 요소, 블록 종류, 문장 끝 수식, 강조. XHTML 표는 기존 상수 그대로.
- [x] 블록 순회가 `breaks_run`으로 내려가고, 블록을 품은 인라인 요소는 투명 컨테이너로 걷는다. 제목 레벨은 절 깊이, 표 머리 구역의 행은 머리 행.
- [x] `Item::Attr`를 `Item::Inner { host, value }`로 일반화하고 `math` 안 `mtext`를 섬으로 모은다(앞뒤 공백 제외).
- [x] 재구성: 세그먼트 사이 원문 공백 유지, 호스트 갱신은 구간 번역 안 또는 직접.
- [x] `XhtmlParser`·`CnxmlParser`·`PretextParser`, `skeleton_with`.
- [x] 검증:

```sh
cargo test -p yeokja-parser-xhtml
YEOKJA_XHTML_CORPUS=projects/technicalpm/upstream/OEBPS cargo test -p yeokja-parser-xhtml --test corpus -- --nocapture
# 30 documents, 4073 blocks, 8858 segments
```

### Task 2: 인라인 코덱이 어휘를 공유

**Files:** Modify `crates/translate/src/inline/{xhtml.rs,markdown.rs}`, `crates/translate/src/orchestrator.rs`

- [x] `Dialect::{Cnxml, Pretext}`, `Dialect::for_vocabulary`, `Dialect::vocabulary`. `DocContext::new`·`tagify`·`render`·`prose_of`가 어휘가 있는 방언을 `inline::xhtml`로 넘긴다.
- [x] `inline::xhtml`의 코드·불투명·인라인 종류 판단을 `Vocabulary`로. XHTML 각주 참조 규칙은 `Vocabulary::is_opaque_inline`.
- [x] `audit_file`은 어휘가 있는 방언을 건너뛴다.
- [x] 테스트: CNXML 의미 태그 매핑, 계산기 키·`math`, PreTeXt `m`·`me`·`xref`·`term`.

### Task 3: 등록과 설정

**Files:** Modify `crates/core/src/config.rs`, `crates/parsers/src/lib.rs`

- [x] `XML_PARSERS`, `INLINE_TAG_PARSERS`에 `cnxml`·`pretext`, `uses_inline_tags`.
- [x] `parser_by_name`에 `cnxml`·`pretext`, 확장자 `.cnxml`.
- [x] 테스트: `xml_sources_always_use_inline_tags`, `xml_vocabulary_parsers_by_name_and_cnxml_by_extension`.

### Task 4: 픽스처와 말뭉치 측정

**Files:** Create `crates/parser-xhtml/tests/{vocabularies.rs,fixtures/cnxml/*,fixtures/pretext/*}`; Modify `crates/parser-xhtml/tests/corpus.rs`, `crates/parser-xhtml/src/tests.rs`

- [x] CNXML 발췌 `m00002`·`m00011`·`m00026`·`m00050`·컬렉션, PreTeXt 발췌 `ila.xml`·`vectors.xml`·`cplx-eigenvals.xml`.
- [x] 무변환 왕복, 세그먼트 수, 한국어 대역 번역의 골격 동치, `mtext` 번역, `p` 안 디스플레이 자리표시, `type-name` 번역, 표 머리 행, 제목 레벨.
- [x] `corpus.rs`가 `YEOKJA_XHTML_CORPUS_PARSER`로 어휘를 고르고, 하위 디렉터리까지 읽고, 길이 분위수와 블록 종류별 세그먼트 수를 낸다.
- [x] 검증:

```sh
YEOKJA_XHTML_CORPUS=<osbooks-principles-finance>/modules YEOKJA_XHTML_CORPUS_PARSER=cnxml \
  cargo test -p yeokja-parser-xhtml --test corpus -- --nocapture
# 132 documents, 8741 blocks, 15212 segments
YEOKJA_XHTML_CORPUS=<ila>/src YEOKJA_XHTML_CORPUS_PARSER=pretext \
  cargo test -p yeokja-parser-xhtml --test corpus -- --nocapture
# 40 documents, 3192 blocks, 5865 segments
```

### Task 5: XHTML 불변 확인과 문서

**Files:** Modify `docs/superpowers/specs/2026-10-22-xhtml-parser-design.md`, `docs/translation-project-checklist.md`

- [x] technicalpm: `yeokja status upstream/OEBPS` 28개·8,637개, `status --check` 통과, `yeokja translate`가 다시 쓴 `ko/OEBPS`가 변경 전 바이너리의 출력과 바이트 동일.
- [x] 임시 프로젝트로 두 사본의 `yeokja status`·`coverage`를 재고 spec 4장에 적는다.
- [x] `inline_roundtrip`(무시 표시)을 두 사본에 돌려 태그화할 수 없는 세그먼트 0개 확인.
- [x] XHTML spec에서 어휘 프로필 spec을 가리키고, 체크리스트의 인라인 태그 절에 세 파서를 적는다.
- [x] 검증:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets
cargo build --release -p yeokja-cli
```
