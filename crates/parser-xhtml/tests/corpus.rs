//! Round trip over a directory of real documents, named by
//! `YEOKJA_XHTML_CORPUS` (for example an EPUB's unpacked `OEBPS/`, an
//! OpenStax book's `modules/`, a PreTeXt book's `src/`) and read with the
//! parser `YEOKJA_XHTML_CORPUS_PARSER` names (`xhtml` by default, `cnxml`,
//! `pretext`). Skipped when the variable is unset.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use yeokja_core::model::BlockRole;
use yeokja_core::parser::{DocumentParser, TranslationMap};
use yeokja_parser_xhtml::{CnxmlParser, PretextParser, Vocabulary, XhtmlParser, skeleton_with};

fn documents(dir: &Path, extensions: &[&str], out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            documents(&path, extensions, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| extensions.contains(&e))
        {
            out.push(path);
        }
    }
}

#[test]
fn corpus_round_trip() {
    let Ok(dir) = std::env::var("YEOKJA_XHTML_CORPUS") else {
        return;
    };
    let name = std::env::var("YEOKJA_XHTML_CORPUS_PARSER").unwrap_or_else(|_| "xhtml".into());
    let vocab = Vocabulary::for_parser(&name).unwrap_or_else(|| panic!("unknown parser {name}"));
    let (parser, extensions): (Box<dyn DocumentParser>, &[&str]) = match vocab {
        Vocabulary::Xhtml => (Box::new(XhtmlParser), &["html", "xhtml"]),
        Vocabulary::Cnxml => (Box::new(CnxmlParser), &["cnxml", "xml"]),
        Vocabulary::Pretext => (Box::new(PretextParser), &["xml", "ptx"]),
    };
    let mut files = Vec::new();
    documents(Path::new(&dir), extensions, &mut files);
    files.sort();
    assert!(!files.is_empty(), "no documents in {dir}");
    let mut blocks = 0;
    let mut lengths: Vec<usize> = Vec::new();
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();
    for path in &files {
        let source = std::fs::read_to_string(path).unwrap();
        let doc = parser
            .parse_checked(&source)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let identity: TranslationMap = doc
            .all_segments()
            .iter()
            .map(|s| (s.id.clone(), s.source.clone()))
            .collect();
        let rebuilt = parser.reconstruct(&doc, &identity);
        assert_eq!(rebuilt, source, "{} does not round trip", path.display());
        assert_eq!(
            skeleton_with(vocab, &rebuilt).unwrap(),
            skeleton_with(vocab, &source).unwrap()
        );
        for section in &doc.sections {
            blocks += section.blocks.len();
            assert!(
                section.blocks.len() < 1000,
                "{}: section too large",
                path.display()
            );
            for block in &section.blocks {
                let kind = if block.role == BlockRole::Literal {
                    "Literal".to_string()
                } else {
                    format!("{:?}", block.block_type)
                };
                *kinds.entry(kind).or_insert(0usize) += block.segments.len();
                lengths.extend(block.segments.iter().map(|seg| seg.source.chars().count()));
                // YEOKJA_XHTML_CORPUS_LONG=N shows the segments longer than N chars.
                if let Some(limit) = std::env::var("YEOKJA_XHTML_CORPUS_LONG")
                    .ok()
                    .and_then(|n| n.parse::<usize>().ok())
                {
                    for seg in block
                        .segments
                        .iter()
                        .filter(|s| s.source.chars().count() > limit)
                    {
                        let head: String = seg.source.chars().take(300).collect();
                        eprintln!("{} {}: {head}", path.display(), seg.id);
                    }
                }
            }
        }
    }
    lengths.sort_unstable();
    let at = |q: usize| lengths.get(lengths.len() * q / 100).copied().unwrap_or(0);
    eprintln!(
        "{} documents, {blocks} blocks, {} segments; chars median {}, p90 {}, p99 {}, longest {}",
        files.len(),
        lengths.len(),
        at(50),
        at(90),
        at(99),
        lengths.last().copied().unwrap_or(0)
    );
    eprintln!("segments by block: {kinds:?}");
}
