//! CSV (RFC 4180) as a translation source: one table whose first record
//! names the columns.
//!
//! Each field is a table cell ([`BlockRole::TableCell`]), so `[[tables]]`
//! rules choose the columns to translate (`translate = ["Definition"]`).
//! The header record is never translated — code reads the file by those
//! names — and a field without a letter (a number, an empty field) is not
//! offered. A field's span is the whole field, quotes included, so
//! reconstruction writes it back quoted when the source quoted it or the
//! translation needs quotes (a comma, a quote, a line break), doubling the
//! quotes inside. Everything else — separators, record ends (`\n` or
//! `\r\n`), untouched fields — stays byte for byte.

use std::ops::Range;
use yeokja_core::model::*;
use yeokja_core::parser::{DocumentParser, Markup, TranslationMap};
use yeokja_parser_utils::{apply_splices, join_segments_with_translations, make_segments, normalize_inline_text};

pub struct CsvParser;

/// One field: its bytes (quotes included) and its value.
#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub range: Range<usize>,
    pub value: String,
    pub quoted: bool,
}

/// The records of `source`. A quoted field runs to the quote that a
/// separator, a record end or the end of the input follows; `""` inside it
/// is one quote. A quote inside an unquoted field is an ordinary character.
pub fn records(source: &str) -> Vec<Vec<Field>> {
    let b = source.as_bytes();
    let mut out = Vec::new();
    let mut record = Vec::new();
    let mut i = 0;
    if b.is_empty() {
        return out;
    }
    loop {
        let start = i;
        let (value, quoted) = if b.get(i) == Some(&b'"') {
            let mut value = String::new();
            let mut j = i + 1;
            let mut seg = j;
            loop {
                match b.get(j) {
                    None => break,
                    Some(b'"') if b.get(j + 1) == Some(&b'"') => {
                        value.push_str(&source[seg..j + 1]);
                        j += 2;
                        seg = j;
                    }
                    Some(b'"') => {
                        value.push_str(&source[seg..j]);
                        j += 1;
                        seg = j;
                        break;
                    }
                    Some(_) => j += 1,
                }
            }
            if seg < j {
                value.push_str(&source[seg..j]);
            }
            // Text between the closing quote and the separator is kept.
            while j < b.len() && b[j] != b',' && b[j] != b'\n' && !(b[j] == b'\r' && b.get(j + 1) == Some(&b'\n')) {
                j += 1;
            }
            i = j;
            (value, true)
        } else {
            while i < b.len() && b[i] != b',' && b[i] != b'\n' && !(b[i] == b'\r' && b.get(i + 1) == Some(&b'\n')) {
                i += 1;
            }
            (source[start..i].to_string(), false)
        };
        record.push(Field { range: start..i, value, quoted });
        match b.get(i) {
            Some(b',') => i += 1,
            Some(b'\r') => {
                i += 2;
                out.push(std::mem::take(&mut record));
                if i >= b.len() {
                    break;
                }
            }
            Some(b'\n') => {
                i += 1;
                out.push(std::mem::take(&mut record));
                if i >= b.len() {
                    break;
                }
            }
            _ => {
                out.push(std::mem::take(&mut record));
                break;
            }
        }
    }
    out
}

/// Blocks per section stay under the segment id's thousand.
const BLOCKS_PER_SECTION: usize = 900;

impl DocumentParser for CsvParser {
    fn markup(&self) -> Markup {
        Markup::Plain
    }

    fn parse(&self, source: &str) -> Document {
        let records = records(source);
        let Some((header, body)) = records.split_first() else {
            return Document { sections: Vec::new(), source: source.to_string() };
        };
        let names: Vec<String> = header.iter().map(|f| f.value.trim().to_string()).collect();
        // Every section repeats the header cells, untranslated and without a
        // span, so a table rule finds the column names in each.
        let label_row = || -> Vec<Block> {
            names
                .iter()
                .enumerate()
                .map(|(column, name)| Block {
                    block_type: BlockType::Table,
                    segments: Vec::new(),
                    raw_content: name.clone(),
                    heading_level: None,
                    span: None,
                    role: BlockRole::TableCell { table: 0, column, label_row: true, header: None },
                    translatable: false,
                })
                .collect()
        };
        let mut sections = vec![Section { blocks: label_row() }];
        for record in body {
            let cells: Vec<&Field> = record.iter().filter(|f| f.value.chars().any(char::is_alphabetic)).collect();
            if sections.last().unwrap().blocks.len() + cells.len() > BLOCKS_PER_SECTION {
                sections.push(Section { blocks: label_row() });
            }
            let section_idx = sections.len() - 1;
            let section = sections.last_mut().unwrap();
            for field in cells {
                let column = record.iter().position(|f| f.range == field.range).unwrap_or(0);
                let block_idx = section.blocks.len();
                let text = normalize_inline_text(&field.value);
                section.blocks.push(Block {
                    block_type: BlockType::Table,
                    segments: make_segments(&text, BlockType::Table, section_idx, block_idx),
                    raw_content: field.value.clone(),
                    heading_level: None,
                    span: Some(field.range.clone()),
                    role: BlockRole::TableCell {
                        table: 0,
                        column,
                        label_row: false,
                        header: names.get(column).filter(|n| !n.is_empty()).cloned(),
                    },
                    translatable: true,
                });
            }
        }
        Document { sections, source: source.to_string() }
    }

    fn reconstruct(&self, document: &Document, translations: &TranslationMap) -> String {
        let source = &document.source;
        let mut splices = Vec::new();
        for block in document.sections.iter().flat_map(|s| &s.blocks) {
            let Some(span) = &block.span else { continue };
            if !block.translatable || !block.segments.iter().any(|seg| translations.contains_key(&seg.id)) {
                continue;
            }
            let text = join_segments_with_translations(&block.segments, translations);
            let quoted = source.as_bytes()[span.start] == b'"';
            let needs = text.contains([',', '"', '\n', '\r']);
            let field = if quoted || needs { format!("\"{}\"", text.replace('"', "\"\"")) } else { text };
            splices.push((span.clone(), field));
        }
        apply_splices(source, splices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GLOSS: &str = include_str!("../tests/fixtures/GlossFct.csv");

    fn values(source: &str) -> Vec<Vec<String>> {
        records(source).into_iter().map(|r| r.into_iter().map(|f| f.value).collect()).collect()
    }

    #[test]
    fn rfc4180_quotes_separators_and_line_breaks() {
        let source = "a,b\r\n\"x, y\",\"say \"\"hi\"\"\"\r\n\"two\nlines\",\n";
        assert_eq!(values(source), [vec!["a", "b"], vec!["x, y", "say \"hi\""], vec!["two\nlines", ""]]);
    }

    #[test]
    fn glossary_parses_into_four_columns() {
        let rows = records(GLOSS);
        assert_eq!(rows.len(), 507);
        assert!(rows.iter().all(|r| r.len() == 4));
        assert_eq!(values(GLOSS)[0], ["Term", "Definition", "Section", ""]);
    }

    #[test]
    fn untranslated_round_trip_is_byte_identical() {
        let doc = CsvParser.parse(GLOSS);
        assert_eq!(CsvParser.reconstruct(&doc, &TranslationMap::new()), GLOSS);
    }

    #[test]
    fn header_is_never_offered_and_numbers_are_skipped() {
        let doc = CsvParser.parse(GLOSS);
        let segs = doc.translatable_segments();
        assert!(segs.iter().all(|s| s.source != "Term" && s.source != "1.1"));
        for seg in &segs {
            let (s, b, k) = seg.id.position().unwrap();
            assert!(s < 1000 && b < 1000 && k < 1000);
        }
    }

    #[test]
    fn a_table_rule_selects_the_definition_column() {
        let mut doc = CsvParser.parse(GLOSS);
        let rule: yeokja_core::config::TableRule =
            toml_rule(r#"headers = ["Term", "Definition"]
translate = ["Definition"]"#);
        yeokja_core::select::apply_table_rules(&mut doc, &[rule], std::path::Path::new("GlossFct.csv"));
        let segs: Vec<String> = doc.translatable_segments().iter().map(|s| s.source.clone()).collect();
        assert!(segs.contains(&"Analytics is the process of using data to make decisions.".to_string()));
        assert!(!segs.contains(&"analytics".to_string()));
        assert!(!segs.contains(&"renters insurance".to_string()));
    }

    fn toml_rule(body: &str) -> yeokja_core::config::TableRule {
        let config = yeokja_core::config::ProjectConfig::from_toml(&format!(
            "[project]\nsource_lang = \"en\"\ntarget_lang = \"ko\"\n[provider]\ntype = \"x\"\nmodel = \"y\"\n[[tables]]\n{body}\n"
        ))
        .unwrap();
        config.tables[0].clone()
    }

    #[test]
    fn translations_are_quoted_when_they_need_it() {
        let source = "Term,Definition,Section\nloss,Plain text,1.1\nclaim,\"Quoted, text\",1.2\n";
        let doc = CsvParser.parse(source);
        let mut map = TranslationMap::new();
        for seg in doc.translatable_segments() {
            let t = match seg.source.as_str() {
                "Plain text" => "평문, \"인용\"",
                "Quoted, text" => "인용된 글",
                other => other,
            };
            map.insert(seg.id.clone(), t.to_string());
        }
        assert_eq!(
            CsvParser.reconstruct(&doc, &map),
            "Term,Definition,Section\nloss,\"평문, \"\"인용\"\"\",1.1\nclaim,\"인용된 글\",1.2\n"
        );
    }
}
