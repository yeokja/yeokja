# 파서 충실도 개선 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 파서·재구성이 원문 구조를 바꾸는 여섯 갈래 결함(AsciiDoc 스타일 문단, RST 도메인 지시자·
graphviz, RST 재구성 줄 합침, RST 목록·제목 인식, Markdeep 인용문, 강조 안의 강제 줄바꿈, LaTeX
차례 문구)을 고치고, 저장된 번역은 다시 번역하지 않고 새 파서로 옮긴다.

**Architecture:** 먼저 모델 호출 없이 state를 새 파서 모양으로 옮기는 `yeokja reconcile`을 만든다.
그다음 파서별로 고치고, 프로젝트별로 `reconcile --dry-run` → `reconcile` → 새 세그먼트만
`translate` 순서로 이관한다.

**Tech Stack:** Rust 2024(작업 공간 crate들), pulldown-cmark 0.12, Python docutils(검증 전용, 기존
Sphinx venv).

**Spec:** `docs/superpowers/specs/2026-09-27-parser-fidelity-design.md`

## Global Constraints

- 커밋은 Secretive(Touch ID) SSH 서명이다. 서명을 끄거나 우회하지 않는다. 사람이 없으면 변경을
  커밋하지 않은 채 두고, 계획한 커밋을 순서대로 만드는 스크립트를 스크래치에 남긴다.
- 커밋 단위: 파서 변경(코드+테스트)은 Task마다 한 커밋, 프로젝트 이관(state·`ko/`·스크립트)은
  프로젝트마다 한 커밋. 커밋 메시지 형식은 저장소 관례(`[crate] fix: …`, `[project] chore: …`).
- 새 세그먼트 번역: 프로젝트 디렉터리에서 `../../target/release/yeokja translate <파일>`
  (pypy는 반드시 파일 단위). `reconcile` 없이 `translate`를 먼저 부르지 않는다 — 문맥 변경
  세그먼트까지 다시 번역된다.
- peps 이관(Task 9의 peps 단계)은 보류 중인
  `yeokja -C projects/peps fuse upstream/peps --previous 24aa727`이 끝난 뒤에만 한다.
- 건드리지 말 것: `evals/ko-translation/v1/` 동결 파일, `judgments/*/page-pairs.json`·`human.jsonl`.
- push 전: `.github/pages-projects.json`의 모든 프로젝트에 `.github/scripts/rebuild-translations.sh`
  를 돌려 0이 아닌 종료만 본다. 이 스크립트는 로컬에서 남은 세그먼트를 조용히 번역하므로
  `git diff`를 검토한다.

---

### Task 1: `yeokja reconcile`

**Files:**
- Create: `crates/cli/src/commands/reconcile.rs`
- Modify: `crates/cli/src/commands/mod.rs`, `crates/cli/src/main.rs`(서브커맨드),
  `crates/core/src/reconcile.rs`(문맥 받아들이기 함수와 테스트)
- Modify: `docs/translation-project-checklist.md`(파서 변경 뒤 이관 절차 한 절)

**Interfaces:**
- Produces: `pub fn reconcile_accepting_context(document: &Document, existing: &StateFile, glossary: &Glossary) -> ReconcileReport`
  (`struct ReconcileReport { segments: Vec<SegmentState>, kept: usize, added: Vec<String>, dropped: Vec<String>, context_accepted: usize, glossary_stale: usize }`,
  `added`/`dropped`는 원문 텍스트). 규칙: `reconcile` 결과에서 상태가 `ContextChanged`인
  세그먼트만 `context_hash`를 현재 값으로 바꾼다. `GlossaryStale`은 그대로 두고 센다. ID로 짝짓지
  않으므로 원문이 바뀐 세그먼트는 `added`와 `dropped`에 나온다. state가 없는 파일은 건너뛴다.
- CLI: `Reconcile { path: String, #[arg(long)] dry_run: bool }`. `collect_files`/`scan_file`
  (`yeokja_translate::orchestrator`)로 파일을 모으고, `--dry-run`이 아니면
  `StateFile::save`로 저장한다. 출력은 파일별 한 줄 요약과 `added`/`dropped` 원문 목록.

- [ ] **Step 1: 실패하는 테스트** — `crates/core/src/reconcile.rs`의 `mod tests`에 세그먼트 셋
  (A, B, C)의 state와, B가 빠지고 D가 A 앞에 생긴 문서를 만든다. 단언: A·C는 번역을 유지하고
  `context_hash`가 새 값, D는 `translation: None`, B는 결과에 없음, `context_accepted == 2`.
  원문이 바뀐 세그먼트가 섞인 경우 `translation`과 옛 `source_hash`가 그대로(Stale 유지)인지도
  단언한다.
- [ ] **Step 2: 구현** 후 `cargo test -p yeokja-core reconcile` 통과 확인.
- [ ] **Step 3: CLI 연결** — `crates/cli/tests/`(없으면 `commands/reconcile.rs`의 테스트 모듈)에
  임시 프로젝트 디렉터리로 `--dry-run`이 파일을 바꾸지 않고, 본 실행 뒤 `status --check`가
  Pending만 세는지 확인하는 테스트.
- [ ] **Step 4: 체크리스트 문서** — "파서를 바꾼 뒤" 절: `reconcile --dry-run`으로 수 확인 →
  `reconcile` → 새 세그먼트만 `translate`. 원문 서브모듈을 올린 뒤에는 쓰지 않는다는 이유 한 문장.
- [ ] **Step 5: 커밋** `[cli] feat: 파서 변경 뒤 state를 옮기는 reconcile 명령`

### Task 2: AsciiDoc 스타일 문단 (spec 1절)

**Files:** Modify `crates/parser-asciidoc/src/lib.rs`(파싱 상태, 속성 줄 분기, 테스트)

**Interfaces:** `fn verbatim_paragraph_style(attribute_line: &str) -> bool` — `[` 뒤 첫 쉼표(또는
`]`) 앞 토큰에서 `#`/`.`/`%` 앞까지를 스타일로 보고 spec의 목록과 비교. 상태에
`pending_verbatim: bool`, `verbatim_paragraph: Option<usize>`(시작 오프셋).

- [ ] **Step 1: 실패하는 테스트**

```rust
    #[test]
    fn styled_source_paragraph_without_delimiters_is_opaque() {
        let source = "Before.\n\n[source, C]\nBeamInstr* next;\nPreFetch(2, next);\n\nAfter.\n";
        let doc = AsciidocParser.parse(source);
        let sources: Vec<_> = doc.translatable_segments().iter().map(|s| s.source.clone()).collect();
        assert_eq!(sources, ["Before.", "After."]);
        let output = translate_all(source, &[("Before.", "앞."), ("After.", "뒤.")]);
        assert_eq!(output, "앞.\n\n[source, C]\nBeamInstr* next;\nPreFetch(2, next);\n\n뒤.\n");
    }
```

  함께: `[source,erlang]`, `[listing]`, `[source#ex]`는 불투명; `[NOTE]` 문단과 `[[id]]` 뒤 문단은
  번역; 속성 줄과 문단 사이에 빈 줄이 있으면 번역; `[source]` 뒤 `----` 구분자는 기존대로.
  (`translate_all`이 이 파일 테스트 모듈에 없으면 parser-rst의 것과 같은 모양으로 만든다.)
- [ ] **Step 2: 구현** — 속성 줄 분기에서 `pending_verbatim = verbatim_paragraph_style(trimmed)`
  (다른 `[…]` 줄은 덮어씀). 빈 줄에서 `pending_verbatim = false`, 열린 `verbatim_paragraph`가
  있으면 `push_opaque_block(CodeBlock, start..직전 줄 끝)`. 일반 텍스트 줄이 올 때
  `pending_verbatim`이면 `verbatim_paragraph = Some(line_start)`로 열고 그 뒤 줄은 빈 줄까지
  건너뛴다. 구분자 줄이면 `pending_verbatim = false` 후 기존 분기. 문서 끝에서도 닫는다.
- [ ] **Step 3:** `cargo test -p yeokja-parser-asciidoc`, 말뭉치 왕복(번역 없이 재구성 = 원문)을
  theBeamBook 모든 `.asciidoc`에 돌리는 기존 테스트 또는 스크래치 스크립트로 확인.
- [ ] **Step 4: 커밋** `[parser-asciidoc] fix: 구분자 없는 [source] 문단을 코드로 두기`

### Task 3: RST 도메인 지시자와 graphviz (spec 2절)

**Files:** Modify `crates/parser-rst/src/lib.rs` (`parse_directive`, `OPAQUE_DIRECTIVES`, 테스트)

- [ ] **Step 1: 실패하는 테스트** — `.. py:function:: getexecutioncontext()\n\n   Return the currently active execution context.\n`
  → 세그먼트 하나(본문). `.. c:function:: int f(void)\n\n   Does things.\n`도 같음.
  `.. graphviz::\n\n    digraph G{\n    a -> b; // c\n    }\n` → 세그먼트 없음, 재구성 바이트 동일.
  `.. digraph:: t\n\n   a -> b\n`도 같음. `.. _target: http://x`와 `.. |sub| replace:: x`는
  여전히 지시자가 아님(기존 테스트 유지).
- [ ] **Step 2: 구현** — 이름 문법: 첫 글자 영숫자, 나머지 영숫자와 `-_.+:`, 마지막 글자가 `:`가
  아님. `OPAQUE_DIRECTIVES`에 `graphviz`, `graph`, `digraph`를 넣고 배열 길이를 고친다.
- [ ] **Step 3:** `cargo test -p yeokja-parser-rst`
- [ ] **Step 4: 커밋** `[parser-rst] fix: 도메인 지시자 본문 번역, graphviz 본문은 그대로`

### Task 4: RST 열거 표지·한 글자 밑줄·glossary 용어 (spec 4절)

**Files:** Modify `crates/parser-rst/src/lib.rs` (`parse_enumerated_item`, 파싱 루프의 목록 분기,
`adornment_char` 호출부, glossary 처리, `indented_alpha_enumeration_edit`/`alpha_enumerator` 제거, 테스트)

**Interfaces:**
- Produces: `fn enumerator(content: &str) -> Option<Enumerator>`
  (`struct Enumerator { kind: EnumKind, ordinal: Option<u32>, text_offset: usize }`,
  `EnumKind::{Arabic, Auto, LowerAlpha, UpperAlpha, LowerRoman, UpperRoman}`, `(x)`/`x)`/`x.` 형식 구분 포함),
  `fn reads_as_block_start(first_line: &str, next_line: Option<&str>, previous: Option<&Enumerator>) -> bool`.
  Task 5가 `reads_as_block_start`를 쓴다.

- [ ] **Step 1: 실패하는 테스트**
  - pep-0818 모양: `    a. Let x.\n    b. For each:\n\n        i.   Set a\n        ii.  Set b\n        iii. Set c.\n`
    → 세그먼트 `Let x.`, `For each:`, `Set a`, `Set b`, `Set c.`; 번역 후 표지와 공백이 원문 그대로.
  - pep-0209 모양: `    a.  Use a model\n\n        Instead of ...\n` → 표지 `a.  `가 원문 그대로 남고 본문 열(8) 유지.
  - pep-0497 모양: `a. It should enable\nto re-enable.\n` → 문단 하나(원문 `a. It should enable to re-enable.`).
  - `h. x\ni. y\n`(빈 줄 없이 연속)에서 `i`는 알파벳, 단독 `i. x\n\n`은 로마.
  - 기존 `an_initial_is_not_a_list_marker`를 "다음 줄이 이어지면 문단" 단언으로 바꾸고,
    `J. Smith wrote this.\n\nNext.\n`은 목록 항목(docutils와 같음)임을 단언.
  - pep-0450 모양: `Intro.\n\nR\n-\n\nR is a language.\n` → `R`가 제목, 새 절 시작.
  - glossary: `.. glossary::\n\n    license expression\n    SPDX expression\n        A string.\n`
    → 세그먼트 `license expression`, `SPDX expression`, `A string.`.
- [ ] **Step 2: 구현** — 목록 분기에서 `enumerator` + `reads_as_block_start(line, next, prev)`를
  함께 본다. 같은 들여쓰기의 직전 표지를 `ParseState`에 두고 빈 줄이 아닌 다른 블록에서 지운다.
  한 글자 밑줄은 "직전 줄이 한 줄짜리 텍스트 run"일 때만 `adornment_char`의 길이 조건을 1로
  완화하는 별도 함수 `underline_char`로 받는다. glossary 지시자는 본문 들여쓰기에서 정의보다 얕은
  줄마다 `push_span_block(BlockType::Heading, 줄 텍스트, BlockRole::None)`.
  `indented_alpha_enumeration_edit`와 `alpha_enumerator`, `reconstruct_rst`의 호출부를 지운다.
- [ ] **Step 3:** `cargo test -p yeokja-parser-rst` 전체 통과(기존 `indented_alpha_enumeration_keeps_each_item_on_its_source_line`은
  같은 단언이 새 경로로 통과해야 한다).
- [ ] **Step 4: 커밋** `[parser-rst] fix: docutils 규칙으로 열거 표지·한 글자 밑줄·glossary 용어 읽기`

### Task 5: RST 재구성의 첫 줄 이스케이프와 이어짐 줄 (spec 3절)

**Files:**
- Modify `crates/parser-utils/src/lib.rs` (`split_at_safe_space`)
- Modify `crates/core/src/model.rs` (`BlockRole::ContinuationColumn { column: usize }`)
- Modify `crates/parser-rst/src/lib.rs` (`Run`에서 역할 판정, `reconstruct_rst`, 테스트)

**Interfaces:**
- Produces: `pub fn split_at_safe_space(text: &str, target_width: usize, is_safe: impl Fn(usize) -> bool) -> Option<usize>`
  — `text`의 공백 바이트 위치 가운데 `is_safe`가 참이고, 뒤쪽 조각에 영숫자·한글이 하나 이상
  있는 곳 중 앞쪽 조각 표시 폭이 `target_width`에 가장 가까운 곳. Task 6이 쓴다.

- [ ] **Step 1: 실패하는 테스트**
  - `* (2009) Dino offers his opinion:\n  https://x/y\n` + 번역 `(2009) Dino의 의견: https://x/y`
    → `* \(2009) Dino의 의견: https://x/y\n`.
  - `.. [4] 1976. R. H. Lathwell. APL\n   the eighth\n` → 번역문 앞에 `\`.
  - 원문도 목록인 `* 1. foo\n` → 이스케이프 없음.
  - 번역만 표지로 시작(`Compare (1) with (2).` → `(1) 호출하기 …`) → 이스케이프.
  - pep-3144 모양: `.. [1] Appealing to authority\n       follows:\n\n          I have seen\n          traffic.\n`
    → 첫 문단 번역이 두 줄, 둘째 줄이 7칸에서 시작; 인용 문단은 10칸 그대로.
  - 나눌 공백이 백틱 안에만 있으면 한 줄 그대로; 둘째 조각이 `---`뿐이면 그 위치를 쓰지 않음.
- [ ] **Step 2: 구현** — 파싱 루프에서 각주·인용·첫 줄 내용 지시자·필드 표지로 시작한 `Run`이
  닫힐 때, 원문 줄 수 ≥ 2이고 같은 본문(들여쓰기 블록) 안에 `text_col`보다 깊은 줄이 뒤따르면
  블록 역할을 `ContinuationColumn { column: text_col }`로 둔다. `reconstruct_rst`에서 이 역할의
  블록은 `split_at_safe_space`(안전 판정: `backtick_spans` 밖, `|…|` 밖)로 나눈 뒤
  `"\n" + " ".repeat(column)`을 끼워 넣는 splice로 바꾼다. 첫 줄 이스케이프는 `apply_splices`
  직후 블록마다(원문 span의 첫 줄·다음 줄, 번역 결과의 첫 줄·다음 줄) `reads_as_block_start`를
  비교해 `\`를 삽입하는 edit로 한다.
- [ ] **Step 3:** `cargo test -p yeokja-core -p yeokja-parser-utils -p yeokja-parser-rst`.
  `BlockRole`을 match하는 다른 crate가 컴파일되는지 `cargo build --workspace`.
- [ ] **Step 4: 커밋** `[parser-rst] fix: 줄 합침이 docutils의 목록·들여쓰기 해석을 바꾸지 않게`

### Task 6: Markdeep 인용문 줄 나누기 (spec 5절)

**Files:** Modify `crates/parser-markdeep/src/lib.rs`(reconstruct, 테스트)

- [ ] **Step 1: 실패하는 테스트** — `> one two three four\n> five six seven.\n` + 긴 번역 →
  출력이 `> `로 시작하는 줄 두 개 이상. 코드 스팬만 있는 짧은 번역(`` `x` ``) → `> …\n>` 두 줄.
  인용문 밖 문단은 기존대로 한 줄.
- [ ] **Step 2: 구현** — 블록 타입이 `BlockQuote`이고 원문 span이 두 줄 이상이면, 번역문을
  원문 가장 긴 줄 폭으로 `split_at_safe_space`를 반복 적용해 나눈다(안전 판정: 번역문을
  pulldown-cmark로 읽은 `Event::Text` 범위 안의 공백). 줄 사이에 원문 둘째 줄의 접두어
  (`> ` 등, span 안 줄머리에서 복사)를 넣는다. 결과가 한 줄이면 `\n>`을 덧붙인다.
- [ ] **Step 3:** `cargo test -p yeokja-parser-markdeep`
- [ ] **Step 4: 커밋** `[parser-markdeep] fix: 인용문을 두 줄 이상으로 재구성`

### Task 7: 강조 안의 강제 줄바꿈 (spec 6절)

**Files:** Modify `crates/parser-markdown-dialect/src/lib.rs`(`ParseState`, `HardBreak` 분기,
`extend_run`, 테스트), `crates/parser-mdx/src/lib.rs`(테스트만)

- [ ] **Step 1: 실패하는 테스트** (parser-mdx 테스트 모듈)

```rust
    #[test]
    fn hard_breaks_inside_emphasis_survive() {
        let source = "> *A: Knock, knock!  \n> B: Who's there?  \n> A: **MOO!***\n";
        let doc = MdxParser.parse(source);
        let sources: Vec<_> = doc.translatable_segments().iter().map(|s| s.source.clone()).collect();
        assert_eq!(sources, ["*A: Knock, knock!", "B: Who's there?", "A: **MOO!***"]);
        let output = translate_all(source, |s| s.replace("Knock, knock!", "똑똑!"));
        assert_eq!(output, "> *A: 똑똑!  \n> B: Who's there?  \n> A: **MOO!***\n");
    }
```

  백슬래시 강제 줄바꿈(`\` 줄 끝)도 같은 테스트 하나.
- [ ] **Step 2: 구현** — `ParseState`에 `resume_end: Option<usize>`. `HardBreak`에서 run이 있고
  `run.end > range.start`면 `resume_end = Some(run.end)`, `run.end = range.start`(뒤쪽 공백 제외)
  후 `flush_run`. `extend_run`에서 새 run을 열 때 `resume_end.take()`가 있으면 끝을 그 값과의
  최댓값으로. 블록 경계(`flush_run`을 부르는 Start/End 태그)에서 `resume_end = None`.
- [ ] **Step 3:** `cargo test -p yeokja-parser-markdown-dialect -p yeokja-parser-markdown -p yeokja-parser-mdx -p yeokja-parser-myst -p yeokja-parser-mkdocs -p yeokja-parser-markdeep`
- [ ] **Step 4: 커밋** `[parser-markdown-dialect] fix: 강조 안의 강제 줄바꿈 보존`

### Task 8: LaTeX `\addcontentsline` (spec 7절)

**Files:** Modify `crates/parser-latex/src/lib.rs` (`is_layout_command`, 새 `TOC_TEXT_COMMANDS` 처리, 테스트)

- [ ] **Step 1: 실패하는 테스트** — `\chapter*{Preface}\n\addcontentsline{toc}{chapter}{Preface}\n`
  → 세그먼트 `Preface` 두 개(제목, 차례), 재구성 `\addcontentsline{toc}{chapter}{서문}`.
  `\addcontentsline{toc}{section}{\ExercisesName}` → 세그먼트 없음. `%\addcontentsline{…}{…}{X}` → 없음.
- [ ] **Step 2: 구현** — 셋째 균형 중괄호 인자의 내부 범위를 `builder.add_text(…, BlockType::Heading)`
  (앞뒤 `flush`). 인자가 `\이름` 하나뿐이면 건너뜀. `addcontentsline`을 `is_layout_command`에서 뺀다.
- [ ] **Step 3:** `cargo test -p yeokja-parser-latex`
- [ ] **Step 4: 커밋** `[parser-latex] fix: \addcontentsline 차례 문구 번역`

### Task 9: 프로젝트 이관

`cargo build --release -p yeokja-cli` 뒤, 프로젝트마다 아래를 한다. 각 단계의 수를 커밋
메시지에 적는다(문서에는 적지 않는다).

- [ ] **Step 1: 미리보기** — 모든 번역 프로젝트에서
  `../../target/release/yeokja reconcile . --dry-run`. 예상 밖 프로젝트(spec이 영향 없다고 한
  곳)에서 `added`/`dropped`가 나오면 멈추고 원인을 적은 뒤 사용자에게 넘긴다.
- [ ] **Step 2: thebeambook** — `reconcile .`; `added` 0, `dropped` 10 확인; 빌드 후 Task 10 확인.
- [ ] **Step 3: pypy** — `reconcile .`; 새 세그먼트가 든 파일마다 `translate <파일>`
  (`objspace.rst` 등). graphviz 세그먼트 11개가 `dropped`인지 확인.
- [ ] **Step 4: cpython-devguide, mil** — 같은 절차(도메인 지시자 3개, 열거 표지 변화 여부).
- [ ] **Step 5: peps** — 보류 중인 합성이 끝났는지 `git log -- projects/peps/state`로 확인한 뒤
  같은 절차. pep-0639 용어 6개는 번역문을 원문 그대로(`license expression` 등) 둔다 —
  `:term:` 참조 대상이 영어 표제어이기 때문이다(기존 번역과 같음).
- [ ] **Step 6: raytracing, putting-the-you-in-cpu 및 dialect 영향 프로젝트** — Step 1 결과대로.
- [ ] **Step 7: jeffe-algorithms, hott** — `reconcile .` 뒤 새 차례 세그먼트의 번역을 같은 파일의
  같은 원문 제목 세그먼트 번역으로 채운다(스크래치 파이썬: state에서 `source`가 같은 Heading
  세그먼트의 `translation`·`glossary_snapshot` 복사, `translated_at`은 현재 시각). 제목이 없는
  항목(`Image Credits`, `Colophon`, `Index of symbols`)은 `translate <파일>`. HoTT는
  `symbols.tex`를 `기호 찾아보기`, `introduction.tex`를 `서론`으로 맞춘 뒤
  `projects/hott/scripts/prepare_pdf.py`의 두 치환과 `test_prepare_pdf.py`의 해당 단언을 지운다.
- [ ] **Step 8:** 각 프로젝트 `status .` 결과가 Pending/Stale/ContextChanged 0.

### Task 10: 검증과 마무리

- [ ] **Step 1: docutils 대조** — 스크래치 스크립트: peps·pypy·cpython-devguide의 원문과 `ko/`
  RST를 docutils(기존 Sphinx venv, 알 수 없는 역할·지시자는 일반 노드로 등록)로 읽어 문서마다
  `enumerated_list`, `bullet_list`, `list_item`, `block_quote`, `definition_list`, `section`,
  `term` 수를 비교한다. 감사 대상 8개 PEP(0421, 0485, 0450, 3144, 0818, 0209, 0639, 0497)의
  차이가 0이고, 이관 전보다 차이가 늘어난 문서가 없어야 한다.
- [ ] **Step 2: 빌드 확인** — 각 프로젝트 `yeokja build <target>` 후:
  theBeamBook `<code data-lang="C">` 블록 줄 수가 원문과 같음; pypy `objspace.html`의
  `getexecutioncontext` 설명이 한국어, `translation.html` 도표 SVG에 `Transformation Step`
  범례 간선; raytracing 1권 HTML 인용문이 `<blockquote>`; putting-the-you-in-cpu 6장 인용문에
  `<br` 4개; jeffe PDF `pdftotext -f 1 -l 20` 차례에 `Preface`/`Colophon` 없음; HoTT PDF 차례의
  `기호 찾아보기`·`서론` 유지.
- [ ] **Step 3: 전체 상태** — `.github/pages-projects.json`의 모든 프로젝트에
  `.github/scripts/rebuild-translations.sh <project>`; 0이 아닌 종료 없음, `git diff`에 예상 밖
  재번역 없음.
- [ ] **Step 4:** 사용자에게 커밋·push 승인 요청(서명 필요).
