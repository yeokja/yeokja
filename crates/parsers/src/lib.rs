//! Registry of available document parsers.
//!
//! Central place where a file is mapped to its parser, shared by the CLI and
//! the server so both always agree on parser selection.

use std::path::Path;
use yeokja_core::config::ProjectConfig;
use yeokja_core::parser::DocumentParser;

/// Select the parser for a file, preferring the source config whose `path` and
/// `pattern` both match, and falling back to file-extension detection.
pub fn select_parser(file_path: &Path, config: &ProjectConfig) -> Box<dyn DocumentParser> {
    let file_path = file_path.strip_prefix(".").unwrap_or(file_path);
    if let Some(source) = config.source_for(file_path) {
        return parser_by_name(&source.parser, file_path, source.parser_manifest.as_deref());
    }
    match file_path.extension().and_then(|e| e.to_str()) {
        Some("adoc" | "asciidoc" | "asc") => Box::new(yeokja_parser_asciidoc::AsciidocParser),
        Some("rst" | "rest") => Box::new(yeokja_parser_rst::RstParser),
        Some("tex" | "latex") => Box::new(yeokja_parser_latex::LatexParser),
        // A Lean manual must never be interpreted as Markdown just because its
        // source rule is missing. With no manifest this parser reports a hard,
        // actionable error from `parse_checked`.
        Some("lean") => Box::new(yeokja_parser_verso::VersoParser::new(file_path, "")),
        Some("mdx") => Box::new(yeokja_parser_mdx::MdxParser),
        // `.html` may be HTML5, so it is read as XHTML only when a source says so.
        Some("xhtml") => Box::new(yeokja_parser_xhtml::XhtmlParser),
        _ => Box::new(yeokja_parser_markdown::MarkdownParser),
    }
}

fn parser_by_name(
    name: &str,
    file_path: &Path,
    parser_manifest: Option<&str>,
) -> Box<dyn DocumentParser> {
    match name {
        "asciidoc" => Box::new(yeokja_parser_asciidoc::AsciidocParser),
        "markdeep" => Box::new(yeokja_parser_markdeep::MarkdeepParser),
        "mdx" => Box::new(yeokja_parser_mdx::MdxParser),
        "rst" => Box::new(yeokja_parser_rst::RstParser),
        "pep" => Box::new(yeokja_parser_rst::PepParser),
        "pep_plaintext" => Box::new(yeokja_parser_rst::PepPlaintextParser),
        "pep_text_block" => Box::new(yeokja_parser_rst::PepTextBlockParser),
        "mil" | "mathematics_in_lean" => Box::new(yeokja_parser_rst::MilParser),
        "latex" | "tex" => Box::new(yeokja_parser_latex::LatexParser),
        "latex-extended" => Box::new(yeokja_parser_latex::ExtendedLatexParser),
        "verso" => Box::new(yeokja_parser_verso::VersoParser::new(
            file_path,
            parser_manifest.unwrap_or_default(),
        )),
        "myst" => Box::new(yeokja_parser_myst::MystParser),
        "mkdocs" => Box::new(yeokja_parser_mkdocs::MkdocsParser),
        "xhtml" => Box::new(yeokja_parser_xhtml::XhtmlParser),
        // `ProjectConfig::from_toml` rejects names outside `PARSER_NAMES`, so
        // this arm is only reached for "markdown".
        _ => Box::new(yeokja_parser_markdown::MarkdownParser),
    }
}

#[cfg(test)]
mod registry_tests {
    use super::*;
    use yeokja_core::config::PARSER_NAMES;
    use yeokja_core::parser::Markup;

    /// Every advertised parser name must resolve to its own parser, so the
    /// config validation list and this registry cannot drift apart: only the
    /// CommonMark-family names may come back as plain Markdown markup.
    #[test]
    fn every_known_parser_name_has_a_registry_arm() {
        let probe = Path::new("doc.unknown-ext");
        for name in PARSER_NAMES {
            let markup = parser_by_name(name, probe, Some("")).markup();
            let markdown_family = matches!(*name, "markdown" | "myst" | "mdx" | "markdeep");
            assert_eq!(
                markup == Markup::Markdown,
                markdown_family,
                "{name} resolved to {markup:?}"
            );
        }
    }
}
