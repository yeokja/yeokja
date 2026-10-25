//! CNXML and PreTeXt read from excerpts of real books (`tests/fixtures`):
//! OpenStax *Principles of Finance* and *Interactive Linear Algebra*.

use std::path::Path;
use yeokja_core::model::{Block, BlockRole, BlockType, Document};
use yeokja_core::parser::{DocumentParser, TranslationMap};
use yeokja_parser_xhtml::xml::{self, Node};
use yeokja_parser_xhtml::{CnxmlParser, PretextParser, Vocabulary, skeleton_with};

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn parser(vocab: Vocabulary) -> Box<dyn DocumentParser> {
    match vocab {
        Vocabulary::Cnxml => Box::new(CnxmlParser),
        Vocabulary::Pretext => Box::new(PretextParser),
        Vocabulary::Xhtml => unreachable!(),
    }
}

fn parse(vocab: Vocabulary, name: &str) -> (String, Document) {
    let source = fixture(name);
    let doc = parser(vocab).parse_checked(&source).unwrap();
    (source, doc)
}

fn blocks(doc: &Document) -> Vec<&Block> {
    doc.sections.iter().flat_map(|s| &s.blocks).collect()
}

fn sources(doc: &Document) -> Vec<String> {
    doc.all_segments()
        .iter()
        .map(|s| s.source.clone())
        .collect()
}

fn block_of<'a>(doc: &'a Document, source: &str) -> &'a Block {
    blocks(doc)
        .into_iter()
        .find(|b| b.segments.iter().any(|s| s.source == source))
        .unwrap_or_else(|| panic!("no segment {source:?}"))
}

/// A stand-in translation of a segment: every text node with a letter
/// becomes Korean; markup, code and skipped elements stay.
fn koreanize(vocab: Vocabulary, source: &str, literal: bool) -> String {
    fn nodes(vocab: Vocabulary, source: &str, list: &[Node], out: &mut String) {
        for node in list {
            match node {
                Node::Text(r) => {
                    let text = &source[r.clone()];
                    if text.chars().any(char::is_alphabetic) {
                        let lead = &text[..text.len() - text.trim_start().len()];
                        let trail = &text[text.trim_end().len()..];
                        out.push_str(&format!("{lead}번역{trail}"));
                    } else {
                        out.push_str(text);
                    }
                }
                Node::Other(r) => out.push_str(&source[r.clone()]),
                Node::Element(e)
                    if e.end.is_none()
                        || vocab.is_untranslated(e, source)
                        || vocab.is_code(e, source) =>
                {
                    out.push_str(&source[e.range()])
                }
                Node::Element(e) => {
                    out.push_str(&source[e.start.clone()]);
                    nodes(vocab, source, &e.children, out);
                    out.push_str(&source[e.end.clone().unwrap()]);
                }
            }
        }
    }
    if literal {
        return "번역".to_string();
    }
    let mut out = String::new();
    nodes(vocab, source, &xml::parse(source).unwrap(), &mut out);
    out
}

/// Every fixture: an identity translation rebuilds the source byte for
/// byte, and a stand-in Korean translation keeps its skeleton.
fn check_round_trips(vocab: Vocabulary, name: &str) -> usize {
    let (source, doc) = parse(vocab, name);
    let parser = parser(vocab);
    let identity: TranslationMap = doc
        .all_segments()
        .iter()
        .map(|s| (s.id.clone(), s.source.clone()))
        .collect();
    assert_eq!(
        parser.reconstruct(&doc, &identity),
        source,
        "{name}: identity"
    );
    assert_eq!(
        parser.reconstruct(&doc, &TranslationMap::new()),
        source,
        "{name}: empty"
    );
    let mut korean = TranslationMap::new();
    for block in blocks(&doc) {
        for seg in &block.segments {
            korean.insert(
                seg.id.clone(),
                koreanize(vocab, &seg.source, block.role == BlockRole::Literal),
            );
        }
    }
    let translated = parser.reconstruct(&doc, &korean);
    assert_ne!(translated, source);
    assert_eq!(
        skeleton_with(vocab, &translated).unwrap(),
        skeleton_with(vocab, &source).unwrap(),
        "{name}: skeleton"
    );
    // The translation reads back with as many blocks.
    let again = parser.parse_checked(&translated).unwrap();
    assert_eq!(
        blocks(&again).len(),
        blocks(&doc).len(),
        "{name}: blocks after translation"
    );
    doc.all_segments().len()
}

#[test]
fn cnxml_fixtures_round_trip() {
    let counts: Vec<usize> = [
        "cnxml/m00002.cnxml",
        "cnxml/m00011.cnxml",
        "cnxml/m00026.cnxml",
        "cnxml/m00050.cnxml",
        "cnxml/collection.xml",
    ]
    .iter()
    .map(|name| check_round_trips(Vocabulary::Cnxml, name))
    .collect();
    assert_eq!(counts, [73, 25, 75, 33, 2]);
}

#[test]
fn pretext_fixtures_round_trip() {
    let counts: Vec<usize> = [
        "pretext/ila.xml",
        "pretext/vectors.xml",
        "pretext/cplx-eigenvals.xml",
    ]
    .iter()
    .map(|name| check_round_trips(Vocabulary::Pretext, name))
    .collect();
    assert_eq!(counts, [32, 38, 20]);
}

fn literals(doc: &Document) -> Vec<String> {
    blocks(doc)
        .into_iter()
        .filter(|b| b.role == BlockRole::Literal)
        .map(|b| b.segments[0].source.clone())
        .collect()
}

/// Translate the segments `pairs` names; the rest stay untranslated.
fn translate(vocab: Vocabulary, source: &str, doc: &Document, pairs: &[(&str, &str)]) -> String {
    let mut t = TranslationMap::new();
    for seg in doc.all_segments() {
        if let Some((_, ko)) = pairs.iter().find(|(en, _)| *en == seg.source) {
            t.insert(seg.id.clone(), ko.to_string());
        }
    }
    assert_eq!(t.len(), pairs.len(), "every pair names a segment");
    let out = parser(vocab).reconstruct(doc, &t);
    assert_eq!(
        skeleton_with(vocab, &out).unwrap(),
        skeleton_with(vocab, source).unwrap()
    );
    out
}

#[test]
fn cnxml_structure_titles_and_footnotes() {
    let (_, doc) = parse(Vocabulary::Cnxml, "cnxml/m00002.cnxml");
    let b = blocks(&doc);
    // The module title and `md:title`; the rest of the metadata is not prose.
    assert_eq!(
        (b[0].segments[0].source.as_str(), b[0].heading_level),
        ("What Is Finance?", Some(1))
    );
    assert_eq!(
        (
            b[1].segments[0].source.as_str(),
            b[1].block_type,
            b[1].heading_level
        ),
        ("What Is Finance?", BlockType::Heading, None)
    );
    let all = sources(&doc);
    assert!(
        !all.iter()
            .any(|s| s.contains("m00002") || s.contains("562a910a") || s.contains("os-embed"))
    );
    assert_eq!(block_of(&doc, "Learning Outcomes").heading_level, Some(2));
    assert_eq!(
        block_of(
            &doc,
            "Why Climate Change Means New Risk for US Financial Markets"
        )
        .heading_level,
        Some(3)
    );
    // A note's title sits below the section's.
    assert_eq!(block_of(&doc, "The Great Recession").heading_level, Some(3));
    assert_eq!(
        block_of(&doc, "Describe the main areas in finance.").block_type,
        BlockType::ListItem
    );
    // A footnote is a block of its own; the paragraph goes on after it.
    let after = block_of(
        &doc,
        "He was uniquely qualified to lead the economic response to the crisis.",
    );
    assert_eq!(
        after.segments[0].source,
        "He was uniquely qualified to lead the economic response to the crisis."
    );
    assert!(
        block_of(&doc, "Brookings Institution. “Ben S.")
            .segments
            .len()
            > 1
    );
    // Glossary entries: the term, then its meaning.
    assert!(all.contains(&"<term>business finance</term>".to_string()));
    assert_eq!(literals(&doc), ["climate-change"]);
}

#[test]
fn cnxml_table_rows_and_cells() {
    let (_, doc) = parse(Vocabulary::Cnxml, "cnxml/m00011.cnxml");
    let cell = |text: &str| match block_of(&doc, text).role {
        BlockRole::TableCell {
            table,
            column,
            label_row,
            ..
        } => (table, column, label_row),
        ref other => panic!("{text}: {other:?}"),
    };
    // Both rows of `thead` are label rows.
    assert_eq!(cell("Limited Liability Partnership"), (0, 1, true));
    assert_eq!(cell("Advantages"), (0, 0, true));
    assert_eq!(cell("Only certain professions eligible"), (0, 3, false));
    assert_eq!(
        block_of(&doc, "Only certain professions eligible").block_type,
        BlockType::Table
    );
    // `summary=" "` holds no words.
    assert!(literals(&doc).is_empty());
}

#[test]
fn cnxml_mtext_is_translated_inside_opaque_math() {
    let (source, doc) = parse(Vocabulary::Cnxml, "cnxml/m00026.cnxml");
    // `mtext` only: `mi` names are math.
    let found = literals(&doc);
    assert!(
        found[0].starts_with("Chris’ Landscaping, Income Statement"),
        "a figure's alt text first"
    );
    assert_eq!(found[1..], ["Assets", "Liabilities", "Equity"]);
    let out = translate(
        Vocabulary::Cnxml,
        &source,
        &doc,
        &[
            ("Assets", "자산"),
            ("Liabilities", "부채"),
            ("Equity", "자본"),
        ],
    );
    assert!(
        out.contains("<math xmlns=\"http://www.w3.org/1998/Math/MathML\"><mtext>자산</mtext><mo>"),
        "{out}"
    );
    assert!(out.contains("<mtext>부채</mtext>") && out.contains("<mtext>자본</mtext>"));
    assert!(out.contains("<mi>Revenue</mi>"));
}

#[test]
fn cnxml_mtext_inside_a_translated_run() {
    let (source, doc) = parse(Vocabulary::Cnxml, "cnxml/m00050.cnxml");
    assert_eq!(literals(&doc), ["annual interest", "months in a year"]);
    let run = sources(&doc)
        .into_iter()
        .find(|s| s.starts_with("Note that we represent"))
        .unwrap();
    let korean = run.replace("Note that we represent", "기간당 이자를 나타내는");
    let out = translate(
        Vocabulary::Cnxml,
        &source,
        &doc,
        &[
            (&run, &korean),
            ("annual interest", "연이율"),
            ("months in a year", "1년의 개월 수"),
        ],
    );
    assert!(out.contains("기간당 이자를 나타내는 <emphasis effect=\"italics\" xmlns=\"http://cnx.rice.edu/cnxml\">r </emphasis>"));
    assert!(
        out.contains("<mtext>연이율</mtext></mrow><mrow><mn>12</mn>"),
        "{out}"
    );
    assert!(out.contains("<mtext>1년의 개월 수</mtext>"));
    // Untranslated run, translated `mtext`: the `math` alone changes.
    let out = translate(
        Vocabulary::Cnxml,
        &source,
        &doc,
        &[("annual interest", "연이율")],
    );
    assert!(out.contains("Note that we represent") && out.contains("<mtext>연이율</mtext>"));
}

#[test]
fn cnxml_collection_titles() {
    let (_, doc) = parse(Vocabulary::Cnxml, "cnxml/collection.xml");
    assert_eq!(
        sources(&doc),
        ["Principles of Finance 2e", "Introduction to Finance"]
    );
    assert_eq!(blocks(&doc)[1].heading_level, Some(2));
}

#[test]
fn pretext_titles_type_names_and_lists_in_paragraphs() {
    let (source, doc) = parse(Vocabulary::Pretext, "pretext/ila.xml");
    let b = blocks(&doc);
    // Both versions' titles, though `restrict-version` wraps them.
    assert_eq!(b[0].segments[0].source, "Interactive Linear Algebra");
    assert_eq!(b[1].segments[0].source, "Interactive Linear Algebra [1553]");
    assert_eq!(
        (b[0].block_type, b[0].heading_level),
        (BlockType::Heading, Some(1))
    );
    assert_eq!(
        block_of(&doc, "Systems of Linear Equations: Algebra").heading_level,
        Some(2)
    );
    assert_eq!(literals(&doc), ["Primary Goal", "Primary Goals"]);
    let out = translate(
        Vocabulary::Pretext,
        &source,
        &doc,
        &[("Primary Goal", "핵심 목표")],
    );
    assert!(out.contains("<note type-name=\"핵심 목표\">"));
    // A list inside a paragraph ends the run; the text after it is a run too.
    assert!(sources(&doc).contains(&"We will study two related questions:".to_string()));
    assert_eq!(
        block_of(&doc, "What is the set of solutions to <m>Ax=b</m>?").block_type,
        BlockType::ListItem
    );
    let after = block_of(
        &doc,
        "This question is more subtle at first glance, but you can solve it in the same way as the first question, with the quadratic formula.",
    );
    assert!(
        after.segments[0]
            .source
            .starts_with("The first question is the kind")
    );
}

#[test]
fn pretext_display_math_stays_in_its_sentence() {
    let (_, doc) = parse(Vocabulary::Pretext, "pretext/ila.xml");
    let form = sources(&doc)
        .into_iter()
        .find(|s| s.starts_with("We will learn a systematic way"))
        .unwrap();
    assert!(
        form.contains("of the form\n        <me>") && form.ends_with("</me>"),
        "{form}"
    );

    let (_, doc) = parse(Vocabulary::Pretext, "pretext/vectors.xml");
    let all = sources(&doc);
    // The equation ends its sentence with the period it holds.
    assert!(all.contains(&"When we think of a point in <m>\\R^n</m> as a vector, we will usually write it vertically, like a matrix with one column:\n    <me>v = \\vec{1 3}.</me>".to_string()));
    assert!(all.contains(&"We will also write <m>0</m> for the zero vector.".to_string()));
    // TikZ in a paragraph rides along, opaque.
    assert!(all.iter().any(
        |s| s.starts_with("For instance, these arrows all represent the vector")
            && s.contains("<latex-code>")
    ));
    // Index entries and notation usage are not prose.
    assert!(!all.iter().any(|s| s == "Point"
        || s == "definition of"
        || s.contains("\\choose 2}") && !s.contains("<m>")));
    assert!(all.contains(&"A vector".to_string()));
    let li = block_of(
        &doc,
        "We can add two vectors together:\n          <me>\\vec{ a b c} + \\vec{ x y z} = \\vec{ a+x b+y c+z}.</me>\n          <idx><h>Vector</h><h>addition</h></idx>",
    );
    assert_eq!(li.block_type, BlockType::ListItem);
}

#[test]
fn pretext_restrict_version_is_transparent() {
    let (_, doc) = parse(Vocabulary::Pretext, "pretext/cplx-eigenvals.xml");
    let all = sources(&doc);
    assert_eq!(block_of(&doc, "Learn to recognize a rotation-scaling matrix, and compute by how much the matrix rotates and scales.").block_type, BlockType::ListItem);
    assert!(all.contains(
        &"See <xref ref=\"complex-numbers\"/> for a review of the complex numbers.".to_string()
    ));
    assert_eq!(
        block_of(&doc, "Matrices with Complex Eigenvalues").heading_level,
        Some(2)
    );
}
