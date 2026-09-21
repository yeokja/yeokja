# CommonMark 강조와 AsciiDoc 제약 쌍 보정 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 문장 부호 뒤에서 닫히지 않는 CommonMark 강조를 평가기가 잡고 괄호 풀이 꼴은 결정적으로
고치며, AsciiDoc 조사 앞 한 겹 제약 쌍을 두 겹으로 보정한 뒤, 저장된 번역의 해당 결함을 모두
바로잡는다.

**Architecture:** `FormatEvaluator`가 `Markup::Markdown`의 강조 쌍을 pulldown-cmark로 세어
원문과 비교한다(MkDocs·Verso는 제외). 번역 직후 평가 전에 파이프라인이 markup별 보정을
부른다: RST(기존) `repair_rst_boundaries`, Markdown(새) `repair_markdown_emphasis`, AsciiDoc(새)
`repair_asciidoc_boundaries`. 보정은 결과를 파서 규칙으로 다시 읽어 나아질 때만 받아들인다.

**Tech Stack:** Rust 2024, pulldown-cmark 0.12(작업 공간에 이미 있음), tokio 테스트.

**Spec:** `docs/superpowers/specs/2026-09-21-translation-robustness-design.md` 7·8절

## Global Constraints

- pulldown-cmark는 `parser-markdown-dialect`와 같은 `0.12`를 쓴다(새 다운로드 없음).
- 새 규칙은 `Markup::Markdown`에만 적용한다. MkDocs(Python-Markdown)는 `*`에 측면 규칙이 없고,
  Verso는 자체 파서다.
- 커밋은 Secretive(Touch ID) SSH 서명이다. 서명을 끄거나 우회하지 않는다.
- 재번역: `state/**/*.yeokja.json`에서 세그먼트의 `"translation"`을 `null`로 바꾸고, 프로젝트
  디렉터리에서 `../../target/release/yeokja translate <파일>`을 실행한다(pypy는 반드시 파일 단위).
- 감사: 프로젝트 디렉터리에서 `../../target/release/yeokja evaluate --mechanical-only .`
- 로컬 빌드 전에 `.github/scripts/rebuild-translations.sh <project>`로 `ko/`를 다시 만든다.
- 건드리지 말 것: rustc-dev-guide `diagnostics/diagnostic-structs.md` section:1/block:70/seg:0
  (원문 오타), learn-fpga `LiteX/orange_crab.md` section:0/block:2/seg:0, thebeambook
  `chapters/live.asciidoc` section:0/block:1/seg:0, jeffe-algorithms `chapters/00-intro.tex`
  section:2/block:3/seg:0, thebeambook `compiler.asciidoc` section:0/block:7/seg:0~2(재번역 금지),
  spec 6절 "그대로 두는 항목".

---

### Task 1: CommonMark가 읽는 강조를 세는 검사

**Files:**
- Modify: `crates/translate/Cargo.toml` (의존성 추가)
- Modify: `crates/translate/src/evaluator_format.rs` (`evaluate`, `markup_issues`, 새 함수, 테스트)

**Interfaces:**
- Produces: `fn commonmark_emphasis(text: &str) -> CommonMarkEmphasis`
  (`struct CommonMarkEmphasis { formed: usize, stray: Vec<std::ops::Range<usize>> }`, `stray`는
  char 인덱스 범위), `fn left_flanking(before: Option<char>, after: Option<char>) -> bool`,
  `fn commonmark_punctuation(c: char) -> bool`. Task 2가 `commonmark_emphasis`와
  `without_reference_labels`를 쓴다.

- [ ] **Step 1: 실패하는 테스트 작성** (`mod tests` 끝에 추가)

```rust
    #[tokio::test]
    async fn markdown_emphasis_does_not_close_between_punctuation_and_a_particle() {
        let ctx = context_in(
            Markup::Markdown,
            "\"Outer\" comes from the **outer product** of linear algebra.",
            "\"Outer\"는 선형대수학의 **외적(outer product)**에서 유래합니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        let message = &result.issues.iter().find(|i| i.message.contains("never closes")).unwrap().message;
        assert!(message.contains("**외적(outer product)**에서"), "{message}");
        assert!(message.contains("**외적**(outer product)에서"), "{message}");
    }

    #[tokio::test]
    async fn markdown_emphasis_ending_in_code_or_a_link_does_not_close_before_a_particle() {
        for (source, translation) in [
            ("Short for _argument-position `impl Trait`_.", "*argument-position `impl Trait`*의 줄임말입니다."),
            ("**[Fetch](./fetch-engine.md)** reads DM.", "**[Fetch](./fetch-engine.md)**는 DM을 읽습니다."),
            ("approved by *FCPs* or *r+*.", "*FCP* 또는 *r+*로 승인합니다."),
        ] {
            let ctx = context_in(Markup::Markdown, source, translation);
            assert!(!FormatEvaluator.evaluate(&ctx).await.unwrap().passed, "{translation}");
        }
    }

    #[tokio::test]
    async fn markdown_emphasis_that_ends_on_a_letter_closes() {
        for (source, translation) in [
            ("comes from the **outer product** of it.", "**외적**(outer product)에서 유래합니다."),
            ("Short for _argument-position `impl Trait`_.", "*argument-position `impl Trait`의* 줄임말입니다."),
            ("**[Fetch](./fetch-engine.md)** reads DM.", "[**Fetch**](./fetch-engine.md)는 DM을 읽습니다."),
            ("No _\"formal power\"_: none.", "\"*공식 권한*\"은 없습니다: 없습니다."),
        ] {
            let ctx = context_in(Markup::Markdown, source, translation);
            let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
            assert!(result.passed, "{translation}: {:?}", result.issues);
        }
    }

    #[tokio::test]
    async fn mkdocs_stars_close_against_a_particle_after_punctuation() {
        // Python-Markdown has no flanking rule for `*`.
        let ctx = mkdocs_ctx(
            "**Simulated Annealing (SA)** is a randomized algorithm.",
            "**시뮬레이티드 어닐링(SA)**은 무작위 알고리즘입니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn markdown_source_with_an_unclosed_mark_is_no_measure() {
        // rustc-dev-guide `diagnostic-structs.md`: the source forgets its closing `_`.
        let ctx = context_in(
            Markup::Markdown,
            "_Applied to `Span` fields on `Subdiagnostic`s.",
            "*`Subdiagnostic`*의 `Span` 필드에 적용됩니다.",
        );
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.issues.iter().any(|i| i.message.contains("never closes")), "{:?}", result.issues);
    }

    #[tokio::test]
    async fn a_markdown_underscore_before_a_particle_gets_one_message() {
        let ctx = context_in(Markup::Markdown, "The _arity_ is the count.", "_arity_는 개수입니다.");
        let result = FormatEvaluator.evaluate(&ctx).await.unwrap();
        assert!(!result.passed);
        assert_eq!(result.issues.len(), 1, "{:?}", result.issues);
    }

    #[test]
    fn commonmark_emphasis_counts_what_forms_and_names_what_prints() {
        let read = commonmark_emphasis("**외적(outer product)**에서 *a*");
        assert_eq!(read.formed, 1);
        assert_eq!(read.stray, vec![0..2, 19..21]);
        // An identifier's underscore and a spaced star are text, not marks.
        let read = commonmark_emphasis("min_heap_size 는 2 * 3 입니다");
        assert_eq!((read.formed, read.stray.len()), (0, 0));
        // An escaped star is text too.
        assert!(commonmark_emphasis(r"\*외적\*에서").stray.is_empty());
    }
```

- [ ] **Step 2: 테스트가 실패하는지 확인**

Run: `cargo test -p yeokja-translate --lib evaluator_format`
Expected: 컴파일 오류(`commonmark_emphasis` 없음). 이후 구현 전 임시로 함수만 두면 위 네 테스트가 실패.

- [ ] **Step 3: 의존성 추가** (`crates/translate/Cargo.toml`의 `[dependencies]`)

```toml
pulldown-cmark = { version = "0.12", default-features = false }
```

- [ ] **Step 4: 구현** (`evaluator_format.rs`)

`evaluate`에서 규칙을 인자로 넘긴다:

```rust
        let mut issues = Self::markup_issues(context, context.markup == Markup::Markdown);
```

`markup_issues`의 시그니처와 MkDocs 재귀를 바꾼다:

```rust
    /// `commonmark` says whether emphasis follows CommonMark's delimiter
    /// rules. MkDocs checks its masked text as Markdown, but Python-Markdown
    /// closes `*` wherever it finds one.
    fn markup_issues(context: &EvaluationContext, commonmark: bool) -> Vec<EvaluationIssue> {
        ...
            issues.extend(Self::markup_issues(&masked, false));
```

표시 연속 개수 검사에서 강조 결과를 기억한다(기존 `for (name, in_source, in_translation) in checks` 루프):

```rust
            for (name, in_source, in_translation) in checks {
                if in_source != in_translation {
                    told_about_emphasis |= name == "Emphasis marker runs";
                    issues.push(...기존 그대로...);
                }
            }
```

(`let mut told_about_emphasis = false;`를 `let mut issues = Vec::new();` 바로 아래에 둔다.)

제약 쌍 루프에서 CommonMark일 때 `_` 쌍 수 대조를 건너뛴다(새 검사가 `*`·`_`를 함께 센다):

```rust
        for &(mark, unconstrained) in constrained_marks(context.markup) {
            if commonmark && mark == '_' {
                continue;
            }
```

닫지 못하는 쌍 검사(`unclosable_pairs`) 블록에서 보고했으면 `told_about_emphasis = true;`를
넣는다. 그 블록 바로 뒤, 줄 머리 검사 앞에 새 검사를 둔다:

```rust
        // CommonMark closes `*` against a Korean particle — unless punctuation
        // sits on its inner side: then only a space or punctuation after it
        // lets it close, and `**외적(outer product)**에서` prints its marks.
        // The pairs that form are counted by the parser mdBook itself uses.
        if commonmark {
            let source = commonmark_emphasis(&visible_source);
            let translation = commonmark_emphasis(&visible_translation);
            if source.stray.is_empty() && source.formed != translation.formed {
                let blocked = blocked_emphasis(&visible_translation, &translation.stray);
                if !blocked.is_empty() {
                    issues.push(EvaluationIssue {
                        severity: IssueSeverity::Error,
                        kind: IssueKind::FormatLost,
                        message: format!(
                            "{} never closes: CommonMark closes * or ** right after \
                             punctuation — ), ], `, \" — only when a space or punctuation \
                             follows it, so a particle straight after it prints the marks as \
                             themselves. End the emphasis on a letter: keep a gloss, a link or \
                             quotation marks outside it — **외적**(outer product)에서, \
                             [**Fetch**](url)는, \"*권한*\"은 — and when the term itself ends in \
                             code or a symbol, take the particle inside: *`impl Trait`의*. An \
                             opening mark right before punctuation likewise needs a space or \
                             punctuation in front of it.",
                            blocked.join(", "),
                        ),
                    });
                } else if !told_about_emphasis {
                    issues.push(EvaluationIssue {
                        severity: IssueSeverity::Error,
                        kind: IssueKind::FormatLost,
                        message: format!(
                            "Emphasis pairs do not line up: the source forms {}, the \
                             translation {}. CommonMark leaves a * or _ run unpaired when \
                             punctuation sits on its inner side and a letter on its outer \
                             side; end each emphasis on a letter and start it after a space: \
                             **외적**(outer product)에서.",
                            source.formed, translation.formed,
                        ),
                    });
                }
            }
        }
```

새 함수들(`markdown_emphasis_pairs` 아래):

```rust
/// Emphasis the way CommonMark reads it: the pairs that form, and every `*`
/// or `_` run left printed as itself where a mark could open or close.
///
/// mdBook parses with pulldown-cmark, and markdown-it (MyST), micromark (MDX)
/// and pandoc's gfm reader implement the same delimiter algorithm, so asking
/// pulldown-cmark is asking the renderer. Simulating it the way `pair_up`
/// does for AsciiDoc would mean reproducing the delimiter stack, the rule of
/// three, and the precedence of code spans and links.
struct CommonMarkEmphasis {
    formed: usize,
    /// Char ranges of the runs left as text.
    stray: Vec<std::ops::Range<usize>>,
}

fn commonmark_emphasis(text: &str) -> CommonMarkEmphasis {
    use pulldown_cmark::{Event, Options, Parser, Tag};
    let mut formed = 0;
    let mut literal = vec![false; text.len()];
    for (event, range) in Parser::new_ext(text, Options::empty()).into_offset_iter() {
        match event {
            Event::Start(Tag::Emphasis | Tag::Strong) => formed += 1,
            Event::Text(_) => literal[range].fill(true),
            _ => {}
        }
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut stray = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (byte, mark) = chars[i];
        if !matches!(mark, '*' | '_') || !literal[byte] || (i > 0 && chars[i - 1].1 == '\\') {
            i += 1;
            continue;
        }
        let start = i;
        while chars.get(i).is_some_and(|&(byte, c)| c == mark && literal[byte]) {
            i += 1;
        }
        let before = start.checked_sub(1).map(|at| chars[at].1);
        let after = chars.get(i).map(|&(_, c)| c);
        // `min_heap_size` flanks on both sides and still opens nothing.
        let intraword = mark == '_'
            && before.is_some_and(char::is_alphanumeric)
            && after.is_some_and(char::is_alphanumeric);
        if (left_flanking(before, after) || left_flanking(after, before)) && !intraword {
            stray.push(start..i);
        }
    }
    CommonMarkEmphasis { formed, stray }
}

/// Whether a delimiter run between `before` and `after` is left-flanking and
/// so may open; with the arguments swapped, whether it is right-flanking and
/// may close. The edge of the text counts as whitespace.
fn left_flanking(before: Option<char>, after: Option<char>) -> bool {
    let space = |c: Option<char>| c.is_none_or(char::is_whitespace);
    let punctuation = |c: Option<char>| c.is_some_and(commonmark_punctuation);
    !space(after) && (!punctuation(after) || space(before) || punctuation(before))
}

/// CommonMark's punctuation: ASCII punctuation, and — since 0.31 — any
/// Unicode punctuation or symbol, taken here as whatever non-ASCII character
/// is not a letter, digit or space.
fn commonmark_punctuation(c: char) -> bool {
    c.is_ascii_punctuation() || (!c.is_ascii() && !c.is_alphanumeric() && !c.is_whitespace())
}

/// The stray runs of `text` that punctuation blocks — a closing mark between
/// punctuation and a letter, or an opening one between a letter and
/// punctuation — each quoted with the run it was meant to pair with and the
/// word the particle belongs to.
fn blocked_emphasis(text: &str, stray: &[std::ops::Range<usize>]) -> Vec<String> {
    let chars = chars(text);
    let letter = |at: usize| chars.get(at).is_some_and(|c| c.is_alphanumeric());
    let punctuation = |at: usize| chars.get(at).is_some_and(|c| commonmark_punctuation(*c));
    let same = |a: &std::ops::Range<usize>, b: &std::ops::Range<usize>| {
        a.len() == b.len() && chars[a.start] == chars[b.start]
    };
    let mut shown = Vec::new();
    for (k, run) in stray.iter().enumerate() {
        let (from, to) = if run.start > 0 && punctuation(run.start - 1) && letter(run.end) {
            let from = stray[..k].iter().rev().find(|open| same(open, run)).map_or(run.start, |open| open.start);
            let mut to = run.end;
            while letter(to) {
                to += 1;
            }
            (from, to)
        } else if run.start > 0 && letter(run.start - 1) && punctuation(run.end) {
            let mut from = run.start;
            while from > 0 && letter(from - 1) {
                from -= 1;
            }
            let to = stray[k + 1..].iter().find(|close| same(close, run)).map_or(run.end, |close| close.end);
            (from, to)
        } else {
            continue;
        };
        let text: String = chars[from..to].iter().collect();
        if !shown.contains(&text) {
            shown.push(text);
        }
    }
    shown
}
```

- [ ] **Step 5: 테스트 통과 확인**

Run: `cargo test -p yeokja-translate --lib evaluator_format`
Expected: 새 테스트 포함 전부 PASS. 기존 Markdown 테스트(`a_markdown_underscore_still_has_to_close`,
`markdown_allows_underscore_emphasis_to_become_stars_before_a_particle`)도 PASS.

- [ ] **Step 6: 커밋하지 않고 Task 2로** (Task 1·2를 한 커밋으로 묶는다)

### Task 2: 괄호 풀이 보정, 파이프라인 배선, 프롬프트

**Files:**
- Modify: `crates/translate/src/evaluator_format.rs` (`repair_markdown_emphasis` + 테스트)
- Modify: `crates/translate/src/pipeline.rs:179-186` (배선) + 테스트
- Modify: `crates/translate/src/prompt.rs:108-112` (Markdown 규칙) + 테스트
- Modify: `projects/webassembly-component-docs/yeokja.toml` (커스텀 템플릿 한 줄)
- Modify: `docs/superpowers/specs/2026-09-21-translation-robustness-design.md`, 이 계획 파일(커밋에 포함)

**Interfaces:**
- Consumes: `commonmark_emphasis`, `without_reference_labels`, `chars` (Task 1)
- Produces: `pub fn repair_markdown_emphasis(source: &str, translation: &str) -> String`
  (Task 4의 일회성 이행 도구가 쓰므로 `pub`)

- [ ] **Step 1: 실패하는 테스트 작성** (`evaluator_format.rs` 테스트)

```rust
    #[test]
    fn markdown_repair_moves_a_closing_mark_before_a_gloss() {
        let source = "The **goal** is to prove and the **clause** is known.";
        assert_eq!(
            repair_markdown_emphasis(source, "**목표(goal)**란 증명할 것이며 **조항(clause)**이란 아는 것입니다."),
            "**목표**(goal)란 증명할 것이며 **조항**(clause)이란 아는 것입니다.",
        );
        assert_eq!(
            repair_markdown_emphasis("an *issue (kind)*.", "*이슈 (종류)*로 봅니다."),
            "*이슈* (종류)로 봅니다.",
        );
    }

    #[test]
    fn markdown_repair_leaves_what_it_cannot_move_alone() {
        for (source, translation) in [
            // A link's destination is no gloss.
            ("**[Fetch](./fetch-engine.md)** reads.", "**[Fetch](./fetch-engine.md)**는 읽습니다."),
            // Code at the end has no gloss to move.
            ("Short for _`impl Trait` in assoc_.", "*연관 타입의 `impl Trait`*의 줄임말입니다."),
            // Math is no gloss.
            ("holds *tensor \\\\(T\\\\)*.", "*텐서 \\\\(T\\\\)*를 보유합니다."),
            // Emphasis the source does not have is not repaired into existence.
            ("the outer product", "**외적(outer product)**에서"),
            // A source that leaves a mark open is no measure.
            ("_Applied to `Span` fields.", "*필드(fields)*에 적용됩니다."),
        ] {
            assert_eq!(repair_markdown_emphasis(source, translation), translation);
        }
    }
```

`pipeline.rs` 테스트(`mod tests` 끝):

```rust
    #[tokio::test]
    async fn markdown_emphasis_is_repaired_before_evaluation() {
        let provider = MockProvider::new(vec![
            [(1, "선형대수학의 **외적(outer product)**에서 유래합니다.".to_string())].into(),
        ]);
        let format = crate::evaluator_format::FormatEvaluator;
        let evaluators: Vec<&dyn TranslationEvaluator> = vec![&format];
        let source = "It comes from the **outer product** of linear algebra.";
        let request = TranslateRequest {
            segments: vec![(1, source.to_string())],
            block_context: source.to_string(),
            glossary: HashMap::new(),
            source_lang: "en".to_string(),
            target_lang: "ko".to_string(),
            markup: Markup::Markdown,
            feedback: None,
            prompt_template: None,
            paragraphs: HashMap::new(),
        };
        let results = translate_with_evaluation(
            &provider, &evaluators, request, &HashMap::new(), "en", "ko", Markup::Markdown, 3,
        )
        .await
        .unwrap();
        assert_eq!(results[&1].translation, "선형대수학의 **외적**(outer product)에서 유래합니다.");
        assert_eq!(results[&1].attempts, 1);
    }
```

`prompt.rs`의 기존 규칙 테스트(`assert!(markdown.contains("*arity*는"));` 근처)에 추가:

```rust
        assert!(markdown.contains("**외적**(outer product)에서"));
        assert!(!mkdocs.contains("**외적**(outer product)에서"));
```

(해당 테스트에서 MkDocs 프롬프트를 담는 변수 이름을 확인해 맞춘다. 없으면
`let mkdocs = closing_rule(yeokja_core::parser::Markup::MkDocs);`를 더한다.)

- [ ] **Step 2: 실패 확인**

Run: `cargo test -p yeokja-translate --lib`
Expected: `repair_markdown_emphasis` 없음으로 컴파일 실패.

- [ ] **Step 3: 구현**

`evaluator_format.rs`(`repair_rst_boundaries` 위):

```rust
/// Move the closing mark of an emphasis that ends on a parenthesised gloss
/// in front of the gloss: `**외적(outer product)**에서` →
/// `**외적**(outer product)에서`.
///
/// CommonMark does not close `**` between `)` and a particle (see
/// `commonmark_emphasis`), and Korean glosses a term in parentheses straight
/// after it, so the model writes that shape again and again. In front of the
/// `(` the mark follows the term and closes. The repair is kept only when it
/// forms more pairs without forming more than the source does; everything
/// else is left for the evaluator to report. Also run once over stored
/// translations, hence `pub`.
pub fn repair_markdown_emphasis(source: &str, translation: &str) -> String {
    let wanted = commonmark_emphasis(&without_reference_labels(source));
    if !wanted.stray.is_empty() {
        return translation.to_string();
    }
    let chars = chars(translation);
    let mut moves: Vec<(std::ops::Range<usize>, usize)> = Vec::new();
    for run in commonmark_emphasis(translation).stray {
        if run.start == 0
            || chars[run.start - 1] != ')'
            || !chars.get(run.end).is_some_and(|c| c.is_alphanumeric())
        {
            continue;
        }
        let Some(open) = chars[..run.start - 1].iter().rposition(|c| *c == '(') else {
            continue;
        };
        if chars[open + 1..run.start - 1]
            .iter()
            .any(|c| matches!(c, '*' | '_' | '`' | '(' | ')' | '\\'))
        {
            continue;
        }
        let term_end = if open > 0 && chars[open - 1] == ' ' { open - 1 } else { open };
        // A link destination or escaped math is no gloss, and nothing before
        // the `(` means there is no term to end the emphasis on.
        if term_end == 0
            || chars[term_end - 1].is_whitespace()
            || matches!(chars[term_end - 1], ']' | '\\' | '*' | '_')
        {
            continue;
        }
        moves.push((run, term_end));
    }
    if moves.is_empty() {
        return translation.to_string();
    }
    let mut repaired = String::with_capacity(translation.len());
    for (at, c) in chars.iter().enumerate() {
        for (run, to) in &moves {
            if *to == at {
                repaired.extend(&chars[run.clone()]);
            }
        }
        if !moves.iter().any(|(run, _)| run.contains(&at)) {
            repaired.push(*c);
        }
    }
    let before = commonmark_emphasis(&without_reference_labels(translation)).formed;
    let after = commonmark_emphasis(&without_reference_labels(&repaired)).formed;
    if after > before && after <= wanted.formed {
        repaired
    } else {
        translation.to_string()
    }
}
```

`pipeline.rs` 배선(`let translation = match markup {` 안):

```rust
                Markup::Markdown => {
                    crate::evaluator_format::repair_markdown_emphasis(source, translation)
                }
```

`prompt.rs`의 `Markup::Markdown` 규칙:

```rust
        Markup::Markdown => {
            "A closing _ that a letter follows does not close the pair. When the translation \
             puts a suffix straight after an italicised term, use * instead: _arity_ → \
             *arity*는. A closing * or ** right after punctuation — ), ], `, \" — does not \
             close when a letter follows it either, so end emphasis on a letter: \
             **외적**(outer product)에서, not **외적(outer product)**에서; \
             [**Fetch**](url)는; *`impl Trait`의*.\n"
        }
```

`projects/webassembly-component-docs/yeokja.toml` 템플릿의
`Use *italic* instead of _italic_ when Korean particles follow emphasis.` 다음 줄에:

```
End emphasis on a letter before a particle: **외적**(outer product)에서, not **외적(outer product)**에서.
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test --workspace`
Expected: 전부 PASS.

- [ ] **Step 5: 커밋**

```bash
git add crates/translate docs/superpowers Cargo.lock projects/webassembly-component-docs/yeokja.toml
git commit -m "[*] feat: 문장 부호 뒤에서 닫히지 않는 CommonMark 강조를 잡고 괄호 풀이를 보정하기"
```

### Task 3: AsciiDoc 조사 앞 제약 쌍 보정

**Files:**
- Modify: `crates/translate/src/evaluator_format.rs` (`Pairing`, `pair_up`, 새 함수, 테스트)
- Modify: `crates/translate/src/pipeline.rs` (배선) + 테스트

**Interfaces:**
- Consumes: `pair_up`, `constrained_marks`, `chars`
- Produces: `pub(crate) fn repair_asciidoc_boundaries(source: &str, translation: &str) -> String`,
  `Pairing.blocked_closers: Vec<std::ops::Range<usize>>`(`unclosable`과 같은 순서로, 각 쌍이 처음
  만난 닫는 후보 연속의 char 범위)

- [ ] **Step 1: 실패하는 테스트 작성**

```rust
    #[test]
    fn asciidoc_repair_doubles_a_pair_a_particle_keeps_open() {
        assert_eq!(
            repair_asciidoc_boundaries(
                "The `erlc` compiler and *bold* and _it_ work.",
                "`erlc`의 컴파일러와 *굵게*를, _기울임_은 동작합니다.",
            ),
            "``erlc``의 컴파일러와 **굵게**를, __기울임__은 동작합니다.",
        );
    }

    #[test]
    fn asciidoc_repair_finishes_a_half_doubled_pair() {
        assert_eq!(
            repair_asciidoc_boundaries("the `Atom` chunk", "`Atom``이라는 청크"),
            "``Atom``이라는 청크",
        );
    }

    #[test]
    fn asciidoc_repair_spaces_a_passthrough_instead_of_doubling() {
        assert_eq!(
            repair_asciidoc_boundaries(
                "Write `+receive [] -> ok end+` here.",
                "`+receive [] -> ok end+`를 씁니다.",
            ),
            "`+receive [] -> ok end+` 를 씁니다.",
        );
    }

    #[test]
    fn asciidoc_repair_leaves_a_source_that_cannot_close_alone() {
        let translation = "`{...}``의 모양";
        assert_eq!(repair_asciidoc_boundaries("the `{...}`` shape", translation), translation);
        let fine = "``erlc``의 컴파일러";
        assert_eq!(repair_asciidoc_boundaries("The `erlc` compiler", fine), fine);
    }
```

`pipeline.rs` 테스트:

```rust
    #[tokio::test]
    async fn asciidoc_pairs_are_doubled_before_evaluation() {
        let provider = MockProvider::new(vec![[(1, "`erlc`의 출력입니다.".to_string())].into()]);
        let format = crate::evaluator_format::FormatEvaluator;
        let evaluators: Vec<&dyn TranslationEvaluator> = vec![&format];
        let source = "The output of `erlc`.";
        let request = TranslateRequest {
            segments: vec![(1, source.to_string())],
            block_context: source.to_string(),
            glossary: HashMap::new(),
            source_lang: "en".to_string(),
            target_lang: "ko".to_string(),
            markup: Markup::Asciidoc,
            feedback: None,
            prompt_template: None,
            paragraphs: HashMap::new(),
        };
        let results = translate_with_evaluation(
            &provider, &evaluators, request, &HashMap::new(), "en", "ko", Markup::Asciidoc, 3,
        )
        .await
        .unwrap();
        assert_eq!(results[&1].translation, "``erlc``의 출력입니다.");
        assert_eq!(results[&1].attempts, 1);
    }
```

- [ ] **Step 2: 실패 확인**

Run: `cargo test -p yeokja-translate --lib`
Expected: `repair_asciidoc_boundaries` 없음으로 컴파일 실패.

- [ ] **Step 3: 구현**

`Pairing`에 필드 추가:

```rust
struct Pairing {
    formed: usize,
    unclosable: Vec<std::ops::Range<usize>>,
    /// For each unclosable pair, in the same order, the run of marks it first
    /// met and could not close on.
    blocked_closers: Vec<std::ops::Range<usize>>,
}
```

`pair_up`에서 처음 만난 후보를 범위로 기억한다:

```rust
            first_candidate.get_or_insert(close..close + close_len);
        ...
        match (closes_at, first_candidate) {
            (Some(end), _) => {
                pairing.formed += 1;
                i = end;
            }
            (None, None) => break,
            (None, Some(candidate)) => {
                pairing.unclosable.push(i..(candidate.end + 1).min(chars.len()));
                i = candidate.end;
                pairing.blocked_closers.push(candidate);
            }
        }
```

새 함수(`repair_markdown_emphasis` 아래):

```rust
/// Double the marks of an AsciiDoc pair a Korean particle keeps from
/// closing: `` `erlc`의 `` → ``` ``erlc``의 ```.
///
/// The evaluator names the doubled form, and the model still wrote the single
/// one four retries running (thebeambook `compiler.asciidoc`); like
/// `repair_rst_boundaries`, this is typography, not translation. A pair
/// doubled at its closing end only gets its opening end doubled too. A
/// passthrough span `` `+x+` `` gets a space after it instead — told to double,
/// the model dropped the pluses. A mark the source itself leaves open is left
/// alone, and a repair is kept only when the mark then forms more pairs and
/// leaves none open.
pub(crate) fn repair_asciidoc_boundaries(source: &str, translation: &str) -> String {
    let mut repaired = translation.to_string();
    for &(mark, _) in constrained_marks(Markup::Asciidoc) {
        if !pair_up(&chars(source), mark).unclosable.is_empty() {
            continue;
        }
        let text = chars(&repaired);
        let before = pair_up(&text, mark);
        let mut doubled = std::collections::BTreeSet::new();
        let mut spaced = std::collections::BTreeSet::new();
        for (span, close) in before.unclosable.iter().zip(&before.blocked_closers) {
            let open_len = text[span.start..].iter().take_while(|c| **c == mark).count();
            if open_len != 1 || text[close.start - 1].is_whitespace() {
                continue;
            }
            let content = &text[span.start + 1..close.start];
            let passthrough = mark == '`'
                && content.len() > 1
                && content.first() == Some(&'+')
                && content.last() == Some(&'+');
            match close.len() {
                1 if passthrough => {
                    spaced.insert(close.end);
                }
                1 => {
                    doubled.insert(span.start);
                    doubled.insert(close.start);
                }
                2 if !passthrough => {
                    doubled.insert(span.start);
                }
                _ => {}
            }
        }
        if doubled.is_empty() && spaced.is_empty() {
            continue;
        }
        let mut candidate = String::with_capacity(repaired.len() + doubled.len() + spaced.len());
        for (at, c) in text.iter().enumerate() {
            if doubled.contains(&at) {
                candidate.push(mark);
            }
            if spaced.contains(&at) {
                candidate.push(' ');
            }
            candidate.push(*c);
        }
        let after = pair_up(&chars(&candidate), mark);
        if after.unclosable.is_empty() && after.formed > before.formed {
            repaired = candidate;
        }
    }
    repaired
}
```

`pipeline.rs` 배선:

```rust
                Markup::Asciidoc => {
                    crate::evaluator_format::repair_asciidoc_boundaries(source, translation)
                }
```

- [ ] **Step 4: 통과 확인**

Run: `cargo test --workspace`
Expected: 전부 PASS(기존 `a_half_doubled_pair_closes_neither_way` 등 평가기 테스트 포함).

- [ ] **Step 5: 커밋**

```bash
git add crates/translate
git commit -m "[*] feat: AsciiDoc에서 조사 앞 한 겹 쌍을 두 겹으로 보정하기"
```

### Task 4: 저장된 번역 감사와 수선

**Files:**
- Modify: `projects/{rustc-dev-guide,rust-forge,furiosa-opt,putting-the-you-in-cpu,webassembly-component-docs,learn-fpga,zero-to-nix,raytracing}/state/**/*.yeokja.json`
- Create(스크래치패드, 커밋하지 않음): 일회성 이행 도구 `emfix`

- [ ] **Step 1: 새 바이너리로 전 프로젝트 감사**

Run: `cargo build --release` 후 각 프로젝트에서 `evaluate --mechanical-only .`
Expected: 이전(63건) 대비 늘어난 항목이 spec 7절의 88건 + raytracing 1건과 정확히 겹친다
(rustc-dev-guide `diagnostic-structs.md`는 원문이 열린 표시를 남겨 대조하지 않으므로 나오지 않음).
다른 항목이 새로 나오면 오탐인지 확인하고 규칙이나 spec에 반영한다.

- [ ] **Step 2: 괄호 풀이 꼴을 제자리에서 보정**

스크래치패드에 `yeokja-translate`를 경로 의존성으로 쓰는 작은 바이너리를 만들어, 대상 프로젝트
state의 모든 번역에 `repair_markdown_emphasis(source, translation)`을 적용하고 바뀐 것만 쓴다
(JSON은 `serde_json::Value`로 읽고 `serde_json::to_string_pretty`가 아닌 기존 서식을 확인해 같은
서식으로 쓴다 — 서식이 다르면 파이썬 `json.dump(..., ensure_ascii=False, indent=?)`로 맞춘다).
바뀐 세그먼트 목록을 기록한다.

- [ ] **Step 3: 남은 세그먼트 재번역**

다시 감사해 남은 새 검사 항목(코드·링크·인용·기호 꼴)과 rust-forge `compiler/reviews.md`
section:2/block:9/seg:1, raytracing `RayTracingTheNextWeek.html` section:12/block:14/seg:1의
번역을 `null`로 바꾸고 파일 단위로 `translate`한다.

- [ ] **Step 4: 재감사**

Expected: 새 검사 항목 0건, 나머지 항목은 이전 63건과 같다(재번역이 새 항목을 만들면 확인해 처리).
`yeokja status --check .`가 대상 프로젝트에서 통과한다.

- [ ] **Step 5: 빌드 확인**

rustc-dev-guide, furiosa-opt: `rebuild-translations.sh` → `nix develop path:../../nix#<name> -c
../../target/release/yeokja build html` → 빌드된 HTML의 code·pre 밖 본문에 `**`·`*`가 글자로 남은
곳이 없는지(이전: rustc-dev-guide 34곳, furiosa-opt 3곳) 확인한다.

- [ ] **Step 6: 프로젝트별 커밋**

```bash
git add projects/<name>/state
git commit -m "[<name>] fix: 문장 부호 뒤에서 닫히지 않는 강조 바로잡기"
```

### Task 5: spec에 결과 기록

- [ ] spec 7절 "기존 번역"에 실제 수치(제자리 보정 건수, 재번역 건수, 재감사 결과)를 적고 커밋한다.

```bash
git add docs/superpowers/specs/2026-09-21-translation-robustness-design.md
git commit -m "[*] docs: CommonMark 강조 수선 결과 기록"
```
