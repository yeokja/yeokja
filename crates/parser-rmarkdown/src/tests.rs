use super::*;

/// Excerpts of OpenActTexts/LDAVer2 (Loss Data Analytics, 2nd ed., CC BY 4.0).
const CONTINUOUS: &str = include_str!("../tests/fixtures/Continuous.Rmd");
const FREQUENCY: &str = include_str!("../tests/fixtures/Frequency-excerpt.Rmd");

fn sources_with(parser: &RmarkdownParser, source: &str) -> Vec<String> {
    parser.parse(source).translatable_segments().iter().map(|s| s.source.clone()).collect()
}

fn sources(source: &str) -> Vec<String> {
    sources_with(&RmarkdownParser::default(), source)
}

fn translate_with(parser: &RmarkdownParser, source: &str, f: impl Fn(&str) -> String) -> String {
    let doc = parser.parse(source);
    let mut map = TranslationMap::new();
    for seg in doc.translatable_segments() {
        map.insert(seg.id.clone(), f(&seg.source));
    }
    parser.reconstruct(&doc, &map)
}

fn translate_all(source: &str, f: impl Fn(&str) -> String) -> String {
    translate_with(&RmarkdownParser::default(), source, f)
}

fn ko(s: &str) -> String {
    format!("가 {s}")
}

#[test]
fn untranslated_round_trip_is_byte_identical() {
    for source in [CONTINUOUS, FREQUENCY] {
        let parser = RmarkdownParser::default();
        let doc = parser.parse(source);
        assert_eq!(parser.reconstruct(&doc, &TranslationMap::new()), source);
        // Translating every segment touches only the spans: the code and the
        // math survive byte for byte.
        // math survives too, up to the line breaks a segment joins.
        let out = translate_all(source, ko);
        for fence in scan_regions(source, 0).fences {
            assert!(out.contains(&source[fence.range]));
        }
        let squash = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
        let squashed = squash(&out);
        for m in math_spans(source) {
            assert!(squashed.contains(&squash(&source[m.clone()])), "{}", &source[m]);
        }
    }
}

/// Continuous.Rmd L938: a `$$` table row starting with `~~~~` read as a
/// tilde fence by CommonMark, swallowing the rest of the file.
#[test]
fn lines_inside_display_math_open_no_blocks() {
    let segs = sources(CONTINUOUS);
    // Prose after the first `~~~~` row is still offered.
    let last_heading = CONTINUOUS.lines().rfind(|l| l.starts_with('#')).unwrap();
    let title = last_heading.trim_start_matches('#').trim();
    let title = title.split(" {").next().unwrap();
    assert!(segs.iter().any(|s| s == title), "{title} missing from {} segments", segs.len());
    assert!(segs.iter().any(|s| s == "**`R` Commands**"));
    // No formula reaches a segment.
    assert!(segs.iter().all(|s| !s.contains("\\mathrm{E}[X^k]")), "{:?}", segs.iter().find(|s| s.contains("\\mathrm{E}[X^k]")));
    let source = "Before.\n\n$$\n\\begin{array}{l}\n  ~~~~\\mathrm{E}[X^k] \\\\\n+ a \\\\\n# b\n\\end{array}\n$$\n\nAfter `$x$`.\n";
    assert_eq!(sources(source), ["Before.", "After `$x$`."]);
    assert_eq!(translate_all(source, ko), source.replace("Before.", "가 Before.").replace("After `$x$`.", "가 After `$x$`."));
}

#[test]
fn math_spans_follow_pandoc() {
    let text = "`$code$` a $x_1$ b $$\\sum_i\n i$$ c \\(y\\) d \\[z\\] e \\$5 and $20,000 and $30,000 f \\begin{align}a\\end{align}";
    let found: Vec<&str> = math_spans(text).into_iter().map(|r| &text[r]).collect();
    assert_eq!(found, ["$x_1$", "$$\\sum_i\n i$$", "\\(y\\)", "\\[z\\]", "\\begin{align}a\\end{align}"]);
    assert!(math_spans("$a\n\nb$").is_empty());
}

#[test]
fn heading_attributes_stay_outside_the_title() {
    let source = "# Frequency Modeling {#ChapFrequency-Modeling}\n\n#### Basic Terminology {-}\n\n## Title {#id .cls key=\"a b\"}\n\nSetext {.unnumbered}\n=====\n\n## Sets {a, b}\n";
    assert_eq!(sources(source), ["Frequency Modeling", "Basic Terminology", "Title", "Setext", "Sets {a, b}"]);
    assert_eq!(
        translate_all(source, ko),
        "# 가 Frequency Modeling {#ChapFrequency-Modeling}\n\n#### 가 Basic Terminology {-}\n\n## 가 Title {#id .cls key=\"a b\"}\n\n가 Setext {.unnumbered}\n=====\n\n## 가 Sets {a, b}\n"
    );
}

#[test]
fn chunk_captions_are_literal_strings_and_text_references_are_not() {
    let source = "```{r Fig1, echo=FALSE, fig.cap=\"A *gamma* density\", fig.alt='Its \"shape\"'}\nplot(1)\n```\n\n```{r Fig2, fig.cap='(ref:Fig2)'}\nplot(2)\n```\n";
    let doc = RmarkdownParser::default().parse(source);
    let literal: Vec<(&str, BlockRole)> = doc
        .sections
        .iter()
        .flat_map(|s| &s.blocks)
        .filter(|b| b.translatable)
        .map(|b| (b.segments[0].source.as_str(), b.role.clone()))
        .collect();
    assert_eq!(literal, [("A *gamma* density", BlockRole::Literal), ("Its \"shape\"", BlockRole::Literal)]);
    let out = translate_all(source, |s| format!("\"{s}\"의 '번역'"));
    assert!(out.contains("fig.cap=\"\\\"A *gamma* density\\\"의 '번역'\""), "{out}");
    assert!(out.contains("fig.alt='\"Its \"shape\"\"의 \\'번역\\''"), "{out}");
    assert!(out.contains("fig.cap='(ref:Fig2)'"));
}

#[test]
fn text_reference_label_stays_outside_its_text() {
    let source = "(ref:Fig31) **Likelihood of a $(0,1,0)$ $3$-sample from Bernoulli**\n\n```{r Fig31, echo=FALSE, fig.cap='(ref:Fig31)', out.width='45%' }\nknitr::include_graphics(\"Figures/figure2.1.png\")\n```\n";
    assert_eq!(sources(source), ["**Likelihood of a $(0,1,0)$ $3$-sample from Bernoulli**"]);
    assert!(translate_all(source, ko).starts_with("(ref:Fig31) 가 **Likelihood"));
}

#[test]
fn inline_r_only_paragraphs_and_sentences_are_not_offered() {
    let source = "Determine the value.\n\n`r HideExample('3.3.1', 'Show Example Solution')`\n\n`r SolnBegin()` Since the $pmf$ holds, we know it.\n\nIt holds. `r SolnEnd()`\n";
    assert_eq!(sources(source), ["Determine the value.", "Since the $pmf$ holds, we know it.", "It holds."]);
    assert_eq!(
        translate_all(source, ko),
        "가 Determine the value.\n\n`r HideExample('3.3.1', 'Show Example Solution')`\n\n`r SolnBegin()` 가 Since the $pmf$ holds, we know it.\n\n가 It holds. `r SolnEnd()`\n"
    );
}

#[test]
fn math_only_and_reference_only_sentences_are_not_offered() {
    let source = "Intro:\n\n$$\nd = 0\n$$\n\n<a id=tab:1.5></a>\n\n\\@ref(fig:x).\n\nSee \\@ref(fig:x).\n";
    assert_eq!(sources(source), ["Intro:", "See \\@ref(fig:x)."]);
}

#[test]
fn sentences_never_split_inside_math() {
    assert_eq!(sources("Let $x. Y$ be given. Then stop.\n"), ["Let $x. Y$ be given.", "Then stop."]);
}

/// AggregateLossModels2024.Rmd: the example title is bold across its dots.
#[test]
fn sentences_never_split_inside_strong_emphasis() {
    assert_eq!(
        sources("**Example 7.2.1. Actuarial Exam Question.** An insurer sold policies. Then it paid.\n"),
        ["**Example 7.2.1. Actuarial Exam Question.** An insurer sold policies.", "Then it paid."]
    );
}

#[test]
fn inline_code_captures_offer_only_their_groups() {
    let parser = RmarkdownParser::with_patterns(&[
        r#"^r Hide\w+\(\s*['"][^'"]*['"]\s*,\s*['"](?P<label>[^'"]+)['"]"#,
    ])
    .unwrap();
    let source = "`r HideExample('3.3.1', 'Show Example Solution')`\n\n`r HideRCode('kpdf',\"Show R Code\")`\n\nUse `r HideExample('a', 'Inline')` here.\n";
    assert_eq!(sources_with(&parser, source), ["Show Example Solution", "Show R Code", "Use `r HideExample('a', 'Inline')` here."]);
    let out = translate_with(&parser, source, |s| if s.starts_with("Use") { s.to_string() } else { format!("{s}의 '풀이'") });
    assert_eq!(
        out,
        "`r HideExample('3.3.1', 'Show Example Solution의 \\'풀이\\'')`\n\n`r HideRCode('kpdf',\"Show R Code의 '풀이'\")`\n\nUse `r HideExample('a', 'Inline')` here.\n"
    );
    assert!(sources(source).len() == 1, "no captures without patterns");
}

#[test]
fn front_matter_values_are_literal_and_quoted_as_yaml_needs() {
    let source = "--- \ntitle: \"Loss Data Analytics <br> \\\\newline Second Edition\"\nauthor: \"An open text\"\nsite: bookdown::bookdown_site\ndescription: \"Loss Data Analytics is a text.\n  -  The online version.\"\nsubtitle: Plain value\n---\n\nBody.\n";
    assert_eq!(
        sources(source),
        ["Loss Data Analytics <br> \\\\newline Second Edition", "Loss Data Analytics is a text. -  The online version.", "Plain value", "Body."]
    );
    let out = translate_all(source, |s| if s == "Plain value" { "값: \"평범\"".into() } else { format!("\"{s}\"") });
    assert!(out.contains("title: \"\\\"Loss Data Analytics <br> \\\\newline Second Edition\\\"\""), "{out}");
    assert!(out.contains("subtitle: \"값: \\\"평범\\\"\""), "{out}");
    assert!(out.contains("author: \"An open text\""));
}

#[test]
fn tokens_are_found_outside_math_and_code() {
    let text = "See \\@ref(S:Sec31), (\\#eq:mle), (#tab:x), (ref:Fig1), @billingsley and [@a; @b2-c] [^1] <a id=tab:1.5></a> a@b.com `@code` $@m$ [Table 7.1]{#tab:7.1}.";
    assert_eq!(
        preserved_tokens(text),
        ["(#tab:x)", "(\\#eq:mle)", "(ref:Fig1)", "<a id=tab:1.5></a>", "@billingsley", "[@a; @b2-c]", "[^1]", "\\@ref(S:Sec31)", "]{#tab:7.1}"]
    );
}

#[test]
fn prose_chunks_and_divs_are_markdown() {
    let source = "```{theorem, thm1, name=\"Bayes rule\"}\nThe posterior is *proportional*.\n```\n\n::: {.note}\nA note.\n:::\n";
    assert_eq!(sources(source), ["Bayes rule", "The posterior is *proportional*.", "A note."]);
    let out = translate_all(source, ko);
    assert_eq!(out, "```{theorem, thm1, name=\"가 Bayes rule\"}\n가 The posterior is *proportional*.\n```\n\n::: {.note}\n가 A note.\n:::\n");
}

#[test]
fn part_prefix_stays_outside_the_heading() {
    assert_eq!(translate_all("# (PART) Foundations {-}\n", ko), "# (PART) 가 Foundations {-}\n");
}

#[test]
fn excerpt_offers_prose_and_keeps_chunks() {
    let segs = sources(FREQUENCY);
    assert!(segs.contains(&"Frequency Modeling".to_string()));
    assert!(segs.iter().all(|s| !s.contains("{#S:")));
    assert!(segs.iter().all(|s| !s.contains("knitr::include_graphics")));
    assert!(segs.iter().all(|s| !s.starts_with("(ref:")));
    assert!(segs.iter().any(|s| s.starts_with("In Section \\@ref(S:Sec31)")));
    let doc = RmarkdownParser::default().parse(FREQUENCY);
    for seg in doc.translatable_segments() {
        let (s, b, k) = seg.id.position().unwrap();
        assert!(s < 1000 && b < 1000 && k < 1000);
    }
}

#[test]
fn a_glued_citation_is_written_in_braces() {
    let source = "As @billingsley shows, see @a.";
    assert_eq!(
        brace_glued_citations(source, "@billingsley가 보이듯 @a, @ab를 보십시오."),
        "@{billingsley}가 보이듯 @a, @ab를 보십시오."
    );
    assert_eq!(preserved_tokens("@{billingsley}가"), preserved_tokens("@billingsley"));
}
