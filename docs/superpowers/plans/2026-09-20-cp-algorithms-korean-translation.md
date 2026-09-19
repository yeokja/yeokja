# cp-algorithms 한국어 번역 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** cp-algorithms(MkDocs Material, CC BY-SA 4.0)를 새 `mkdocs` 파서로 yeokja 번역하고, 영어 제목 ID가 보존된 한국어 사이트를 Pages 배포 배선까지 갖춘다.

**Architecture:** `crates/parser-mkdocs`가 원문과 길이가 같은 그림자 사본에서 수식·admonition·front matter·Jinja를 CommonMark 구조로 바꿔 `parser-markdown-dialect`에 넘긴다. 평가기는 `Markup::MkDocs`에서 수식 보존과 Jinja 구분자를 검사한 뒤, 수식을 가린 텍스트로 기존 Markdown 검사를 재사용한다. 빌드는 영어 원본을 먼저 빌드해 제목 ID를 뽑고, 한국어 빌드에서 MkDocs 훅이 그 ID를 제목에 입힌다.

**Tech Stack:** Rust(pulldown-cmark 기반 dialect), yeokja CLI, Python 3(MkDocs 1.6 + mkdocs-material 9.7 + pymdown-extensions, Python-Markdown), Nix devShell, GitHub Actions Pages.

**Spec:** `docs/superpowers/specs/2026-09-20-cp-algorithms-korean-translation-design.md`

## Global Constraints

- 원문 서브모듈: `projects/cp-algorithms/upstream`, URL `https://github.com/cp-algorithms/cp-algorithms.git`, `shallow = true`, 구현 시점 main HEAD에 고정. `upstream/`은 수정하지 않는다.
- 번역 범위: `upstream/src/**/*.md`(단 `contrib.md`, `code_of_conduct.md`, `preview.md` 제외) + `upstream/README.md`(홈 본문, `src/index_body`로 출력).
- 파서 이름 `mkdocs`, 크레이트 `yeokja-parser-mkdocs`(`crates/parser-mkdocs`), 타입 `MkdocsParser`, markup `Markup::MkDocs`.
- provider: `type = "claude_code"`, `model = "claude-sonnet-5"`, 커스텀 `prompt_template` 없음.
- 수식(`$..$`, `$$..$$`, `\(..\)`, `\[..\]`, `\begin{..}..\end{..}`)은 바이트 그대로 보존한다. `navigation.md`의 링크 항목은 링크 하나로만 남아야 한다(literate-nav).
- 한국어 사이트: `mkdocs build --strict` 성공, 앵커 훅 개수 불일치 0, `check_links.py` `New: 0`.
- 라이선스: CC BY-SA 4.0 개작물. 저작자 "cp-algorithms contributors", 라이선스 링크, 번역(수정) 고지, 비공식 고지를 유지한다.
- README 출처 표기는 AGENTS.md 규칙(yeokja 링크 `https://github.com/yeokja/yeokja`, 실제 모델, 학습 비허용 문장)을 따른다.
- `ko/`, `build/`, `dist/`, `__pycache__/`는 커밋하지 않는다.
- 번역 provider 동시 사용 제한: 번역(`yeokja translate`)을 시작하기 전에 `pgrep -fl 'yeokja translate'`로 다른 번역 프로세스가 없는지 확인하고, 있으면 끝날 때까지 기다린다.
- 커밋 메시지: `[cp-algorithms] <type>: <한국어 요약>`(파서·평가기처럼 여러 프로젝트에 걸친 변경은 `[*]`), 마지막 줄 `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`. push하지 않는다.
- `$Y`는 저장소 루트의 `target/release/yeokja`다. 파서·평가기를 바꾼 뒤에는 `cargo build --release -p yeokja-cli`로 다시 빌드한다(worktree라면 worktree 안에서 빌드한다).

---

### Task 1: dialect 공통 수정 — 이스케이프 백슬래시와 attr_list 제목 ID

**Files:**
- Modify: `crates/parser-markdown-dialect/src/lib.rs`(`extend_run` 124-136, `strip_heading_id` 91-106)
- Test: `crates/parser-markdown/src/lib.rs`(tests 모듈), `crates/parser-mdx/src/lib.rs`(기존 테스트 통과 확인)

**Interfaces:**
- Produces: 문단·셀이 이스케이프 문자로 시작해도 세그먼트에 `\`가 포함된다. `preserve_heading_ids = true`일 때 `{ #id }`, `{: #id .cls }`, `{#id}`, `\{#id}`와 그 앞의 ATX 닫는 `#`들이 제목 세그먼트 밖에 남는다.

- [ ] **Step 1: 실패하는 테스트 작성(`crates/parser-markdown/src/lib.rs` tests)**

```rust
    #[test]
    fn escaped_bracket_at_paragraph_start_stays_in_segment() {
        let parser = MarkdownParser;
        let source = "\\[ x^2 \\] is a formula.\n";
        let doc = parser.parse(source);
        let segments = doc.translatable_segments();
        assert_eq!(segments[0].source, "\\[ x^2 \\] is a formula.");
        let mut translations = TranslationMap::new();
        translations.insert(segments[0].id.clone(), "\\[ x^2 \\]는 식입니다.".to_string());
        assert_eq!(parser.reconstruct(&doc, &translations), "\\[ x^2 \\]는 식입니다.\n");
    }

    #[test]
    fn escaped_backslash_before_text_is_not_pulled_in_twice() {
        let parser = MarkdownParser;
        let doc = parser.parse("\\\\ path and more.\n");
        assert_eq!(doc.translatable_segments()[0].source, "\\\\ path and more.");
    }
```

Run: `cargo test -p yeokja-parser-markdown escaped_ -- --nocapture`
Expected: 첫 테스트가 FAIL(세그먼트가 `[ x^2 \] is a formula.`).

- [ ] **Step 2: `extend_run` 수정**

`crates/parser-markdown-dialect/src/lib.rs`의 `extend_run`에서 `None => self.run = Some(range),`를 다음으로 바꾸고, 파일 끝(테스트 모듈 앞)에 도우미를 추가한다.

```rust
            None => {
                let mut range = range;
                if starts_after_escape(self.source.as_bytes(), range.start) {
                    range.start -= 1;
                }
                self.run = Some(range);
            }
```

```rust
/// pulldown-cmark reports an escaped character (`\[`) without its backslash.
/// A run that starts there must take the backslash along, or the splice
/// leaves it in front of the translation.
fn starts_after_escape(bytes: &[u8], at: usize) -> bool {
    if at == 0 || bytes[at - 1] != b'\\' || !bytes.get(at).is_some_and(u8::is_ascii_punctuation) {
        return false;
    }
    let backslashes = bytes[..at].iter().rev().take_while(|&&b| b == b'\\').count();
    backslashes % 2 == 1
}
```

Run: `cargo test -p yeokja-parser-markdown`
Expected: PASS(두 새 테스트 포함).

- [ ] **Step 3: attr_list 제목 ID 테스트 작성(`crates/parser-markdown-dialect/src/lib.rs`에 tests 모듈 추가)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn heading_sources(source: &str) -> Vec<String> {
        parse_with(source, source, true, Vec::new())
            .translatable_segments()
            .iter()
            .map(|s| s.source.clone())
            .collect()
    }

    #[test]
    fn attr_list_ids_stay_outside_heading_span() {
        assert_eq!(heading_sources("## Implementation { #implementation }\n"), ["Implementation"]);
        assert_eq!(heading_sources("## Title {: #tid .cls }\n"), ["Title"]);
        assert_eq!(heading_sources("## Title {#tid}\n"), ["Title"]);
        assert_eq!(heading_sources("## Title \\{#tid}\n"), ["Title"]);
    }

    #[test]
    fn closing_hashes_before_attr_list_stay_outside() {
        assert_eq!(heading_sources("## Implementation ### { #implementation}\n"), ["Implementation"]);
    }

    #[test]
    fn braces_without_an_id_are_title_text() {
        assert_eq!(heading_sources("## Sets {a, b}\n"), ["Sets {a, b}"]);
    }
}
```

Run: `cargo test -p yeokja-parser-markdown-dialect`
Expected: FAIL(`{ #implementation }`, `{: #tid .cls }`, 닫는 `###`가 제목에 남음).

- [ ] **Step 4: `strip_heading_id` 일반화**

```rust
    /// Shrink a heading run so an explicit id stays outside the span, verbatim
    /// after the translated title: MDX `\{#id}`/`{#id}` and Python-Markdown
    /// attr_list `{ #id }`/`{: #id .class }`, together with an ATX closing
    /// sequence written before it (`Title ### {#id}`).
    fn strip_heading_id(&self, span: Range<usize>) -> Range<usize> {
        let raw = &self.source[span.clone()];
        let trimmed = raw.trim_end();
        let Some(open) = trimmed.strip_suffix('}').and_then(|s| s.rfind('{')) else {
            return span;
        };
        let inner = trimmed[open + 1..trimmed.len() - 1].trim();
        let inner = inner.strip_prefix(':').unwrap_or(inner).trim();
        let has_id = inner.split_whitespace().any(|t| t.len() > 1 && t.starts_with('#'));
        if !has_id {
            return span;
        }
        let mut cut = open;
        if trimmed[..cut].ends_with('\\') {
            cut -= 1;
        }
        let mut title = trimmed[..cut].trim_end();
        let without_hashes = title.trim_end_matches('#');
        if without_hashes.len() < title.len() && without_hashes.ends_with(char::is_whitespace) {
            title = without_hashes.trim_end();
        }
        span.start..span.start + title.len()
    }
```

Run: `cargo test -p yeokja-parser-markdown-dialect -p yeokja-parser-markdown -p yeokja-parser-mdx -p yeokja-parser-myst`
Expected: 모두 PASS(기존 MDX `\{#id}` 테스트 포함).

- [ ] **Step 5: 기존 프로젝트 영향 조사**

이 수정으로 바뀌는 것은 이스케이프 문자로 시작하는 세그먼트와 `preserve_heading_ids`를 쓰는 MDX 파서의 제목뿐이다. 기존 Markdown 계열 프로젝트 전체에서 원문이 바뀐 세그먼트가 있는지 본다.

```bash
cargo build --release -p yeokja-cli
for p in $(grep -lE 'parser *= *"(markdown|mdx|myst)"' projects/*/yeokja.toml | xargs -n1 dirname); do
  for src in $(grep -A3 '^\[\[sources\]\]' $p/yeokja.toml | sed -nE 's/^path *= *"([^"]+)"/\1/p' | sort -u); do
    (cd $p && ../../target/release/yeokja status "$src" 2>/dev/null | tail -1 | sed "s|^|$p $src: |")
  done
done
```

기대: 모든 프로젝트가 수정 전과 같이 "번역 필요 0"을 보고한다. 번역이 필요한 세그먼트가 생긴 프로젝트가 있으면, 그 세그먼트는 지금까지 `\`가 번역문 앞에 남아 잘못 재구성되던 것이다. 해당 프로젝트에서 `$Y translate <source>`로 재번역하고 프로젝트별로 커밋한다(`[<project>] fix: 이스케이프로 시작하는 세그먼트 재번역`). 재번역 전에 Global Constraints의 번역 동시 사용 규칙을 따른다.

- [ ] **Step 6: 커밋**

```bash
git add crates/parser-markdown-dialect crates/parser-markdown
git commit -m "[*] fix: 이스케이프 문자로 시작하는 세그먼트와 attr_list 제목 ID 처리 수정" -m "pulldown-cmark가 이스케이프 문자의 범위에서 백슬래시를 빼서, 문단이 \\[나 \\(로 시작하면 번역문이 백슬래시 뒤에 붙던 문제를 고친다. preserve_heading_ids일 때 Python-Markdown attr_list 형식({ #id }, {: #id .cls })과 그 앞의 닫는 # 시퀀스도 제목 세그먼트 밖에 둔다." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 2: `parser-mkdocs` 크레이트 — 수식, front matter, Jinja, 공백 없는 제목

**Files:**
- Create: `crates/parser-mkdocs/Cargo.toml`, `crates/parser-mkdocs/src/lib.rs`
- Modify: `Cargo.toml`(workspace members), `crates/core/src/parser.rs`(`Markup::MkDocs`), `crates/parsers/Cargo.toml`, `crates/parsers/src/lib.rs`(`"mkdocs"` 등록)
- Modify: `Markup`을 모든 경우로 매칭하는 곳(`crates/translate/src/prompt.rs` `closing_rule`, `crates/translate/src/evaluator_format.rs` `constrained_marks`) — 이 태스크에서는 `Markup::MkDocs`를 `Markdown`과 같은 팔에 넣기만 한다(Task 4에서 규칙 추가).

**Interfaces:**
- Produces: `pub struct MkdocsParser;`(`DocumentParser`), `pub fn math_spans(text: &str) -> Vec<std::ops::Range<usize>>`, `pub fn mask_math(text: &str) -> String`. `Markup::MkDocs`. `parser_by_name("mkdocs")`.

- [ ] **Step 1: 크레이트 골격과 등록**

`crates/parser-mkdocs/Cargo.toml`:

```toml
[package]
name = "yeokja-parser-mkdocs"
version = "0.1.0"
edition = "2024"

[dependencies]
yeokja-core = { path = "../core" }
yeokja-parser-utils = { path = "../parser-utils" }
yeokja-parser-markdown-dialect = { path = "../parser-markdown-dialect" }
```

- 루트 `Cargo.toml` `members`에 `"crates/parser-mkdocs",`를 `"crates/parser-myst",` 다음에 추가한다.
- `crates/core/src/parser.rs`의 `enum Markup`에 추가한다.

```rust
    /// MkDocs (Python-Markdown + pymdown-extensions): Markdown whose spans may
    /// carry `$`/`$$`/`\(`/`\[` math that must survive translation verbatim.
    MkDocs,
```

- `crates/parsers/Cargo.toml`에 `yeokja-parser-mkdocs = { path = "../parser-mkdocs" }`, `parser_by_name`에 `"mkdocs" => Box::new(yeokja_parser_mkdocs::MkdocsParser),`(`"myst"` 다음)를 추가한다.
- `cargo build --workspace`가 가리키는 모든 비완전(non-exhaustive) `match`에 `Markup::MkDocs`를 `Markup::Markdown`과 같은 팔로 추가한다(`closing_rule`: `Markup::Markdown | Markup::MkDocs =>`, `constrained_marks`: `Markup::Markdown | Markup::MkDocs | Markup::Verso =>`).

- [ ] **Step 2: 실패하는 테스트 작성(`crates/parser-mkdocs/src/lib.rs` 하단)**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn sources(source: &str) -> Vec<String> {
        MkdocsParser.parse(source).translatable_segments().iter().map(|s| s.source.clone()).collect()
    }

    fn translate_all(source: &str, f: impl Fn(&str) -> String) -> String {
        let doc = MkdocsParser.parse(source);
        let mut map = TranslationMap::new();
        for seg in doc.translatable_segments() {
            map.insert(seg.id.clone(), f(&seg.source));
        }
        MkdocsParser.reconstruct(&doc, &map)
    }

    fn ko(s: &str) -> String {
        format!("가 {s}")
    }

    #[test]
    fn math_spans_cover_every_arithmatex_form() {
        let text = "a $x_1$ b $$\\sum_i\n i$$ c \\(y\\) d \\[z\\] e \\$5 and $ 6$ f `$code$`";
        let found: Vec<&str> = math_spans(text).into_iter().map(|r| &text[r]).collect();
        assert_eq!(found, ["$x_1$", "$$\\sum_i\n i$$", "\\(y\\)", "\\[z\\]"]);
    }

    #[test]
    fn begin_end_block_is_math() {
        let text = "Text.\n\n\\begin{align}\na &= b \\\\\nc &= d\n\\end{align}\n\nMore.\n";
        let found: Vec<&str> = math_spans(text).into_iter().map(|r| &text[r]).collect();
        assert_eq!(found, ["\\begin{align}\na &= b \\\\\nc &= d\n\\end{align}"]);
        assert_eq!(sources(text), ["Text.", "More."]);
    }

    #[test]
    fn inline_math_stays_in_the_segment_with_markdown_characters_inside() {
        assert_eq!(sources("Let $a_i * b_j$ be *the* product.\n"), ["Let $a_i * b_j$ be *the* product."]);
    }

    #[test]
    fn display_math_only_paragraph_is_not_offered() {
        let source = "Intro:\n\n$$\nd[v] = \\infty \\\\\n= 0\n$$\n\nOutro.\n";
        assert_eq!(sources(source), ["Intro:", "Outro."]);
        assert_eq!(translate_all(source, ko), "가 Intro:\n\n$$\nd[v] = \\infty \\\\\n= 0\n$$\n\n가 Outro.\n");
    }

    #[test]
    fn math_line_that_looks_like_a_list_or_setext_does_not_split_the_formula() {
        let source = "Consider\n$$\n0. x\n===\n$$\nthen stop.\n";
        let segs = sources(source);
        assert_eq!(segs.len(), 1);
        assert!(segs[0].contains("$$ 0. x === $$") || segs[0].contains("$$\n0. x"));
    }

    #[test]
    fn front_matter_with_trailing_tab_is_blanked_and_title_offered() {
        let source = "---\ntags:\n  - Translated\ntitle: Ternary Search\ne_maxx_link: ternary_search\n---\t\n# Ternary search\n\nBody.\n";
        assert_eq!(sources(source), ["Ternary Search", "Ternary search", "Body."]);
        assert_eq!(
            translate_all(source, ko),
            "---\ntags:\n  - Translated\ntitle: 가 Ternary Search\ne_maxx_link: ternary_search\n---\t\n# 가 Ternary search\n\n가 Body.\n"
        );
    }

    #[test]
    fn quoted_title_keeps_its_quotes() {
        let source = "---\ntitle: \"Main Page\"\n---\n\nBody.\n";
        assert_eq!(translate_all(source, ko), "---\ntitle: \"가 Main Page\"\n---\n\n가 Body.\n");
    }

    #[test]
    fn jinja_statement_line_is_kept_verbatim() {
        let source = "---\ntitle: Main Page\n---\n\n{% include 'index_body' %}\n";
        assert_eq!(sources(source), ["Main Page"]);
        assert_eq!(translate_all(source, ko), "---\ntitle: 가 Main Page\n---\n\n{% include 'index_body' %}\n");
    }

    #[test]
    fn atx_heading_without_space_is_a_heading() {
        assert_eq!(sources("Text.\n\n##Implementation\n\nMore.\n"), ["Text.", "Implementation", "More."]);
    }

    #[test]
    fn dollar_in_code_fence_is_not_math() {
        let source = "```cpp\nint $a = 1; // $b$\n```\n\nText $x$.\n";
        let found: Vec<&str> = math_spans(source).into_iter().map(|r| &source[r]).collect();
        assert_eq!(found, ["$x$"]);
    }

    #[test]
    fn mask_math_replaces_each_span_with_one_placeholder() {
        assert_eq!(mask_math("a $x_1$ b $$y$$"), "a ⟦M⟧ b ⟦M⟧");
    }
}
```

Run: `cargo test -p yeokja-parser-mkdocs`
Expected: 컴파일 실패(구현 없음).

- [ ] **Step 3: 구현(수식·front matter·Jinja·제목 — admonition은 Task 3)**

`crates/parser-mkdocs/src/lib.rs`:

```rust
//! MkDocs flavour of the Markdown parser: Python-Markdown with the
//! pymdown-extensions that MkDocs Material sites use.
//!
//! pulldown-cmark reads CommonMark, so this parser hands it a shadow of the
//! source — same length, same newlines — in which MkDocs-only syntax is
//! rewritten to CommonMark with the same structure. Spans still index the
//! real source, so reconstruction splices translations exactly as the plain
//! Markdown parser does.
//!
//! * Math (`$..$`, `$$..$$`, `\(..\)`, `\[..\]`, `\begin{..}..\end{..}`) is
//!   filled with `x`, so nothing inside it reads as Markdown. Inline math
//!   stays in the segment text; a block that is only math is not offered.
//! * An admonition, details or tab header becomes a list item (Task 3).
//! * YAML front matter is blanked (its closing `---` may carry trailing
//!   whitespace); only a `title:` value is offered.
//! * Jinja statement lines (`{% include ... %}`) are blanked.
//! * An ATX heading written without a space (`##Title`) gets one: its last
//!   `#` becomes a space, so the title span still starts at the title.

use std::ops::Range;
use yeokja_core::model::*;
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_markdown_dialect::{Extra, parse_with};
use yeokja_parser_utils::{resolve_reference_links, splice_reconstruct};

pub struct MkdocsParser;

impl DocumentParser for MkdocsParser {
    fn markup(&self) -> Markup {
        Markup::MkDocs
    }

    fn parse(&self, source: &str) -> Document {
        let layout = scan(source);
        let shadow = String::from_utf8(layout.shadow).expect("shadow edits are ASCII over whole characters");
        let mut doc = parse_with(&shadow, source, true, layout.extras);
        drop_math_only_blocks(&mut doc, &layout.math);
        doc
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        let translations = resolve_reference_links(document, translations);
        splice_reconstruct(document, &translations)
    }
}

struct Layout {
    shadow: Vec<u8>,
    extras: Vec<Extra>,
    math: Vec<Range<usize>>,
}

/// Byte ranges of each line, without its newline.
fn lines(source: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            out.push(start..i);
            start = i + 1;
        }
    }
    if start < source.len() {
        out.push(start..source.len());
    }
    out
}

fn fill(shadow: &mut [u8], range: Range<usize>, with: u8) {
    for b in &mut shadow[range] {
        if *b != b'\n' {
            *b = with;
        }
    }
}

/// A fence opener: its character and length.
fn fence_open(line: &str) -> Option<(u8, usize)> {
    let t = line.trim_start();
    for c in [b'`', b'~'] {
        let n = t.bytes().take_while(|&b| b == c).count();
        if n >= 3 && !(c == b'`' && t[n..].contains('`')) {
            return Some((c, n));
        }
    }
    None
}

fn fence_closes(line: &str, (c, n): (u8, usize)) -> bool {
    let t = line.trim();
    t.len() >= n && t.bytes().all(|b| b == c)
}

/// Line index where YAML front matter ends, when the file starts with it.
fn front_matter_end(source: &str, lines: &[Range<usize>]) -> Option<usize> {
    if lines.first().map(|l| source[l.clone()].trim_end()) != Some("---") {
        return None;
    }
    lines.iter().enumerate().skip(1).find_map(|(i, l)| {
        let t = source[l.clone()].trim_end();
        (t == "---" || t == "...").then_some(i)
    })
}

/// The value of a `title:` line, without surrounding quotes.
fn title_value(source: &str, line: Range<usize>) -> Option<Range<usize>> {
    let text = &source[line.clone()];
    let rest = text.strip_prefix("title:")?;
    let lead = rest.len() - rest.trim_start().len();
    let value = rest.trim();
    if value.is_empty() {
        return None;
    }
    let start = line.start + "title:".len() + lead;
    let (start, len) = match value.as_bytes()[0] {
        q @ (b'"' | b'\'') if value.len() >= 2 && value.as_bytes()[value.len() - 1] == q => (start + 1, value.len() - 2),
        _ => (start, value.len()),
    };
    (len > 0).then_some(start..start + len)
}

fn is_jinja_statement(line: &str) -> bool {
    let t = line.trim();
    t.starts_with("{%") && t.ends_with("%}")
}

/// Offset of the `#` to blank in `##Title`, if the line is such a heading.
fn unspaced_atx(line: &str) -> Option<usize> {
    let indent = line.len() - line.trim_start_matches(' ').len();
    if indent > 3 {
        return None;
    }
    let hashes = line[indent..].bytes().take_while(|&b| b == b'#').count();
    let next = line[indent + hashes..].chars().next()?;
    ((1..=6).contains(&hashes) && !next.is_whitespace() && next != '#').then_some(indent + hashes - 1)
}

fn scan(source: &str) -> Layout {
    let mut layout = Layout { shadow: source.as_bytes().to_vec(), extras: Vec::new(), math: Vec::new() };
    let lines = lines(source);
    let mut first = 0;
    if let Some(end) = front_matter_end(source, &lines) {
        for line in &lines[..=end] {
            if let Some(title) = title_value(source, line.clone()) {
                layout.extras.push(Extra { range: title, block_type: BlockType::Heading });
            }
            fill(&mut layout.shadow, line.clone(), b' ');
        }
        first = end + 1;
    }
    let mut fence = None;
    for line in &lines[first..] {
        let text = &source[line.clone()];
        if let Some(open) = fence {
            if fence_closes(text, open) {
                fence = None;
            }
            continue;
        }
        if let Some(open) = fence_open(text) {
            fence = Some(open);
            continue;
        }
        if scan_block_header(source, line.clone(), &mut layout) {
            continue;
        }
        if is_jinja_statement(text) {
            fill(&mut layout.shadow, line.clone(), b' ');
            continue;
        }
        if let Some(hash) = unspaced_atx(text) {
            layout.shadow[line.start + hash] = b' ';
        }
    }
    let body_start = lines.get(first).map_or(source.len(), |l| l.start);
    layout.math = math_spans_from(source, body_start);
    for m in &layout.math {
        fill(&mut layout.shadow, m.clone(), b'x');
    }
    layout.extras.sort_by_key(|e| e.range.start);
    layout
}

/// Admonition/details/tab headers; implemented in Task 3.
fn scan_block_header(_source: &str, _line: Range<usize>, _layout: &mut Layout) -> bool {
    false
}

/// Math spans as pymdownx.arithmatex (generic mode) finds them, skipping code.
pub fn math_spans(text: &str) -> Vec<Range<usize>> {
    math_spans_from(text, 0)
}

fn math_spans_from(text: &str, start: usize) -> Vec<Range<usize>> {
    let b = text.as_bytes();
    let code = code_ranges(text);
    let in_code = |i: usize| code.iter().any(|r| r.contains(&i));
    let mut out = Vec::new();
    let mut i = start;
    while i < b.len() {
        if in_code(i) {
            i += 1;
            continue;
        }
        let found = match b[i] {
            b'\\' => match b.get(i + 1) {
                Some(b'(') => find(b, i + 2, b"\\)").map(|e| i..e + 2),
                Some(b'[') => find(b, i + 2, b"\\]").map(|e| i..e + 2),
                Some(b'b') if text[i..].starts_with("\\begin{") && at_line_start(b, i) => begin_end(text, i),
                Some(_) => {
                    i += 2;
                    continue;
                }
                None => None,
            },
            b'$' if b.get(i + 1) == Some(&b'$') => find(b, i + 2, b"$$").map(|e| i..e + 2),
            b'$' => inline_dollar(b, i),
            _ => None,
        };
        match found {
            Some(r) => {
                i = r.end;
                out.push(r);
            }
            None => i += 1,
        }
    }
    out
}

fn at_line_start(b: &[u8], i: usize) -> bool {
    b[..i].iter().rev().take_while(|&&c| c != b'\n').all(|c| c.is_ascii_whitespace())
}

fn find(b: &[u8], from: usize, needle: &[u8]) -> Option<usize> {
    let mut j = from;
    while j + needle.len() <= b.len() {
        if b[j] == b'\\' && needle[0] != b'\\' {
            j += 2;
            continue;
        }
        if &b[j..j + needle.len()] == needle {
            return Some(j);
        }
        j += 1;
    }
    None
}

fn begin_end(text: &str, i: usize) -> Option<Range<usize>> {
    let name_end = text[i + 7..].find('}')? + i + 7;
    let closing = format!("\\end{{{}}}", &text[i + 7..name_end]);
    let end = text[name_end..].find(&closing)? + name_end + closing.len();
    Some(i..end)
}

/// `$...$`: no space after the opener or before the closer, not `\$`, and
/// not across a blank line.
fn inline_dollar(b: &[u8], i: usize) -> Option<Range<usize>> {
    let escaped = b[..i].iter().rev().take_while(|&&c| c == b'\\').count() % 2 == 1;
    if escaped || b.get(i + 1).is_none_or(|c| c.is_ascii_whitespace()) {
        return None;
    }
    let mut j = i + 1;
    while j < b.len() {
        match b[j] {
            b'\\' => j += 2,
            b'$' if !b[j - 1].is_ascii_whitespace() => return Some(i..j + 1),
            b'\n' if b[j + 1..].iter().take_while(|&&c| c != b'\n').all(|c| c.is_ascii_whitespace()) => return None,
            _ => j += 1,
        }
    }
    None
}

/// Fenced code blocks and inline code spans, where `$` is not math.
fn code_ranges(text: &str) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut open: Option<((u8, usize), usize)> = None;
    for line in lines(text) {
        let t = &text[line.clone()];
        match open {
            Some((f, start)) if fence_closes(t, f) => {
                out.push(start..line.end);
                open = None;
            }
            Some(_) => {}
            None => {
                if let Some(f) = fence_open(t) {
                    open = Some((f, line.start));
                }
            }
        }
    }
    if let Some((_, start)) = open {
        out.push(start..text.len());
    }
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'`' || out.iter().any(|r| r.contains(&i)) {
            i += 1;
            continue;
        }
        let n = b[i..].iter().take_while(|&&c| c == b'`').count();
        let mut j = i + n;
        let mut close = None;
        while j < b.len() {
            if b[j] == b'`' {
                let m = b[j..].iter().take_while(|&&c| c == b'`').count();
                if m == n {
                    close = Some(j + m);
                    break;
                }
                j += m;
            } else if b[j] == b'\n' && b.get(j + 1) == Some(&b'\n') {
                break;
            } else {
                j += 1;
            }
        }
        match close {
            Some(end) => {
                out.push(i..end);
                i = end;
            }
            None => i += n,
        }
    }
    out
}

/// Replace every math span with the same placeholder, for checks that must
/// not read `_` or `*` inside formulas as emphasis.
pub fn mask_math(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pos = 0;
    for r in math_spans(text) {
        out.push_str(&text[pos..r.start]);
        out.push_str("⟦M⟧");
        pos = r.end;
    }
    out.push_str(&text[pos..]);
    out
}

/// A block whose text is nothing but math (and punctuation) is not prose.
fn drop_math_only_blocks(doc: &mut Document, math: &[Range<usize>]) {
    let source = doc.source.clone();
    for block in doc.sections.iter_mut().flat_map(|s| s.blocks.iter_mut()) {
        let Some(span) = block.span.clone() else { continue };
        if !block.translatable {
            continue;
        }
        let inside: Vec<&Range<usize>> = math.iter().filter(|m| m.start >= span.start && m.end <= span.end).collect();
        if inside.is_empty() {
            continue;
        }
        let mut rest = String::new();
        let mut pos = span.start;
        for m in inside {
            rest.push_str(&source[pos..m.start]);
            pos = m.end;
        }
        rest.push_str(&source[pos..span.end]);
        if rest.chars().all(|c| c.is_whitespace() || ".,;:".contains(c)) {
            block.translatable = false;
            block.segments.clear();
            block.span = None;
        }
    }
}
```

Run: `cargo test -p yeokja-parser-mkdocs`
Expected: PASS. `math_line_that_looks_like_a_list_or_setext...` 테스트의 두 번째 분기는 정규화 방식에 따라 하나만 맞으면 된다. 실패하면 먼저 실제 세그먼트를 출력해 수식이 한 세그먼트 안에 온전히 있는지 확인하고, 단언을 그 실제 형태로 고정한다. 핵심은 세그먼트가 1개이고 `$$`가 양쪽 모두 들어 있다는 점이다.

- [ ] **Step 4: 워크스페이스 전체 테스트와 커밋**

```bash
cargo test --workspace
git add Cargo.toml Cargo.lock crates/parser-mkdocs crates/core/src/parser.rs crates/parsers crates/translate/src/prompt.rs crates/translate/src/evaluator_format.rs
git commit -m "[*] feat: MkDocs 파서 추가 — 수식·front matter·Jinja·공백 없는 제목" -m "원문과 길이가 같은 그림자 사본에서 arithmatex 수식을 가리고, 탭으로 끝나는 front matter와 Jinja 문장 줄을 비우고, ##Title 제목을 인식해 markdown-dialect에 넘긴다. 수식만 있는 블록은 번역 대상에서 뺀다." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 3: admonition·details·탭 머리 줄과 본문

**Files:**
- Modify: `crates/parser-mkdocs/src/lib.rs`(`scan_block_header` 구현, 테스트 추가)

**Interfaces:**
- Consumes: Task 2의 `Layout`, `fill`.
- Produces: `!!!`/`???`/`???+`/`===` 머리 줄의 따옴표 제목이 Heading 세그먼트로 제시되고, 들여쓴 본문이 Markdown으로 분할된다.

- [ ] **Step 1: 실패하는 테스트 추가**

```rust
    #[test]
    fn admonition_title_and_body_are_separate_prose() {
        let source = "Before.\n\n!!! info \"Lemma\"\n    The body is prose.\n    It spans lines.\n\nAfter.\n";
        assert_eq!(sources(source), ["Before.", "Lemma", "The body is prose.", "It spans lines.", "After."]);
        assert_eq!(
            translate_all(source, ko),
            "가 Before.\n\n!!! info \"가 Lemma\"\n    가 The body is prose. 가 It spans lines.\n\n가 After.\n"
        );
    }

    #[test]
    fn body_after_a_blank_line_is_prose_not_code() {
        let source = "??? hint \"Solution\"\n\n    Use a stack.\n\n    ```cpp\n    int x;\n\n    int y;\n    ```\n\nDone.\n";
        assert_eq!(sources(source), ["Solution", "Use a stack.", "Done."]);
    }

    #[test]
    fn tab_indented_body_and_header_after_paragraph() {
        let source = "Intro line\n!!! note\n\tBody with tab.\n";
        assert_eq!(sources(source), ["Intro line", "Body with tab."]);
    }

    #[test]
    fn tabs_keep_code_out_of_prose() {
        let source = "=== \"C++\"\n    ```cpp\n    int main() {}\n    ```\n=== \"Python\"\n    ```py\n    print(1)\n    ```\n";
        assert_eq!(sources(source), ["C++", "Python"]);
    }

    #[test]
    fn nested_admonition_and_list_inside_body() {
        let source = "!!! example \"Outer\"\n    - item one\n      continues\n\n    !!! note \"Inner\"\n        Inner body.\n";
        assert_eq!(sources(source), ["Outer", "item one continues", "Inner", "Inner body."]);
    }

    #[test]
    fn admonition_without_title_offers_only_body() {
        assert_eq!(sources("!!! warning\n    Careful.\n"), ["Careful."]);
    }
```

한 블록의 두 세그먼트가 재구성 때 공백으로 이어지는 방식은 `splice_reconstruct`의 기존 동작을 따른다. 첫 테스트의 실제 결과가 다르면 기존 `parser-markdown`의 여러 문장 문단 테스트와 같은 형태인지 확인하고 그 형태로 고정한다.

Run: `cargo test -p yeokja-parser-mkdocs`
Expected: 새 테스트 FAIL.

- [ ] **Step 2: `scan_block_header` 구현**

```rust
/// `!!! type "title"`, `??? type "title"`, `???+ ...`, `=== "title"`.
///
/// The header becomes the list item `-   ***` in the shadow: a bullet whose
/// content column is the header indentation plus four and whose first content
/// is a thematic break (the mixed `-`/`*` line is not itself a break). The
/// body, indented four columns (spaces or a tab) past the header, then parses
/// as the item's content even across blank lines — the structure
/// Python-Markdown gets by dedenting it. A non-empty bullet item interrupts a
/// paragraph, so a header right after a paragraph line works too.
fn scan_block_header(source: &str, line: Range<usize>, layout: &mut Layout) -> bool {
    let text = &source[line.clone()];
    let indent = text.len() - text.trim_start_matches([' ', '\t']).len();
    let rest = &text[indent..];
    let marker = ["???+", "!!!", "???", "==="].into_iter().find(|m| rest.starts_with(m));
    let Some(marker) = marker else { return false };
    let after = &rest[marker.len()..];
    if !after.starts_with([' ', '\t']) {
        return false;
    }
    if let (Some(open), Some(close)) = (after.find('"'), after.rfind('"'))
        && close > open + 1
    {
        let base = line.start + indent + marker.len();
        layout.extras.push(Extra { range: base + open + 1..base + close, block_type: BlockType::Heading });
    }
    let start = line.start + indent;
    fill(&mut layout.shadow, start..line.end, b' ');
    let item: &[u8] = if line.end - start >= 7 { b"-   ***" } else { b"- ***" };
    layout.shadow[start..start + item.len()].copy_from_slice(item);
    true
}
```

Run: `cargo test -p yeokja-parser-mkdocs`
Expected: PASS. 만약 pulldown-cmark가 `-   ***`를 수평선만 있는 항목으로 읽지 않아 테스트가 실패하면(예: 제목 추출 순서가 어긋남), 항목 내용을 `***` 대신 HTML 주석 `<!-- -->`(7바이트 `-   <!--`는 부족하므로 머리 줄 길이가 11 이상일 때 `-   <!---->`)으로 바꿔 보고, 둘 다 안 되면 원인을 조사해 이 태스크에 기록한다.

- [ ] **Step 3: 커밋**

```bash
git add crates/parser-mkdocs
git commit -m "[*] feat: MkDocs 파서가 admonition·details·탭 본문을 문단으로 분할" -m "머리 줄을 그림자에서 내용 열이 +4인 목록 항목으로 바꿔, 들여쓴 본문이 Python-Markdown과 같은 구조로 분할되게 한다. 따옴표 제목은 제목 세그먼트로 제시한다." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 4: 평가기와 프롬프트의 MkDocs 규칙

**Files:**
- Modify: `crates/translate/Cargo.toml`(의존성 `yeokja-parser-mkdocs = { path = "../parser-mkdocs" }`)
- Modify: `crates/translate/src/evaluator_format.rs`(`evaluate` 시작부, 도우미 `mkdocs_issues`)
- Modify: `crates/translate/src/prompt.rs`(`closing_rule`의 `Markup::MkDocs` 팔 분리)
- Test: 같은 파일들의 tests 모듈

**Interfaces:**
- Consumes: `yeokja_parser_mkdocs::{math_spans, mask_math}`.
- Produces: `Markup::MkDocs` 세그먼트에서 수식 변경·누락과 Jinja 구분자 추가가 `FormatLost` 오류가 된다. 수식 속 `_`는 강조 검사에서 무시된다.

- [ ] **Step 1: 실패하는 테스트 작성(`evaluator_format.rs` tests 모듈, 기존 테스트의 `EvaluationContext` 생성 방식을 따른다)**

```rust
    fn mkdocs_ctx(source: &str, translation: &str) -> EvaluationContext {
        EvaluationContext {
            source: source.to_string(),
            translation: translation.to_string(),
            glossary: HashMap::new(),
            source_lang: "en".to_string(),
            target_lang: "ko".to_string(),
            markup: Markup::MkDocs,
        }
    }

    #[tokio::test]
    async fn mkdocs_math_must_survive_byte_for_byte() {
        let ok = FormatEvaluator.evaluate(&mkdocs_ctx("Let $a_i$ be $O(n)$.", "$a_i$를 $O(n)$이라 하자.")).await.unwrap();
        assert!(ok.passed, "{:?}", ok.issues);
        let changed = FormatEvaluator.evaluate(&mkdocs_ctx("Let $a_i$ be $O(n)$.", "$a_{i}$를 $O(n)$이라 하자.")).await.unwrap();
        assert!(!changed.passed);
        let dropped = FormatEvaluator.evaluate(&mkdocs_ctx("Let $a_i$ be x.", "a_i를 x라 하자.")).await.unwrap();
        assert!(!dropped.passed);
    }

    #[tokio::test]
    async fn mkdocs_underscore_inside_math_is_not_emphasis() {
        let result = FormatEvaluator.evaluate(&mkdocs_ctx("If $x_1 < y_1$ then $z_2$.", "$x_1 < y_1$이면 $z_2$입니다.")).await.unwrap();
        assert!(result.passed, "{:?}", result.issues);
    }

    #[tokio::test]
    async fn mkdocs_added_jinja_delimiter_is_an_error() {
        let result = FormatEvaluator.evaluate(&mkdocs_ctx("Use a set.", "{# 집합 #}을 사용합니다.")).await.unwrap();
        assert!(!result.passed);
    }
```

(`HashMap`과 `tokio::test`가 이 tests 모듈에서 이미 쓰이는지 확인하고, 아니면 기존 테스트가 쓰는 비동기 실행 방식을 따른다.)

Run: `cargo test -p yeokja-translate mkdocs_`
Expected: FAIL.

- [ ] **Step 2: 평가기 구현**

`FormatEvaluator::evaluate` 본문 맨 앞(`let mut issues = Vec::new();` 앞)에 넣는다.

```rust
        if context.markup == Markup::MkDocs {
            let mut issues = mkdocs_issues(&context.source, &context.translation);
            // Formulas are copied verbatim (checked above); hide them so their
            // `_`, `*` and backticks do not count as Markdown emphasis or code.
            let masked = EvaluationContext {
                source: yeokja_parser_mkdocs::mask_math(&context.source),
                translation: yeokja_parser_mkdocs::mask_math(&context.translation),
                markup: Markup::Markdown,
                ..context.clone()
            };
            issues.extend(self.evaluate(&masked).await?.issues);
            let passed = !issues.iter().any(|i| i.severity == IssueSeverity::Error);
            return Ok(EvaluationResult { passed, issues });
        }
```

파일에 도우미를 추가한다(`is_multiset_subset` 근처).

```rust
/// Checks only MkDocs needs: math copied verbatim, and no new Jinja
/// delimiters (the macros plugin renders every page as a template).
fn mkdocs_issues(source: &str, translation: &str) -> Vec<EvaluationIssue> {
    let spans = |text: &str| -> Vec<String> {
        let mut v: Vec<String> = yeokja_parser_mkdocs::math_spans(text).into_iter().map(|r| text[r].to_string()).collect();
        v.sort_unstable();
        v
    };
    let (required, available) = (spans(source), spans(translation));
    let mut issues = Vec::new();
    if !is_multiset_subset(&required, &available) {
        issues.push(EvaluationIssue {
            severity: IssueSeverity::Error,
            kind: IssueKind::FormatLost,
            message: format!(
                "Math changed: copy every $...$, $$...$$, \\(...\\) and \\[...\\] expression \
                 byte-for-byte and translate nothing inside it; put Korean particles after the \
                 closing delimiter. Source math: {required:?}; translation math: {available:?}"
            ),
        });
    }
    for delimiter in ["{{", "{%", "{#"] {
        if translation.matches(delimiter).count() > source.matches(delimiter).count() {
            issues.push(EvaluationIssue {
                severity: IssueSeverity::Error,
                kind: IssueKind::FormatLost,
                message: format!(
                    "`{delimiter}` added: MkDocs renders pages as Jinja templates, so the \
                     translation must not introduce `{{{{`, `{{%` or `{{#`."
                ),
            });
        }
    }
    issues
}
```

`IssueSeverity`의 `PartialEq` 파생 여부를 확인하고, 없으면 기존 코드의 `has_errors` 계산 방식을 그대로 쓴다.

- [ ] **Step 3: 프롬프트 규칙**

`crates/translate/src/prompt.rs` `closing_rule`에서 Task 2에서 합친 팔을 나눠 `Markup::MkDocs` 전용 팔을 둔다.

```rust
        Markup::MkDocs => {
            "A closing _ that a letter follows does not close the pair. When the translation \
             puts a suffix straight after an italicised term, use * instead: _arity_ → \
             *arity*는. Copy every math expression between $...$, $$...$$, \\(...\\) and \
             \\[...\\] byte-for-byte — never translate, respace or rewrite anything inside it — \
             and attach Korean particles after the closing delimiter ($n$개, $O(n)$의). Never \
             add {{, {% or {#. A list item that is only a link [label](page.md) must stay \
             exactly one link with only the label translated.\n"
        }
```

`prompt.rs` tests에 `closing_rule(Markup::MkDocs)`가 `$...$`와 `{%`를 언급하는지 확인하는 테스트를 추가한다.

```rust
    #[test]
    fn mkdocs_rule_mentions_math_and_jinja() {
        let rule = closing_rule(yeokja_core::parser::Markup::MkDocs);
        assert!(rule.contains("$...$"));
        assert!(rule.contains("{%"));
    }
```

- [ ] **Step 4: 테스트와 커밋**

```bash
cargo test --workspace
git add crates/translate
git commit -m "[*] feat: MkDocs 번역의 수식 보존·Jinja 구분자 검사와 프롬프트 규칙 추가" -m "Markup::MkDocs에서 수식 다중집합이 바이트 단위로 보존되는지, Jinja 구분자가 새로 생기지 않는지 검사하고, 수식을 가린 텍스트로 기존 Markdown 검사를 재사용해 수식 속 _가 강조 오탐을 내지 않게 한다." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 5: 프로젝트 골격과 말뭉치 구조 검사

**Files:**
- Modify: `.gitmodules`; Create: `projects/cp-algorithms/upstream`(gitlink)
- Create: `projects/cp-algorithms/.gitignore`, `yeokja.toml`, `glossary.toml`
- Create: `crates/parser-mkdocs/examples/roundtrip.rs`
- Create: `projects/cp-algorithms/scripts/check_parser_corpus.py`
- Create: `nix/projects/cp-algorithms.nix`

**Interfaces:**
- Produces: `cargo run --release -p yeokja-parser-mkdocs --example roundtrip -- <src> <out>`가 `<out>/identity/`, `<out>/fake/`에 재구성본을 쓴다. `check_parser_corpus.py <src> <out>`가 구조 차이가 있으면 종료 코드 1.

- [ ] **Step 1: 서브모듈과 설정**

```bash
git submodule add https://github.com/cp-algorithms/cp-algorithms.git projects/cp-algorithms/upstream
git config -f .gitmodules submodule.projects/cp-algorithms/upstream.shallow true
git -C projects/cp-algorithms/upstream log -1 --format='%H %cd'   # README(Task 9)에 적는다
```

`projects/cp-algorithms/.gitignore`:

```gitignore
ko/
dist/
__pycache__/
```

`projects/cp-algorithms/yeokja.toml`(빌드 섹션은 Task 6):

```toml
[project]
source_lang = "en"
target_lang = "ko"
glossary = "glossary.toml"
state_dir = "state"

[[sources]]
path = "upstream/src"
pattern = "**/*.md"
exclude = ["contrib.md", "code_of_conduct.md", "preview.md"]
parser = "mkdocs"
output = "ko/src/{path}"

[[sources]]
path = "upstream"
pattern = "README.md"
parser = "mkdocs"
output = "ko/src/index_body"

[derive]
base = "upstream"

[[derive.overlay]]
path = "ko"
require_base = true

[provider]
type = "claude_code"
model = "claude-sonnet-5"

[evaluation]
auto_evaluate = true
style_evaluate = false
max_retries = 3

[translation]
concurrency = 8
batch_segments = 32
```

두 번째 source의 고정 출력 경로가 동작하는지 확인한다: `$Y translate upstream/README.md`를 아직 실행하지 말고, `$Y status upstream/README.md`가 오류 없이 대상 1개 파일을 보고하는지 본다. 출력 경로 템플릿에 `{path}`가 필수라 오류가 나면, `output = "ko/{path}"`로 바꾸고 Task 6의 `prepare_site.py`가 조립 트리에서 `src/index_body`를 `README.md` 사본으로 교체하게 한다(`require_base` 때문에 `ko/README.md`는 트리 루트의 `README.md`를 덮는다).

`projects/cp-algorithms/glossary.toml`:

```toml
# Algorithms for Competitive Programming 용어집
# 인명과 알고리즘 고유명사(Dijkstra, Kruskal, Tarjan 등)는 원어로 둔다.
[terms."segment tree"]
translation = "세그먼트 트리"
[terms."Fenwick tree"]
translation = "펜윅 트리"
[terms."disjoint set union"]
translation = "서로소 집합 유니온"
[terms."suffix array"]
translation = "접미사 배열"
[terms."convex hull"]
translation = "볼록 껍질"
[terms."modular inverse"]
translation = "모듈러 역원"
[terms.sieve]
translation = "체"
[terms."dynamic programming"]
translation = "동적 계획법"
[terms."shortest path"]
translation = "최단 경로"
[terms."minimum spanning tree"]
translation = "최소 신장 트리"
[terms."strongly connected component"]
translation = "강한 연결 요소"
[terms."bipartite graph"]
translation = "이분 그래프"
[terms.flow]
translation = "흐름"
[terms.matching]
translation = "매칭"
[terms.complexity]
translation = "복잡도"
[terms.vertex]
translation = "정점"
[terms.edge]
translation = "간선"
[terms."depth-first search"]
translation = "깊이 우선 탐색"
[terms."breadth-first search"]
translation = "너비 우선 탐색"
[terms.recursion]
translation = "재귀"
[terms.Dijkstra]
translation = "Dijkstra"
note = "인명 고유명사"
```

- [ ] **Step 2: Nix devShell**

`nix/projects/cp-algorithms.nix`:

```nix
# cp-algorithms: MkDocs Material 사이트 빌드와 파서 말뭉치 검사용 Python 환경.
# 원문 CI는 버전을 고정하지 않은 pip 설치이므로 nixpkgs(flake.lock 고정) 버전을
# 씁니다. mkdocs-simple-hooks·toggle-sidebar는 nixpkgs에 없어 번역 빌드에서
# 제거하고(scripts/prepare_site.py), git 계열·rss 플러그인은 조립 트리에서
# 동작하지 않으므로 포함하지 않습니다.
{
  pkgs,
  lib,
  system,
}:
pkgs.mkShell {
  packages = [
    (pkgs.python3.withPackages (ps: [
      ps.mkdocs
      ps.mkdocs-material
      ps.mkdocs-macros
      ps.mkdocs-literate-nav
      ps.pymdown-extensions
      ps.markdown
      ps.pyyaml
    ]))
  ];
  DISABLE_MKDOCS_2_WARNING = "true";
}
```

확인: `nix develop path:nix#cp-algorithms -c python3 -c 'import mkdocs, material, pymdownx, mkdocs_macros, mkdocs_literate_nav, yaml; print("ok")'`. 속성 이름이 다르면 `nix search path:nix` 대신 nixpkgs에서 `python3Packages.<이름>`을 찾아 맞춘다(설계 조사에서 `mkdocs-macros`, `mkdocs-literate-nav`로 확인됨).

- [ ] **Step 3: roundtrip 예제**

`crates/parser-mkdocs/examples/roundtrip.rs`:

```rust
//! Reconstruct every Markdown file under <src> twice — unchanged and with
//! each segment prefixed by `가 ` — for the corpus structure check in
//! projects/cp-algorithms/scripts/check_parser_corpus.py.
use std::fs;
use std::path::{Path, PathBuf};
use yeokja_core::parser::{DocumentParser, TranslationMap};
use yeokja_parser_mkdocs::MkdocsParser;

fn markdown_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            markdown_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "md") {
            out.push(path);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (src, out) = (Path::new(&args[1]), Path::new(&args[2]));
    let mut files = Vec::new();
    markdown_files(src, &mut files);
    for file in files {
        let Ok(text) = fs::read_to_string(&file) else { continue };
        let doc = MkdocsParser.parse(&text);
        for (mode, prefix) in [("identity", ""), ("fake", "가 ")] {
            let mut map = TranslationMap::new();
            for seg in doc.translatable_segments() {
                map.insert(seg.id.clone(), format!("{prefix}{}", seg.source));
            }
            let target = out.join(mode).join(file.strip_prefix(src).unwrap());
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(target, MkdocsParser.reconstruct(&doc, &map)).unwrap();
        }
    }
}
```

`crates/parser-mkdocs/Cargo.toml`에 예제가 `yeokja-core`만 쓰므로 추가 의존성은 없다.

- [ ] **Step 4: 말뭉치 검사 스크립트**

`projects/cp-algorithms/scripts/check_parser_corpus.py`(설계 조사 스파이크의 `validate.py`를 다듬은 것):

```python
"""Render original and reconstructed pages with cp-algorithms' Markdown
extensions and fail if the mkdocs parser changed their structure.

usage: check_parser_corpus.py <upstream/src> <roundtrip-out>
"""
from html.parser import HTMLParser
from pathlib import Path
import re
import sys

import markdown
from mkdocs.utils import meta

EXTENSIONS = ['toc', 'tables', 'pymdownx.arithmatex', 'pymdownx.highlight', 'admonition',
              'pymdownx.details', 'pymdownx.superfences', 'pymdownx.tabbed', 'attr_list', 'meta']
CONFIG = {'pymdownx.arithmatex': {'generic': True}, 'pymdownx.tabbed': {'alternate_style': True},
          'toc': {'permalink': True}}


def render(text):
    body, _ = meta.get_data(text)
    return markdown.Markdown(extensions=EXTENSIONS, extension_configs=CONFIG).convert(body)


class Shape(HTMLParser):
    """Tag/class sequence plus the verbatim contents of math and code."""

    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.tags, self.raw, self.stack = [], [], []

    def handle_starttag(self, tag, attrs):
        cls = dict(attrs).get('class') or ''
        self.tags.append((tag, cls))
        verbatim = 'arithmatex' in cls or tag == 'code'
        self.stack.append((tag, verbatim))
        if verbatim and sum(v for _, v in self.stack) == 1:
            self.raw.append('')

    def handle_endtag(self, tag):
        self.tags.append(('/' + tag, ''))
        if self.stack and self.stack[-1][0] == tag:
            self.stack.pop()

    def handle_data(self, data):
        if any(v for _, v in self.stack):
            self.raw[-1] += re.sub(r'\s+', '', data)


def shape(html):
    s = Shape()
    s.feed(html)
    return s.tags, s.raw


def main(src, out):
    failures = []
    for original in sorted(Path(src).rglob('*.md')):
        rel = original.relative_to(src)
        expected = shape(render(original.read_text()))
        for mode in ('identity', 'fake'):
            path = Path(out) / mode / rel
            if not path.exists():
                continue
            if shape(render(path.read_text())) != expected:
                failures.append(f'{mode}: {rel}')
    for failure in failures:
        print(failure)
    print(f'structure differences: {len(failures)}')
    return 1 if failures else 0


if __name__ == '__main__':
    sys.exit(main(*sys.argv[1:3]))
```

- [ ] **Step 5: 말뭉치 검사 실행**

```bash
cargo run --release -p yeokja-parser-mkdocs --example roundtrip -- projects/cp-algorithms/upstream/src build/cp-roundtrip
nix develop path:nix#cp-algorithms -c python3 projects/cp-algorithms/scripts/check_parser_corpus.py projects/cp-algorithms/upstream/src build/cp-roundtrip
```

기대: `structure differences: 0`. 차이가 있으면 파일마다 원인을 찾아 파서(Task 2·3)에 실패하는 단위 테스트를 먼저 추가한 뒤 고친다. 원문 자체가 Python-Markdown에서도 모호한 경우(예: 들여쓰기 규칙이 CommonMark와 근본적으로 다른 구문)만 예외로 두고, 그 파일과 이유를 이 스텝에 기록한 뒤 번역 후 수동 확인 목록에 넣는다. `build/cp-roundtrip`은 루트 `.gitignore`의 `build/`로 제외된다.

- [ ] **Step 6: 분할 확인과 커밋**

```bash
cargo build --release -p yeokja-cli
cd projects/cp-algorithms
$Y status upstream/src | tail -3
$Y coverage upstream/src --min-lines 5 | tail -20   # 건너뛴 구간이 코드·front matter·admonition 머리 줄뿐인지 확인
cd ../..
git add .gitmodules projects/cp-algorithms/upstream projects/cp-algorithms/.gitignore projects/cp-algorithms/yeokja.toml projects/cp-algorithms/glossary.toml projects/cp-algorithms/scripts/check_parser_corpus.py crates/parser-mkdocs/examples nix/projects/cp-algorithms.nix
git commit -m "[cp-algorithms] feat: 원문 서브모듈, 번역 설정, 파서 말뭉치 구조 검사 추가" -m "cp-algorithms 전체를 항등·가짜 번역으로 재구성해 Python-Markdown 렌더링 구조가 원문과 같은지 확인하는 검사를 둔다. (#2)" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 6: 사이트 준비, 영어 제목 ID 훅, 빌드

**Files:**
- Create: `projects/cp-algorithms/scripts/prepare_site.py`, `scripts/test_prepare_site.py`
- Create: `projects/cp-algorithms/scripts/extract_heading_ids.py`, `scripts/test_extract_heading_ids.py`
- Create: `projects/cp-algorithms/overlay/hooks/ko_anchors.py`, `scripts/test_ko_anchors.py`
- Create: `projects/cp-algorithms/overlay/src/overrides/partials/content.html`
- Create: `projects/cp-algorithms/scripts/check_links.py`(webassembly-component-docs 사본)
- Modify: `projects/cp-algorithms/yeokja.toml`(`[[derive.overlay]] overlay`, `[[derive.step]]`, `[build.html]`)

**Interfaces:**
- Produces: `prepare_site.prepare(text: str, english: bool, site_url: str) -> str`(mkdocs.yml 텍스트 변환), CLI `prepare_site.py [--english] <tree>`; `extract_heading_ids.extract(site: Path) -> dict[str, list[str]]`, CLI `extract_heading_ids.py <site> <out.json>`; 훅은 환경 변수 `KO_ANCHOR_IDS`(JSON 경로)를 읽는다.

- [ ] **Step 1: `prepare_site.py` 테스트 먼저**

`scripts/test_prepare_site.py`:

```python
import unittest
from prepare_site import prepare

UPSTREAM = '''site_name: Algorithms for Competitive Programming
site_url: https://cp-algorithms.com
edit_uri: edit/main/src/
copyright: Text is available under the <a href="x">CC BY-SA 4.0</a> License
extra_javascript:
  - javascript/config.js
  - javascript/donation-banner.js
  - https://unpkg.com/mathjax@3/es5/tex-mml-chtml.js
markdown_extensions:
  - pymdownx.emoji:
      emoji_index: !!python/name:material.extensions.emoji.twemoji
plugins:
  - toggle-sidebar:
      toggle_button: all
  - mkdocs-simple-hooks:
      hooks:
          on_env: "hooks:on_env"
  - search
  - tags
  - literate-nav:
      nav_file: navigation.md
  - git-revision-date-localized:
      enabled: !ENV [MKDOCS_ENABLE_GIT_REVISION_DATE, False]
  - git-authors
  - git-committers:
      token: !ENV MKDOCS_GIT_COMMITTERS_APIKEY
  - macros
  - rss
extra:
  analytics:
    provider: google
theme:
  name: material
'''


class PrepareTests(unittest.TestCase):
    def test_korean_config(self):
        out = prepare(UPSTREAM, english=False, site_url='https://example.org/cp-algorithms/')
        for gone in ('toggle-sidebar', 'mkdocs-simple-hooks', 'git-revision', 'git-authors',
                     'git-committers', '- rss', 'analytics', 'donation-banner', 'edit_uri'):
            self.assertNotIn(gone, out)
        self.assertIn('language: ko', out)
        self.assertIn('hooks/ko_anchors.py', out)
        self.assertIn('- hooks.py', out)
        self.assertIn('site_url: https://example.org/cp-algorithms/', out)
        self.assertIn('!!python/name:material.extensions.emoji.twemoji', out)
        self.assertIn('비공식 번역', out)

    def test_english_config_has_no_korean_hook(self):
        out = prepare(UPSTREAM, english=True, site_url='https://example.org/')
        self.assertNotIn('ko_anchors', out)
        self.assertNotIn('language: ko', out)
        self.assertIn('- hooks.py', out)

    def test_missing_expected_entry_fails(self):
        with self.assertRaises(ValueError):
            prepare(UPSTREAM.replace('  - macros\n', ''), english=False, site_url='x')


if __name__ == '__main__':
    unittest.main()
```

Run: `(cd projects/cp-algorithms/scripts && nix develop path:../../../nix#cp-algorithms -c python3 -m unittest test_prepare_site)`
Expected: FAIL(`No module named 'prepare_site'`).

- [ ] **Step 2: `prepare_site.py` 구현**

```python
"""Turn upstream mkdocs.yml into the config the translated (or English
reference) build uses, inside an assembled tree.

Upstream's YAML carries Python tags (!!python/name, !ENV), so it is edited as
text, entry by entry, and every edit must find what it expects: a changed
upstream config fails the build instead of shipping a half-applied one.
"""
import argparse
from pathlib import Path
import re

PLUGIN_BLOCKS = ['toggle-sidebar', 'mkdocs-simple-hooks', 'git-revision-date-localized',
                 'git-authors', 'git-committers', 'rss']
REQUIRED_PLUGINS = ['search', 'tags', 'literate-nav', 'macros']
COPYRIGHT_KO = ('copyright: 원문은 <a href="https://github.com/cp-algorithms/cp-algorithms/blob/main/LICENSE">'
                'CC BY-SA 4.0</a>으로 제공되며 © 2014-2025 '
                '<a href="https://github.com/cp-algorithms/cp-algorithms/graphs/contributors">cp-algorithms contributors</a>입니다.'
                '<br/>이 사이트는 원문을 한국어로 옮긴 비공식 번역(수정본)이며 같은 CC BY-SA 4.0으로 제공합니다.')


def remove_block(text, name):
    """Remove a `  - name` list entry together with its indented options."""
    pattern = re.compile(r'^  - ' + re.escape(name) + r'(:[^\n]*)?\n(?:    [^\n]*\n|      [^\n]*\n)*', re.M)
    new, count = pattern.subn('', text)
    if count != 1:
        raise ValueError(f'expected one plugin entry {name!r}, found {count}')
    return new


def replace_line(text, pattern, replacement):
    new, count = re.subn(pattern, replacement, text, count=1, flags=re.M)
    if count != 1:
        raise ValueError(f'expected a line matching {pattern!r}')
    return new


def prepare(text, english, site_url):
    for name in REQUIRED_PLUGINS:
        if not re.search(r'^  - ' + re.escape(name) + r'\b', text, re.M):
            raise ValueError(f'plugin {name!r} missing from upstream config')
    for name in PLUGIN_BLOCKS:
        text = remove_block(text, name)
    text = replace_line(text, r'^  - javascript/donation-banner\.js\n', '')
    text = replace_line(text, r'^edit_uri:.*\n', '')
    text = replace_line(text, r'^site_url:.*$', f'site_url: {site_url}')
    text = re.sub(r'^  analytics:\n(?:    [^\n]*\n)*', '', text, flags=re.M)
    hooks = ['hooks.py'] if english else ['hooks.py', 'hooks/ko_anchors.py']
    text += '\nhooks:\n' + ''.join(f'  - {h}\n' for h in hooks)
    if not english:
        text = replace_line(text, r'^copyright:.*$', COPYRIGHT_KO)
        text = replace_line(text, r'^site_name:.*$', 'site_name: 경쟁 프로그래밍을 위한 알고리즘')
        text = replace_line(text, r'^theme:\n', 'theme:\n  language: ko\n')
        text = replace_line(text, r'^  - search\n', '  - search:\n      lang:\n        - ko\n        - en\n')
    return text


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--english', action='store_true')
    parser.add_argument('--site-url', default='https://yeokja.github.io/yeokja/cp-algorithms/')
    parser.add_argument('tree')
    args = parser.parse_args()
    config = Path(args.tree) / 'mkdocs.yml'
    text = config.read_text()
    if config.is_symlink():
        config.unlink()
    config.write_text(prepare(text, args.english, args.site_url))


if __name__ == '__main__':
    main()
```

원문 `hooks.py`(`on_env`)는 MkDocs 기본 `hooks:`가 같은 서명으로 부르므로 그대로 쓴다. 조립 트리의 `mkdocs.yml`은 원문으로의 링크일 수 있으므로 링크를 끊고 새로 쓴다(원문을 덮어쓰지 않는다). 실제 Pages 기본 주소는 `.github/workflows/pages.yml`과 기존 사이트(`site/index.html`의 다른 프로젝트 링크 방식)를 확인해 `--site-url` 기본값을 맞춘다.

Run: `python3 -m unittest test_prepare_site` → PASS.

- [ ] **Step 3: `content.html` 오버레이**

`overlay/src/overrides/partials/content.html`(원문 파일에서 git 정보·기여자·편집 버튼을 빼고 문구를 한국어로 옮긴 것):

```html
{#- 한국어 번역 빌드용. 원문 partials/content.html에서 git 기반 기여자·수정일과
    원문 저장소 편집·문제 보고 버튼을 빼고, 영어 원문 링크를 더했다. -#}
<a href="https://cp-algorithms.com/{{ page.url }}" title="영어 원문" class="md-content__button md-icon">
  {% include ".icons/material/translate.svg" %}
</a>
<ul class="metadata page-metadata" data-bi-name="page info" lang="ko" dir="ltr">
{% if page and page.meta and page.meta.e_maxx_link %}
  {% set links = page.meta.e_maxx_link %}
  {% set links = [links] if links is string else links %}
  {% for link in links %}
    {% do tags.append({'name':'출처: e-maxx.ru', 'url': 'https://e-maxx.ru/algo/' + link}) %}
  {% endfor %}
{% endif %}
{% for tag in tags %}
  {% if tag.url %}
    <a href="{{ tag.url | url }}" class="md-tag">{{ tag.name }}</a>
  {% else %}
    <span class="md-tag">{{ tag.name }}</span>
  {% endif %}
{% endfor %}
</ul>

{% if not "\x3ch1" in page.content %}
  <h1>{{ page.title | d(config.site_name, true)}}</h1>
{% endif %}
{{ page.content }}
```

- [ ] **Step 4: 제목 ID 추출 스크립트와 테스트**

`scripts/test_extract_heading_ids.py`:

```python
from pathlib import Path
import tempfile
import unittest
from extract_heading_ids import extract


class ExtractTests(unittest.TestCase):
    def test_only_markdown_headings_with_permalinks(self):
        with tempfile.TemporaryDirectory() as d:
            root = Path(d)
            (root / 'graph').mkdir()
            (root / 'graph' / 'dijkstra.html').write_text(
                '<h1>Template title</h1><article class="md-content__inner">'
                '<h1 id="dijkstra">Dijkstra<a class="headerlink" href="#dijkstra">¶</a></h1>'
                '<div><h3 id="raw">raw html</h3></div>'
                '<h2 id="algorithm">Algorithm<a class="headerlink" href="#algorithm">¶</a></h2>'
                '</article>')
            self.assertEqual(extract(root), {'graph/dijkstra.md': ['dijkstra', 'algorithm']})


if __name__ == '__main__':
    unittest.main()
```

`scripts/extract_heading_ids.py`:

```python
"""Map each page of an English MkDocs build to its Markdown heading ids.

Only headings that carry a toc permalink (`a.headerlink`) come from Markdown;
template headings and raw-HTML headings are skipped, matching what the
ko_anchors treeprocessor sees.
"""
from html.parser import HTMLParser
import json
from pathlib import Path
import sys


class Headings(HTMLParser):
    def __init__(self):
        super().__init__(convert_charrefs=True)
        self.ids, self.current = [], None

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if tag in ('h1', 'h2', 'h3', 'h4', 'h5', 'h6'):
            self.current = attrs.get('id')
        elif tag == 'a' and self.current and 'headerlink' in (attrs.get('class') or ''):
            self.ids.append(self.current)
            self.current = None

    def handle_endtag(self, tag):
        if tag in ('h1', 'h2', 'h3', 'h4', 'h5', 'h6'):
            self.current = None


def extract(site):
    site = Path(site)
    pages = {}
    for page in sorted(site.rglob('*.html')):
        parser = Headings()
        parser.feed(page.read_text())
        if parser.ids:
            pages[page.relative_to(site).with_suffix('.md').as_posix()] = parser.ids
    return pages


if __name__ == '__main__':
    Path(sys.argv[2]).write_text(json.dumps(extract(sys.argv[1]), ensure_ascii=False, indent=1))
```

Run: `python3 -m unittest test_extract_heading_ids` → PASS.

- [ ] **Step 5: `ko_anchors.py` 훅과 테스트**

`overlay/hooks/ko_anchors.py`:

```python
"""MkDocs hook: give translated headings the ids of their English originals.

A Python-Markdown treeprocessor runs after attr_list and before toc, so the
toc, permalinks and search index all use the English ids. Headings are paired
by document order with the English build's list (extract_heading_ids.py); a
count mismatch means translation changed the heading structure and fails the
build.
"""
import json
import os
from pathlib import Path

from markdown.extensions import Extension
from markdown.treeprocessors import Treeprocessor

HEADINGS = ('h1', 'h2', 'h3', 'h4', 'h5', 'h6')
GENERATED = {'tags.md'}
_state = {'page': None, 'ids': {}}


class EnglishIds(Treeprocessor):
    def run(self, root):
        page = _state['page']
        ids = _state['ids'].get(page)
        if page in GENERATED or ids is None:
            return
        headings = [el for el in root.iter() if el.tag in HEADINGS]
        if len(headings) != len(ids):
            titles = [''.join(el.itertext()) for el in headings]
            raise ValueError(f'{page}: {len(headings)} translated headings {titles} '
                             f'vs {len(ids)} English ids {ids}')
        for element, heading_id in zip(headings, ids):
            element.set('id', heading_id)


class EnglishIdsExtension(Extension):
    def extendMarkdown(self, md):
        # attr_list is 8 and toc is 5 in Python-Markdown's treeprocessor registry.
        md.treeprocessors.register(EnglishIds(md), 'ko_anchors', 6)


def on_config(config):
    _state['ids'] = json.loads(Path(os.environ['KO_ANCHOR_IDS']).read_text())
    config.markdown_extensions.append(EnglishIdsExtension())
    return config


def on_page_markdown(markdown, page, config, files):
    _state['page'] = page.file.src_uri
    return markdown
```

`scripts/test_ko_anchors.py`:

```python
import importlib.util
from pathlib import Path
import unittest

import markdown

spec = importlib.util.spec_from_file_location(
    'ko_anchors', Path(__file__).parent.parent / 'overlay' / 'hooks' / 'ko_anchors.py')
ko_anchors = importlib.util.module_from_spec(spec)
spec.loader.exec_module(ko_anchors)


def render(text, page, ids):
    ko_anchors._state.update(page=page, ids=ids)
    md = markdown.Markdown(extensions=['attr_list', ko_anchors.EnglishIdsExtension(), 'toc'],
                           extension_configs={'toc': {'permalink': True}})
    return md.convert(text)


class AnchorTests(unittest.TestCase):
    def test_korean_headings_get_english_ids_and_permalinks(self):
        html = render('# 다익스트라\n\n## 알고리즘\n', 'graph/dijkstra.md',
                      {'graph/dijkstra.md': ['dijkstra', 'algorithm']})
        self.assertIn('id="dijkstra"', html)
        self.assertIn('href="#algorithm"', html)

    def test_heading_count_mismatch_fails(self):
        with self.assertRaises(ValueError):
            render('# 하나\n', 'a.md', {'a.md': ['one', 'two']})

    def test_generated_page_is_skipped(self):
        self.assertIn('<h1', render('# 태그\n', 'tags.md', {'tags.md': ['tags', 'x']}))


if __name__ == '__main__':
    unittest.main()
```

Run: `(cd projects/cp-algorithms/scripts && nix develop path:../../../nix#cp-algorithms -c python3 -m unittest test_prepare_site test_extract_heading_ids test_ko_anchors)` → PASS.

- [ ] **Step 6: 링크 검사 사본과 빌드 설정**

```bash
cp projects/webassembly-component-docs/scripts/check_links.py projects/cp-algorithms/scripts/
```

`projects/cp-algorithms/yeokja.toml`의 `[[derive.overlay]] path = "ko"` 블록 뒤에 추가:

```toml
[[derive.overlay]]
path = "overlay"

[[derive.step]]
kind = "generate"
command = 'python3 "$YEOKJA_ROOT/scripts/prepare_site.py" .'

[build.html]
command = '''
set -e
rm -rf "$YEOKJA_ROOT/build/original-source" "$YEOKJA_ROOT/build/original"
cp -RL "$YEOKJA_ROOT/upstream" "$YEOKJA_ROOT/build/original-source"
cp -R "$YEOKJA_ROOT/overlay/src/." "$YEOKJA_ROOT/build/original-source/src/"
python3 "$YEOKJA_ROOT/scripts/prepare_site.py" --english "$YEOKJA_ROOT/build/original-source"
(cd "$YEOKJA_ROOT/build/original-source" && mkdocs build --strict -d "$YEOKJA_ROOT/build/original")
python3 "$YEOKJA_ROOT/scripts/extract_heading_ids.py" "$YEOKJA_ROOT/build/original" "$YEOKJA_ROOT/build/en-heading-ids.json"
KO_ANCHOR_IDS="$YEOKJA_ROOT/build/en-heading-ids.json" mkdocs build --strict -d site
python3 "$YEOKJA_ROOT/scripts/check_links.py" "$YEOKJA_ROOT/build/original" site
'''
outputs = ["site"]
```

`derive.step`의 정확한 TOML 형식(`kind = "generate"`)은 `projects/hott/yeokja.toml`의 `[[derive.step]]` 예로 확인한다. `cp -RL`은 `src/index_body` 같은 심볼릭 링크를 실제 파일로 복사해 영어 빌드가 원문 README를 포함하게 한다.

- [ ] **Step 7: 번역 없이 빌드(영어 내용 그대로)로 파이프라인 검증**

```bash
cd projects/cp-algorithms
nix develop path:../../nix#cp-algorithms -c $Y build html 2>&1 | tail -5
```

기대: 두 빌드 모두 `--strict` 통과, `New: 0`. 아직 번역이 없으므로 한국어 트리는 영어 내용에 한국어 설정만 적용된 상태이며, 앵커 훅은 모든 페이지에서 개수가 일치해야 한다. 실패하면 원인을 고친다(설정 제거 누락, 템플릿 변수 등).

- [ ] **Step 8: 커밋**

```bash
cd ../..
git add projects/cp-algorithms
git commit -m "[cp-algorithms] feat: 한국어 MkDocs 빌드와 영어 제목 ID 보존 훅 추가" -m "원문 mkdocs.yml을 번역 빌드용으로 바꾸는 준비 스크립트, 영어 빌드의 제목 ID를 한국어 제목에 입히는 Python-Markdown 훅, 원문 대비 깨진 링크 검사를 둔다." -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 7: 파일럿 번역

**Files:** `projects/cp-algorithms/state/**`(파일럿 대상), 필요 시 `glossary.toml`

- [ ] **Step 1: 번역 동시 사용 확인 후 파일럿 번역**

```bash
pgrep -fl 'yeokja translate' || echo "free"
cd projects/cp-algorithms
$Y translate upstream/src/graph/dijkstra.md
$Y translate upstream/src/navigation.md
$Y translate upstream/README.md
for f in $(grep -rlE '^\s*(!!!|\?\?\?)' upstream/src | head -2); do $Y translate "$f"; done
for f in $(grep -rlE '^\s*=== "' upstream/src | head -1); do $Y translate "$f"; done
```

- [ ] **Step 2: 파일럿 검사**

```bash
$Y evaluate --mechanical-only upstream/src 2>&1 | tail -20
nix develop path:../../nix#cp-algorithms -c $Y build html 2>&1 | tail -5
```

직접 확인한다.

- `ko/src/graph/dijkstra.md`를 원문과 나란히 읽는다. 수식이 그대로이고, 격식체이며, 용어집을 따르는지 본다.
- `ko/src/navigation.md`의 각 항목이 `[번역된 제목](page.md)` 형태인지 본다.
- 빌드된 `dist/site/graph/dijkstra.html`에서 수식 렌더링을 확인한다(MathJax 클래스 `arithmatex`가 있어야 한다).
- `#algorithm` 같은 영어 ID와 admonition 접기가 동작하는지 확인한다.

문제가 체계적이면 용어집·프롬프트 규칙(Task 4)을 보강하고, 해당 state를 지운 뒤 다시 번역한다.

- [ ] **Step 3: 커밋**

```bash
cd ../.. && git add projects/cp-algorithms/state projects/cp-algorithms/glossary.toml
git commit -m "[cp-algorithms] feat: 파일럿 문서 번역" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 8: 전체 번역

- [ ] **Step 1: 전체 번역(백그라운드, 장시간)**

```bash
pgrep -fl 'yeokja translate' || echo "free"
cd projects/cp-algorithms
$Y translate upstream/src > build/translate-src.log 2>&1
$Y translate upstream/README.md > build/translate-readme.log 2>&1
```

중단되면 같은 명령을 다시 실행한다. 번역된 세그먼트는 건너뛴다.

- [ ] **Step 2: 완결성·기계 검사·빌드**

```bash
$Y status --check upstream/src && $Y status --check upstream/README.md
$Y evaluate --mechanical-only upstream/src 2>&1 | tail -30
nix develop path:../../nix#cp-algorithms -c $Y build html 2>&1 | tail -5
```

기대: `status --check` 통과, 평가 오류 없음(남은 오류는 원인을 보고 state를 고치거나 재번역), 빌드 `--strict` 통과·앵커 불일치 0·`New: 0`. 앵커 훅이 개수 불일치를 보고하면 그 페이지의 제목 세그먼트 번역이 구조를 바꾼 것이므로(예: 제목에 `#` 추가, 목록 항목을 제목으로 바꿈) 해당 state를 고친다.

- [ ] **Step 3: 커밋**

```bash
cd ../.. && git add projects/cp-algorithms/state projects/cp-algorithms/glossary.toml
git commit -m "[cp-algorithms] feat: 전체 한국어 번역" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```

---

### Task 9: 배포 배선, 랜딩, README

**Files:**
- Modify: `.github/pages-projects.json`, `.github/scripts/stage-pages.sh`, `.github/scripts/rebuild-translations.sh`, `.github/workflows/pr.yml`, `site/index.html`
- Create: `projects/cp-algorithms/README.md`

- [ ] **Step 1: Pages 배선**

`.github/pages-projects.json` 배열 끝:

```json
  {
    "project": "cp-algorithms",
    "target": "html",
    "artifact": "dist-cp-algorithms",
    "artifact_path": "projects/cp-algorithms/dist"
  }
```

`stage-pages.sh`의 다른 `overlay_site` 줄들 뒤: `overlay_site "dist-cp-algorithms" "site" "cp-algorithms"`

`rebuild-translations.sh`의 분기 목록에:

```bash
  cp-algorithms)
    # 문서 디렉터리 전체와 홈 본문(README → src/index_body)을 함께 확인해
    # upstream이 추가한 문서가 state 없이 빠지는 경우도 배포를 막습니다.
    "$yeokja" translate upstream/src
    "$yeokja" status --check upstream/src
    "$yeokja" translate upstream/README.md
    "$yeokja" status --check upstream/README.md
    ;;
```

`pr.yml`의 rustc-dev-guide 테스트 스텝 뒤:

```yaml
      - name: Test cp-algorithms site preparation and heading anchors
        run: |
          pip install --quiet markdown pyyaml
          python3 -m unittest discover -s projects/cp-algorithms/scripts -p 'test_*.py'
```

(`pr.yml`이 다른 Python 테스트에서 의존성을 어떻게 설치하는지 먼저 보고 같은 방식을 쓴다. nix를 쓰는 잡이면 `nix develop path:nix#cp-algorithms -c python3 -m unittest ...`로 바꾼다.)

- [ ] **Step 2: 랜딩 — "알고리즘" 주제 추가**

`site/index.html`에서 기존 필터 버튼 목록(`data-topic="hardware"` 버튼 근처)에 같은 형식으로 `data-topic="algorithms"` 버튼(`알고리즘`)을 추가하고, 주제 섹션 목록에 다음 섹션을 추가한다(이미 #3 작업이 추가했다면 항목만 추가).

```html
          <section class="topic" id="algorithms" aria-labelledby="algorithms-heading">
            <div class="topic-head"><h3 id="algorithms-heading">알고리즘</h3><p>자료 구조와 알고리즘 설계</p></div>
            <ul class="works">
      <li class="work" data-topics="algorithms">
        <div class="subjects"><span>알고리즘</span></div>
        <h4><a class="work-title" href="cp-algorithms/">경쟁 프로그래밍을 위한 알고리즘</a></h4>
        <p class="work-desc">그래프, 수론, 문자열, 기하, 자료 구조까지 경쟁 프로그래밍에서 쓰이는 알고리즘을 증명과 구현과 함께 설명하는 cp-algorithms의 한국어 번역입니다.</p>
        <p class="work-meta">원문 <a href="https://github.com/cp-algorithms/cp-algorithms">cp-algorithms/cp-algorithms</a><span class="tag">CC BY-SA 4.0</span><span class="tag">기계 번역</span></p>
      </li>
            </ul>
          </section>
```

필터 버튼의 개수 표시(`<span class="count">`)가 스크립트로 계산되는지 확인한다.

- [ ] **Step 3: 실제 모델 확인(AGENTS.md)과 README**

```bash
git log -p --format='%h %cd' -- projects/cp-algorithms/yeokja.toml | grep -E '^[0-9a-f]{7} |model|type ='
```

`projects/cp-algorithms/README.md`:

````markdown
# 경쟁 프로그래밍을 위한 알고리즘 (cp-algorithms 한국어 번역)

[cp-algorithms](https://cp-algorithms.com)([cp-algorithms/cp-algorithms](https://github.com/cp-algorithms/cp-algorithms))를
한국어로 옮긴 비공식 번역입니다. [yeokja](https://github.com/yeokja/yeokja)와 함께 Anthropic 사의
`claude-sonnet-5` 모델을 활용하여 번역되었으며 학습을 모두 비허용한 상태로 작업하였습니다.

## 범위

- 원문: `upstream/` 서브모듈, 커밋 `<Task 5 Step 1의 해시>`
- 번역 대상: `upstream/src/**/*.md`와 홈 본문 `upstream/README.md`
- 제외: `contrib.md`, `code_of_conduct.md`(원문 저장소 기여 절차), `preview.md`(원문 미리보기 도구)
- 수식, 코드, front matter의 태그는 원문 그대로입니다. 태그 "Translated"는 러시아어 e-maxx에서 옮긴 문서, "Original"은 cp-algorithms에서 새로 쓴 문서라는 원문 표시입니다.

## 라이선스

원문은 cp-algorithms contributors가 [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/)으로
공개했습니다. 이 번역은 원문을 수정한 개작물로서 같은 CC BY-SA 4.0으로 제공하며, 사이트 하단에
원저작자·라이선스·번역 사실을 표시합니다. cp-algorithms의 공식 사이트가 아닙니다.

## 재현

```sh
cd projects/cp-algorithms
../../target/release/yeokja translate upstream/src
../../target/release/yeokja translate upstream/README.md
../../target/release/yeokja status --check upstream/src
nix develop path:../../nix#cp-algorithms -c ../../target/release/yeokja build html   # dist/site
```

`state/`는 번역의 진실의 원천이므로 커밋합니다. `ko/`, `build/`, `dist/`는 재생성되므로 커밋하지 않습니다.

## 구성

- 파서: `mkdocs`(`crates/parser-mkdocs`) — arithmatex 수식, admonition·details·탭, front matter, Jinja를 인식합니다.
- 빌드: 영어 원본을 먼저 빌드해 제목 ID를 뽑고(`scripts/extract_heading_ids.py`), 한국어 빌드에서 `overlay/hooks/ko_anchors.py`가 번역된 제목에 같은 ID를 입힙니다. `scripts/prepare_site.py`가 원문 `mkdocs.yml`에서 조립 트리에서 동작하지 않는 git·rss 플러그인과 분석 설정을 뺍니다.
````

`<Task 5 Step 1의 해시>`는 실제 해시로 바꾼다.

- [ ] **Step 4: 검증**

```bash
python3 -m json.tool .github/pages-projects.json > /dev/null
python3 -m unittest discover -s .github/scripts -p 'test_*.py'
rm -rf projects/cp-algorithms/ko && .github/scripts/rebuild-translations.sh cp-algorithms
(cd projects/cp-algorithms && nix develop path:../../nix#cp-algorithms -c ../../target/release/yeokja build html 2>&1 | tail -3)
cargo test --workspace
git status --short
```

기대: 모두 통과, `New: 0`, 커밋하지 않을 파일이 보이지 않음.

- [ ] **Step 5: 커밋**

```bash
git add .github site/index.html projects/cp-algorithms/README.md
git commit -m "[cp-algorithms] feat: Pages 배포 배선과 README 추가" -m "Pages 빌드 매트릭스·스테이징·CI 재구성 분기·PR 테스트와 랜딩의 알고리즘 주제를 추가하고, README에 범위·라이선스·재현 방법·출처를 적는다. (#2)" -m "Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>"
```
