//! Round trip over a directory of real XHTML documents, named by
//! `YEOKJA_XHTML_CORPUS` (for example an EPUB's unpacked `OEBPS/`). Skipped
//! when the variable is unset.

use yeokja_core::parser::{DocumentParser, TranslationMap};
use yeokja_parser_xhtml::{XhtmlParser, skeleton};

#[test]
fn corpus_round_trip() {
    let Ok(dir) = std::env::var("YEOKJA_XHTML_CORPUS") else {
        return;
    };
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "html" || e == "xhtml"))
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no documents in {dir}");
    let (mut blocks, mut segments, mut longest) = (0, 0, 0);
    for path in &files {
        let source = std::fs::read_to_string(path).unwrap();
        let doc = XhtmlParser
            .parse_checked(&source)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let identity: TranslationMap = doc
            .all_segments()
            .iter()
            .map(|s| (s.id.clone(), s.source.clone()))
            .collect();
        let rebuilt = XhtmlParser.reconstruct(&doc, &identity);
        assert_eq!(rebuilt, source, "{} does not round trip", path.display());
        assert_eq!(skeleton(&rebuilt).unwrap(), skeleton(&source).unwrap());
        for section in &doc.sections {
            blocks += section.blocks.len();
            assert!(
                section.blocks.len() < 1000,
                "{}: section too large",
                path.display()
            );
            for block in &section.blocks {
                for seg in &block.segments {
                    segments += 1;
                    longest = longest.max(seg.source.chars().count());
                    assert!(
                        !seg.source.contains('`'),
                        "{}: backtick in {}",
                        path.display(),
                        seg.id
                    );
                }
            }
        }
    }
    eprintln!(
        "{} documents, {blocks} blocks, {segments} segments, longest {longest} chars",
        files.len()
    );
}
