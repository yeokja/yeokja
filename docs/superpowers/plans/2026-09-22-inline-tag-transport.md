# 인라인 태그 전송 1단계 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** mdBook `markdown` 소스에서 인라인 마크업을 번호 붙은 태그로 모델과 주고받고, CommonMark
문법은 직렬화기가 맡도록 파이프라인을 바꾼다(소스별 `inline_tags = true`로 켠다).

**Architecture:** `yeokja-translate`에 `inline` 모듈(태그화·판독·직렬화·검증)을 두고, orchestrator가
소스 설정과 문서 문맥으로 요청을 태그화한다. 파이프라인은 태그 응답을 판독·검증해 직렬화한 뒤
정렬 검사와 평가기를 돌린다. state 형식과 재구성은 바뀌지 않는다.

**Tech Stack:** Rust 2024, pulldown-cmark 0.12, tokio 테스트.

**Spec:** `docs/superpowers/specs/2026-09-22-inline-tag-transport-design.md` (특히 3절). 참고 구현은
스파이크의 `tagspike/src/main.rs`(세션 스크래치패드)이며, 이 계획의 알고리즘 설명이 그것을 대신한다.

## Global Constraints

- 1단계는 `parser = "markdown"` 소스에만 허용한다. 다른 파서에 켜면 `ConfigError::Invalid`.
- 켠 소스의 프로젝트가 `[provider].prompt_template`을 쓰면 템플릿에 `{inline_tags}`가 있어야 한다.
- 꺼진 소스의 동작은 바이트 하나 바뀌지 않는다(기존 테스트 전부 통과).
- 보이지 않는 문자(`&#8203;`)를 출력하지 않는다.
- 커밋은 Secretive(Touch ID) SSH 서명. 우회 금지.
- 저장소의 추적되는 state는 A/B 동안 건드리지 않는다(스크래치 사본에서만).

---

### Task 1: 설정 — 소스별 `inline_tags`와 소스 찾기

**Files:**
- Modify: `crates/core/src/config.rs` (`SourceConfig.inline_tags`, `ProjectConfig::source_for`, `from_toml` 검증, 테스트)
- Modify: `crates/parsers/src/lib.rs` (`select_parser`가 `source_for`를 쓰도록)

**Interfaces:**
- Produces: `SourceConfig { …, #[serde(default)] pub inline_tags: bool }`,
  `impl ProjectConfig { pub fn source_for(&self, file: &Path) -> Option<&SourceConfig> }`
  (select_parser와 같은 규칙: `./` 제거, `path` 접두어, `pattern`을 `require_literal_separator`로 대조).

- [ ] 테스트: `inline_tags` 기본값 false, `true` 읽기, markdown이 아닌 파서에 켜면 `Invalid`,
  `prompt_template`에 `{inline_tags}` 없이 켜면 `Invalid`, `source_for`가 패턴·접두어로 고르고 없으면 `None`.
- [ ] 실패 확인 → 구현 → `cargo test -p yeokja-core -p yeokja-parsers` 통과.
- [ ] `select_parser`를 `source_for`로 바꾸고 기존 parsers 테스트 통과 확인.

### Task 2: 태그화 (`inline::markdown::tagify`)

**Files:**
- Create: `crates/translate/src/inline/mod.rs`, `crates/translate/src/inline/markdown.rs`
- Modify: `crates/translate/src/lib.rs` (`pub mod inline;`)

**Interfaces:**
- Produces:
  ```rust
  pub struct DocContext { pub reference_labels: HashSet<String> } // 정규화된 레이블
  impl DocContext { pub fn from_markdown(source: &str) -> Self }  // pulldown-cmark로 정의 수집
  #[derive(Clone, Copy, PartialEq, Eq, Debug)] pub enum Position { Inline, TableCell }
  #[derive(Clone, Debug)] pub enum TagKind { Italic, Bold, Strike, Link, Html, Code, Opaque, Math }
  #[derive(Clone, Debug)] pub struct Tag { pub n: usize, pub kind: TagKind,
      pub open: String, pub close: String,        // 원문 표시·꼬리·HTML 태그, void는 open에 원문 바이트
      pub label: Option<String>, pub shortcut: bool, pub code: Option<String> } // code: 스팬 내용
  #[derive(Clone, Debug)] pub struct Tagged { pub source: String, pub text: String, pub tags: Vec<Tag>, pub position: Position }
  #[derive(Debug)] pub struct Untaggable(pub String);
  pub fn tagify(source: &str, ctx: &DocContext, position: Position, counter: &mut usize) -> Result<Tagged, Untaggable>;
  ```
- 알고리즘(스파이크와 같다): 수식 `\\(…\\)`·`\\[…\\]`와 각주 참조 `[^x]`를 사용 영역 문자로 가림 →
  앞에 `GUARD + ' '`를 붙여 인라인으로 읽음 → pulldown-cmark(STRIKETHROUGH, FOOTNOTES,
  정의된 레이블만 받는 broken-link 콜백) 오프셋 순회. 강조·굵게·취소선은 원문 표시로 짝 태그,
  링크는 `[`와 꼬리(`]…`)를 기록한 짝 태그(단축·축약이면 레이블 기록), 자동 링크·이미지·
  각주·줄바꿈·짝 없는 인라인 HTML은 `<xN/>`, 수식은 `<mN/>`, 코드는 **원문 백틱 스팬 그대로**
  (`code`에 내용 기록, `Code` 태그는 텍스트에 번호 없이 존재), 텍스트는 `&<>` 이스케이프.
  같은 이름의 여는·닫는 인라인 HTML이 세그먼트 안에서 짝을 이루면 `<hN>…</hN>`.
- 태그화할 수 없음: 원문 자체에 여닫을 수 있는 자리의 `*`·`_` 연속이 파서에 글자로 남거나
  (`commonmark_emphasis(source).stray`가 비어 있지 않음), 닫히지 않은 백틱이 있으면 `Untaggable`.

- [ ] 테스트: 강조·굵게·링크(인라인/전체/단축)·코드·자동 링크·이미지·각주·수식·짝 맞는 HTML의
  태그 텍스트, 목록 표시처럼 보이는 세그먼트(`1.`, `- same -`), 정의 없는 `[x]`는 글자,
  태그화할 수 없는 세그먼트(`_Applied to …` 한쪽만 연 `_`), 요청 안 번호 유일성.
- [ ] 실패 확인 → 구현 → 통과.

### Task 3: 판독·검증 (`inline::markdown::read`)

**Files:**
- Modify: `crates/translate/src/inline/markdown.rs`
- Modify: `crates/translate/src/evaluator_format.rs` (`stands_for`, `shed_edges`, `written_as_text`를 `pub(crate)`로)

**Interfaces:**
- Produces:
  ```rust
  pub enum Node { Text(String), Open(usize), Close(usize), Void(usize), Code { n: Option<usize>, written: String } }
  pub struct Tree { pub nodes: Vec<Node>, pub notes: Vec<String> }
  pub fn read(reply: &str, tagged: &Tagged) -> Result<Tree, Vec<String>>; // Err = 재시도 피드백 문장들
  ```
- 순서: (1) 백틱 스팬을 먼저 떼어 `Code` 노드로(태그로 읽지 않음), (2) 나머지에서 `<kN>`, `</kN>`,
  `<kN/>`, 번호 없는 `</k>`를 읽고 `&lt; &gt; &amp;`만 되돌림, (3) 느슨한 정규화: 번호가 정체(종류
  글자 교정), 번호 없는 닫기는 가장 안쪽 같은 종류, 표에 없는 태그는 버림, (4) 코드 맞추기(spec 3.2의
  다섯 경우; 2는 `stands_for`/`shed_edges`, 4는 `written_as_text(&tagged.source, content)`), (5) 검증:
  강조·취소선·링크·HTML 짝 태그는 링크·HTML은 정확히 한 번, 강조·취소선은 한 번 이상(분할 허용),
  중첩이 맞아야 함, `<xN/>`·`<mN/>`는 정확히 한 번, 원문 코드는 모두 맞춰져야 함.
- 피드백 문장 예: "<a4> is missing — keep every tag exactly once", "<b2> closes inside <i1>",
  "code `fold` is not in the source (did `fold=0` change?)", "code `x` is missing".

- [ ] 테스트: 번호 없는 닫기, 종류 글자 교정, 지어낸 태그 버림, 백틱 안 `Vec<i32>`, 코드 맞추기 다섯 경우
  (이미 맞춘 코드를 뒤에서 한 번 더 바꾼 경우가 5), 강조 분할 허용, 링크 두 번은 문제, 중첩 어긋남은 문제,
  빠진 원문 코드는 문제.
- [ ] 실패 확인 → 구현 → 통과.

### Task 4: 직렬화와 확인 (`inline::markdown::render`, `verify`)

**Files:**
- Modify: `crates/translate/src/inline/markdown.rs`

**Interfaces:**
- Produces:
  ```rust
  pub struct Rendered { pub markdown: String, pub strategies: Vec<(usize, &'static str)>, pub verified: bool }
  pub fn render(tree: &Tree, tagged: &Tagged, ctx: &DocContext) -> Rendered;
  ```
- 렌더링: 텍스트는 최소 이스케이프(spec 3.3; 표 셀은 `|`도), 코드 노드는 맞춘 원문 스팬 바이트(또는
  4번 경우 모델이 쓴 스팬), void는 원문 바이트, 링크는 `[`+내용+꼬리(단축·축약은 내용이 레이블과 다르거나
  뒤에 `(`·`[`가 오면 `][label]`), HTML 짝은 원문 여는·닫는 태그.
- 전략 탐색: 결과를 pulldown-cmark(같은 콜백)로 다시 읽은 모양·글자를 의도(트리)와 비교해 점수
  0이 될 때까지 강조 태그마다 `Star → Shrink → SwapLink → QuotesOut → ParticleIn → Html` 순으로 탐욕 적용
  (스파이크 `transform`/`score`와 같다). 끝내 0이 아니면 모든 강조를 Html로 바꿔 한 번 더 확인,
  `verified`에 결과를 적는다.

- [ ] 테스트: 각 전략의 대표 예(`**외적**(outer product)에서`, `*arity*는`, `[**Fetch**](u)는`,
  `"*권한*"은`, `*…`impl Trait`의*`), 단축 참조 규칙(`[x][x](GCI)` 방지), 표 셀 `|`, 줄 머리 `# `,
  식별자 속 `_`는 이스케이프하지 않음, 대괄호는 이스케이프하지 않음, `verified` 실패 시 Html 폴백.
- [ ] 실패 확인 → 구현 → 통과.

### Task 5: 말뭉치 왕복 테스트

**Files:**
- Create: `crates/translate/tests/inline_roundtrip.rs` (`#[ignore]`, 환경 변수 `YEOKJA_PROJECTS`로 켬)

- [ ] 저장된 `markdown` 파서 프로젝트(rustc-dev-guide, rust-forge, furiosa-opt,
  webassembly-component-docs, learn-fpga, putting-the-you-in-cpu의 markdown 소스)의 모든 세그먼트를
  `tagify` → 태그 텍스트를 그대로 `read` → `render`해 원문과 렌더링 HTML이 같은 비율을 센다.
  태그화할 수 없는 것은 따로 센다. 기준: HTML 동일 99.99% 이상.
- [ ] 실행해 수치를 기록한다.

### Task 6: 프롬프트

**Files:**
- Modify: `crates/translate/src/provider.rs` (`TranslateRequest.inline_tags: bool`)
- Modify: `crates/translate/src/prompt.rs` (태그 규칙, `{inline_tags}` 자리표시자, 테스트)
- Modify: 모든 `TranslateRequest { … }` 생성 지점(12곳)에 `inline_tags: false`

- [ ] 테스트: `inline_tags`면 `closing_rule` 대신 태그 규칙(스파이크 S1b 문구 + "문맥의 태그는 참고용")이
  들어가고 GFM 경고 표시 문장은 남는다, 꺼지면 지금과 같다, 커스텀 템플릿의 `{inline_tags}`가 켜면
  규칙으로 꺼지면 빈 문자열로 바뀐다.
- [ ] 실패 확인 → 구현 → 통과.

### Task 7: 파이프라인

**Files:**
- Modify: `crates/translate/src/pipeline.rs`
- Modify: `crates/translate/src/evaluator_format.rs` (`ParenthesisEvaluator`: `parenthesis_issues`만)

**Interfaces:**
- Produces: `pub struct InlineBatch { pub tagged: HashMap<usize, Tagged>, pub sources: HashMap<usize, String>, pub ctx: DocContext }`;
  `translate_with_evaluation_observed(…, inline: Option<&InlineBatch>, on_event)` (새 인자),
  `translate_with_evaluation`은 `None`을 넘긴다.
- 태그 모드 흐름(spec 3.4): 요청 세그먼트는 태그 텍스트(재시도에도 같은 텍스트·번호) → 번호 무결성
  (응답 기준, 지금과 같음) → 세그먼트마다 `read`(실패는 피드백과 함께 그 세그먼트 재시도) → `render`
  (`verified == false`면 경고) → `alignment::misaligned`를 직렬화한 Markdown과 **원문**(`sources`)으로
  → 평가기(원문과 직렬화한 Markdown). 마크업 보정은 돌리지 않는다. 전략은 `tracing::debug!`.

- [ ] 테스트(MockProvider): 태그 응답이 직렬화되어 저장된다, 태그 문제는 그 세그먼트만 재시도하며
  피드백에 태그 이름이 든다, 직렬화 뒤 정렬 검사가 원문으로 돈다, `inline: None`이면 지금과 같다.
- [ ] 실패 확인 → 구현 → 통과.

### Task 8: orchestrator

**Files:**
- Modify: `crates/translate/src/orchestrator.rs`

- [ ] `translate_file`: `config.source_for(file)`의 `inline_tags`와 파서 이름을 확인해 켜져 있으면
  `DocContext::from_markdown(&doc.source)`를 만들고, 세그먼트 ID → `Position`(블록 종류가
  `BlockType::Table`이면 `TableCell`) 표를 만든다.
- [ ] 블록 묶음마다 문맥의 모든 세그먼트(대기 중이 아닌 것 포함)를 한 카운터로 태그화하고, 대기 중
  세그먼트의 `Tagged`를 `InlineBatch`로 모은다. 하나라도 `Untaggable`이면 그 요청은 기존 경로
  (`inline: None`, 원문 문맥)로 보내고 `tracing::info!`로 센다.
- [ ] `standard_evaluators`에 태그 모드 인자를 더해 `FormatEvaluator` 대신 `ParenthesisEvaluator`를 쓴다.
- [ ] auto_evaluate가 꺼진 경로도 태그 모드면 파이프라인(평가기 없이)을 탄다.
- [ ] 테스트: 켜진 소스의 요청이 태그 텍스트로 가고 결과가 Markdown으로 저장된다(모의 provider),
  태그화할 수 없는 세그먼트가 든 요청은 원문으로 간다, 꺼진 소스는 지금과 같다.

### Task 9: A/B와 사람 판독

- [ ] 스크래치 사본 두 벌(rustc-dev-guide: 설정·용어집 복사, `upstream` 링크, `state/` 사본에서 세 챕터
  번역을 비움)에서 하나는 켜고 하나는 끈 채로 각 두 번 번역한다(RUST_LOG=debug로 시도 수와 태그
  응답을 남김).
- [ ] 채점: 스파이크 채점 방식(구조 결함, 코드 뒤 조사 일치)과 재시도율, 기존 경로 비율.
- [ ] 태그 응답 100개 이상을 사람이 읽어 엉뚱한 낱말을 감싼 태그를 센다.
- [ ] 합격(spec 4절)하면 `projects/rustc-dev-guide/yeokja.toml`의 `markdown` 소스에 `inline_tags = true`.

### Task 10: 마무리

- [ ] `cargo test --workspace`, `cargo clippy -p yeokja-translate`(새 경고 없음).
- [ ] spec에 A/B 결과를 기록하고, AGENTS.md나 `docs/translation-project-checklist.md`에 켜는 방법과 조건을 적는다.
- [ ] 커밋: 기능(Task 1~8) → A/B 결과와 설정(Task 9~10).
