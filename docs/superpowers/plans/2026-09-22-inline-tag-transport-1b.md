# 인라인 태그 전송 1b단계 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 1단계 태그 전송을 `myst`·`mdx` 파서 소스로 넓히고 nix-dev·zero-to-nix·putting-the-you-in-cpu에서 켠다.

**Architecture:** `inline::markdown`에 방언(`Dialect`)을 더해 가림·이스케이프·응답 판독의 방언별 차이만 가른다.
Markdown이 아닌 필드 문자열은 모델의 `BlockRole::Literal`로 표시해 태그 없이 글자 그대로 주고받는다.
파이프라인·프롬프트·평가기는 그대로다.

**Tech Stack:** Rust 2024, pulldown-cmark 0.12. 검증 보조: nix-dev venv의 markdown-it-py·myst-parser, 스크래치의
`@mdx-js/mdx`·remark-gfm(node).

**Spec:** `docs/superpowers/specs/2026-09-22-inline-tag-transport-1b-design.md`.

## Global Constraints

- CommonMark(`markdown` 파서) 경로의 동작은 바이트 하나 바뀌지 않는다(1단계 테스트와 말뭉치 왕복 전부 통과).
- 꺼진 소스의 동작은 바뀌지 않는다.
- 커밋은 Secretive(Touch ID) SSH 서명. 우회 금지.
- A/B 동안 추적되는 state를 건드리지 않는다(스크래치 사본에서만).

---

### Task 1: 글자 그대로의 필드 표시

**Files:** `crates/core/src/model.rs`, `crates/parser-markdown-dialect/src/lib.rs`, `crates/parser-myst/src/lib.rs`,
`crates/parser-mdx/src/lib.rs`, `crates/parser-mkdocs/src/lib.rs`, `crates/parser-markdeep/src/lib.rs`

- [ ] `BlockRole::Literal` 추가(문서 주석: 컨테이너가 인용할 뿐 마크업으로 읽지 않는 문자열).
- [ ] `Extra { range, block_type, literal: bool }`, `push_extra`가 `literal`이면 `role = BlockRole::Literal`.
  mkdocs·markdeep·방언 공통부의 기존 생성 지점은 `literal: false`.
- [ ] MDX: `Field`를 모두 `literal: true`로. MyST: `Field`에 `literal`을 두고 toctree의 항목 제목과 `:caption:`만
  `true`(코드 블록 `:caption:`·지시문 제목은 `false`).
- [ ] 테스트: MDX front matter 제목·JSX `title` 블록의 역할이 `Literal`, MyST toctree 제목은 `Literal`이고 캡션·지시문
  제목은 아님. 기존 파서 테스트 통과.

### Task 2: 설정 허용 범위

**Files:** `crates/core/src/config.rs`, `crates/translate/src/orchestrator.rs` (`inline_tags_for`)

- [ ] 테스트: `myst`·`mdx`에 켜면 통과, `rst`에 켜면 `Invalid`(메시지에 허용 파서 셋).
- [ ] 구현: 허용 파서 목록 `INLINE_TAG_PARSERS = ["markdown", "myst", "mdx"]`를 core에 두고 config 검증과
  `inline_tags_for`가 함께 쓴다.

### Task 3: 방언과 가림 (`tagify`)

**Files:** `crates/translate/src/inline/markdown.rs`

**Interfaces:**
```rust
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)] pub enum Dialect { #[default] CommonMark, Myst, Mdx }
impl DocContext { pub fn new(source: &str, dialect: Dialect) -> Self; pub fn from_markdown(source) -> Self /* CommonMark */ }
pub enum Position { Inline, TableCell, Plain }
pub enum TagKind { …, Role }   // letter 'a'
```
- 가림(`mask`)은 방언을 받는다. MyST: `{name}` 바로 뒤에 코드 스팬이 시작하면 둘을 한 구문으로 가린다.
  MDX: 이스케이프되지 않은 `{`에서 괄호 깊이가 맞는 `}`까지를 가리고, 짝이 없으면 `Untaggable`. 수식 가림은 CommonMark만.
- 역할 분류 함수 `role_parts(name, content) -> RoleShape { Label { label, target, implicit }, Literal }`를
  spec 2.2대로. 태그화: 레이블 역할은 `<aN>레이블</aN>`(`TagKind::Role`), 글자 그대로 역할은 원문 그대로 보이는
  `Code` 태그(`code`=내용, `open`=역할 원문, 새 필드 `role: Option<String>`=이름). 표현식은 `<xN/>`(shape `e`).
- MyST는 `ENABLE_STRIKETHROUGH`를 끈다(`options(dialect)`).
- `Position::Plain`: 텍스트만 이스케이프, 태그 없음.

- [ ] 테스트: 레이블 역할·암시 `term`·글자 그대로 역할·도메인 역할의 태그 텍스트, 코드 스팬 안의 `{x}`는 표현식이
  아님, `\{`는 글자, 짝 없는 `{`는 `Untaggable`, `<Language />`는 `<x1/>`, `<cite>…</cite>`는 `<h1>…</h1>`,
  MyST `~~a~~`는 글자, CommonMark에서 `{ref}`+코드는 1단계 그대로, Plain은 태그 없음.
- [ ] 실패 확인 → 구현 → 통과.

### Task 4: 판독 (`read`)

- [ ] MyST: 응답에서 코드 스팬 바로 앞 글자 끝의 `{name}`을 떼어 그 코드의 역할 이름으로 붙인다.
- [ ] 코드 맞추기: 정확 일치는 (내용, 역할 이름)이 같은 것 → 내용만 같은 것 순. 덜어 내기(2)는 역할 태그에 쓰지 않는다.
- [ ] 레이블 역할(`Role`)은 링크처럼 정확히 한 번, 안에 다른 노드(태그·코드)가 오면 문제
  ("<a3> holds plain text only — move … outside it").
- [ ] Plain: 태그 토큰은 버리고 백틱은 글자로, 결과는 텍스트 노드 하나.
- [ ] 테스트: `{ref}`를 빠뜨린 응답과 남긴 응답이 같은 트리, 같은 내용의 코드와 역할이 제자리로 돌아감, 레이블 역할
  안의 `<b2>`는 문제, Plain 응답의 `<i1>`은 버려짐.

### Task 5: 직렬화와 되읽기 (`render`)

- [ ] 레이블 역할: Open에서 Close까지의 글자를 날것으로 모아 `` {name}`레이블 <대상>` ``(암시 `term`은 같으면
  `` {term}`용어` ``). 레이블 속 백틱은 더 긴 백틱 줄.
- [ ] MDX 이스케이프: 모든 `{`·`<`. 취소선 방언에서 공백 사이가 아닌 `~`(GFM 한 글자 취소선), MyST는 `~` 그대로.
- [ ] 되읽기(`read_back`)가 방언의 가림을 쓰고, 가린 역할은 레이블 역할이면 `(a`+레이블+`)`, 글자 그대로면
  `c:내용|`, 표현식은 `e`로 읽는다(`intended`와 같은 모양).
- [ ] Plain: 이스케이프 없이 글자를 잇고 `verified = true`.
- [ ] 테스트: `{term}` 번역·유지, `{ref}` 명시 레이블 번역, `**{term}`x`**를`이 조사 넣기로 닫힘, MDX `a<b`·
  `{x}` 글자가 이스케이프되어 되읽기 통과, Plain `2 * 3`이 그대로.

### Task 6: orchestrator

- [ ] `InlineFile::new(doc, dialect)`: 파서 이름으로 방언, `BlockRole::Literal` 블록의 세그먼트는 `Position::Plain`.
- [ ] 테스트(모의 provider): MyST 소스가 태그 텍스트로 가고 역할이 되살아나 저장됨, MDX front matter 제목이
  Plain으로 가서 요청이 기존 경로로 떨어지지 않음.

### Task 7: 말뭉치 왕복

- [ ] `tests/inline_roundtrip.rs`를 `markdown`·`myst`·`mdx` 소스로 넓힌다(방언별 DocContext·Position, 수치를 방언별로).
- [ ] 스크래치 스크립트로 방언 렌더러 확인: 왕복 결과를 MyST는 markdown-it-py 토큰, MDX는 `@mdx-js/mdx` 컴파일로
  원문과 비교.

### Task 8: A/B와 사람 판독

- [ ] 스크래치 사본(nix-dev, zero-to-nix, putting-the-you-in-cpu; 설정·용어집 복사, `upstream` 링크, `state/` 사본에서
  표본 파일의 번역을 비움). 태그 모드 2회, 지금 방식 1회. `RUST_LOG=yeokja_translate=debug`.
- [ ] 방언 렌더러 채점(spec 3절), 재시도율, 기존 경로 비율, 조사 일치.
- [ ] 태그 응답 100개 이상 사람 판독.

### Task 9: 켜기와 마무리

- [ ] 합격하면 세 프로젝트 `yeokja.toml`의 `myst`·`mdx` 소스에 `inline_tags = true`(주석으로 이유).
- [ ] `cargo test --workspace`, `cargo clippy -p yeokja-translate -p yeokja-core`(새 경고 없음).
- [ ] spec에 결과를 적고, `docs/translation-project-checklist.md`에 켤 수 있는 파서와 조건(수식 확장 없음)을 적는다.
- [ ] 푸시 전 `.github/pages-projects.json`의 모든 프로젝트로 `rebuild-translations.sh`를 돌려 낡은 세그먼트가 없는지 본다.
- [ ] 커밋: 설계·계획 → 기능(Task 1~7) → A/B 결과와 설정(Task 8~9).
