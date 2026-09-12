use yeokja_core::parser::{DocumentParser, TranslationMap};
use yeokja_parser_markdeep::MarkdeepParser;
fn main() {
    for path in std::env::args().skip(1) {
        let source = std::fs::read_to_string(&path).unwrap();
        let doc = MarkdeepParser.parse(&source);
        assert_eq!(
            MarkdeepParser.reconstruct(&doc, &TranslationMap::new()),
            source
        );
        let mut spans: Vec<_> = doc
            .sections
            .iter()
            .flat_map(|s| &s.blocks)
            .filter_map(|b| b.span.clone())
            .collect();
        spans.sort_by_key(|r| r.start);
        for pair in spans.windows(2) {
            assert!(
                pair[0].end <= pair[1].start,
                "overlapping translation spans in {path}"
            );
        }
        let mut offset = 0;
        let mut covered_lines = 0;
        let mut opaque_lines = 0;
        let mut code = false;
        let mut equation = false;
        println!("{path}: {} segments", doc.translatable_segments().len());
        for (n, line) in source.split_inclusive('\n').enumerate() {
            let trimmed = line.trim();
            let fence = trimmed.bytes().take_while(|b| *b == b'~').count();
            let ignored = code || fence >= 3 || equation || trimmed.starts_with("$$");
            if fence >= 3 {
                code = !(code && trimmed[fence..].trim().is_empty());
            }
            if !code && trimmed.matches("$$").count() % 2 == 1 {
                equation = !equation;
            }
            let covered = spans
                .iter()
                .any(|r| r.start < offset + line.len() && offset < r.end);
            if ignored {
                assert!(
                    !covered,
                    "code or display math offered for translation at {path}:{}",
                    n + 1
                );
                opaque_lines += 1;
            } else if covered {
                covered_lines += 1;
            } else if trimmed.chars().any(|c| c.is_ascii_alphabetic()) {
                let syntax = trimmed.starts_with('<')
                    || trimmed.starts_with("](")
                    || (trimmed.starts_with('[') && trimmed.contains("]: "))
                    || (trimmed.starts_with("(insert ") && trimmed.ends_with(" here)"));
                assert!(syntax, "uncovered prose at {path}:{}: {trimmed}", n + 1);
            }
            offset += line.len();
        }
        println!(
            "  {covered_lines} prose lines covered; {opaque_lines} code/math lines protected; no overlapping spans or uncovered prose lines"
        );
    }
}
