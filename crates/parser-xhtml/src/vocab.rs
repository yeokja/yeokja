//! XML vocabularies: which elements of a document hold prose, which are
//! inline markup, and which are never translated. See
//! `docs/superpowers/specs/2026-10-25-xml-vocabulary-profiles-design.md`.
//!
//! The parser and the inline tag codec both ask these questions, so a
//! vocabulary is the one place that says what an element is.

use crate::xml::{Element, Node};

/// An XML vocabulary the parser can read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Vocabulary {
    /// XHTML content documents (EPUB).
    #[default]
    Xhtml,
    /// OpenStax CNXML modules and collections.
    Cnxml,
    /// PreTeXt (and its predecessor MathBook XML).
    Pretext,
}

/// How an inline element travels to the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InlineKind {
    Italic,
    Bold,
    Link,
    /// Any other element with content.
    Other,
}

/// What the runs directly inside a block element are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Paragraph,
    /// A heading of a fixed level (`h1`–`h6`).
    Heading(Option<u8>),
    /// A `title`, whose level follows the sectioning elements around it.
    Title,
    ListItem,
    TableCell,
    BlockQuote,
}

/// The table structure of a vocabulary.
struct Tables {
    table: &'static [&'static str],
    row: &'static [&'static str],
    cell: &'static [&'static str],
    head: &'static [&'static str],
}

mod xhtml {
    pub const BLOCK: &[&str] = &[
        "address",
        "article",
        "aside",
        "blockquote",
        "body",
        "caption",
        "center",
        "dd",
        "details",
        "dialog",
        "dir",
        "div",
        "dl",
        "dt",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "head",
        "header",
        "hgroup",
        "hr",
        "html",
        "legend",
        "li",
        "main",
        "menu",
        "nav",
        "noscript",
        "ol",
        "p",
        "pre",
        "section",
        "summary",
        "table",
        "tbody",
        "td",
        "tfoot",
        "th",
        "thead",
        "tr",
        "ul",
    ];
    pub const SKIP: &[&str] = &[
        "pre", "script", "style", "math", "svg", "object", "iframe", "template", "textarea",
        "select",
    ];
    pub const CODE: &[&str] = &["code", "kbd", "samp", "tt"];
    pub const ATTRS: &[&str] = &["alt", "title"];
    pub const ITALIC: &[&str] = &["em", "i", "cite", "dfn", "var"];
    pub const BOLD: &[&str] = &["strong", "b"];
}

mod cnxml {
    pub const BLOCK: &[&str] = &[
        "caption",
        "collection",
        "commentary",
        "content",
        "definition",
        "document",
        "entry",
        "equation",
        "example",
        "exercise",
        "figure",
        "footnote",
        "glossary",
        "item",
        "label",
        "list",
        "meaning",
        "metadata",
        "note",
        "para",
        "preformat",
        "problem",
        "proof",
        "quote",
        "row",
        "rule",
        "section",
        "seealso",
        "solution",
        "statement",
        "subcollection",
        "subfigure",
        "table",
        "tbody",
        "tfoot",
        "tgroup",
        "thead",
        "title",
    ];
    pub const SKIP: &[&str] = &["math", "preformat", "iframe", "object", "embed"];
    pub const CODE: &[&str] = &["code"];
    /// Calculator keys (`2ND`, `ENTER`): labels printed on a device.
    pub const CODE_CLASSES: &[&str] = &["primary-function", "secondary-function"];
    pub const ATTRS: &[&str] = &["alt", "summary"];
    pub const SECTIONING: &[&str] = &["document", "section", "collection", "subcollection"];
}

mod pretext {
    /// Inline elements; every other element with content is a block.
    pub const INLINE: &[&str] = &[
        // Text markup.
        "abbr",
        "acro",
        "alert",
        "articletitle",
        "attr",
        "booktitle",
        "c",
        "delete",
        "dq",
        "em",
        "email",
        "fillin",
        "fn",
        "foreign",
        "icon",
        "init",
        "insert",
        "kbd",
        "m",
        "personname",
        "pubtitle",
        "q",
        "quantity",
        "sq",
        "stale",
        "sub",
        "sup",
        "tag",
        "tage",
        "taxon",
        "term",
        "url",
        "var",
        "xref",
        // Characters and generated words, usually empty elements.
        "ad",
        "am",
        "ampersand",
        "asterisk",
        "backslash",
        "bc",
        "circa",
        "circumflex",
        "copyleft",
        "copyright",
        "degree",
        "dollar",
        "eg",
        "ellipsis",
        "etc",
        "gt",
        "greater",
        "hash",
        "ie",
        "latex",
        "lbrace",
        "ldq",
        "less",
        "lsq",
        "lt",
        "mdash",
        "midpoint",
        "minus",
        "nb",
        "nbsp",
        "ndash",
        "obelus",
        "percent",
        "permille",
        "phonomark",
        "pilcrow",
        "plusminus",
        "pm",
        "pretext",
        "prime",
        "ps",
        "rbrace",
        "rdq",
        "registered",
        "rsq",
        "section-mark",
        "servicemark",
        "slash",
        "solidus",
        "swungdash",
        "tex",
        "tilde",
        "times",
        "today",
        "trademark",
        "underscore",
        "viz",
        "vs",
        "xelatex",
        "xetex",
        // Conditional text: transparent, kept with what it wraps.
        "restrict-version",
    ];
    pub const SKIP: &[&str] = &[
        "asymptote",
        "cd",
        "console",
        "docinfo",
        "idx",
        "image",
        "input",
        "interactive",
        "latex-code",
        "latex-image-code",
        "macros",
        "mathbox",
        "md",
        "mdn",
        "me",
        "men",
        "output",
        "pre",
        "program",
        "sage",
        "sageplot",
        "usage",
        "video",
    ];
    pub const CODE: &[&str] = &["m", "c"];
    pub const DISPLAY_MATH: &[&str] = &["me", "men", "md", "mdn"];
    pub const ATTRS: &[&str] = &["type-name"];
    pub const ITALIC: &[&str] = &["em", "foreign", "pubtitle", "articletitle", "booktitle"];
    pub const BOLD: &[&str] = &["term", "alert"];
    pub const LINK: &[&str] = &["xref", "url"];
    pub const TITLE: &[&str] = &["title", "subtitle", "shorttitle"];
    pub const SECTIONING: &[&str] = &[
        "acknowledgement",
        "appendix",
        "article",
        "backmatter",
        "biography",
        "book",
        "chapter",
        "colophon",
        "exercises",
        "foreword",
        "frontmatter",
        "glossary",
        "index",
        "paragraphs",
        "part",
        "preface",
        "reading-questions",
        "references",
        "section",
        "solutions",
        "subsection",
        "subsubsection",
        "worksheet",
    ];
}

impl Vocabulary {
    /// The vocabulary a parser name reads.
    pub fn for_parser(name: &str) -> Option<Self> {
        match name {
            "xhtml" => Some(Vocabulary::Xhtml),
            "cnxml" => Some(Vocabulary::Cnxml),
            "pretext" => Some(Vocabulary::Pretext),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Vocabulary::Xhtml => "XHTML",
            Vocabulary::Cnxml => "CNXML",
            Vocabulary::Pretext => "PreTeXt",
        }
    }

    /// Whether `e` is a block element by name. In PreTeXt every element
    /// that is not inline, skipped or code is a block, unless it is empty
    /// (`<nbsp/>`, `<xi:include/>`): an empty element holds no prose, so it
    /// never ends a run.
    fn is_block_name(self, e: &Element) -> bool {
        let name = e.local();
        match self {
            Vocabulary::Xhtml => xhtml::BLOCK.contains(&name),
            Vocabulary::Cnxml => cnxml::BLOCK.contains(&name),
            Vocabulary::Pretext => {
                !(pretext::INLINE.contains(&name)
                    || pretext::SKIP.contains(&name)
                    || pretext::CODE.contains(&name)
                    || e.children.is_empty())
            }
        }
    }

    /// Whether `e` ends a run: a block element, or an inline element that
    /// holds one (PreTeXt's `restrict-version` around whole paragraphs),
    /// which is walked like a block so the block inside it is reached.
    pub fn breaks_run(self, e: &Element, source: &str) -> bool {
        self.is_block_name(e) || (!self.is_untranslated(e, source) && self.holds_block(e, source))
    }

    /// Whether `e` is walked as a block only because it holds one.
    pub fn is_container(self, e: &Element) -> bool {
        !self.is_block_name(e)
    }

    fn holds_block(self, e: &Element, source: &str) -> bool {
        e.children.iter().any(|n| match n {
            Node::Element(c) => {
                self.is_block_name(c)
                    || (!self.is_untranslated(c, source) && self.holds_block(c, source))
            }
            _ => false,
        })
    }

    /// Never translated, wherever it sits: a skipped element or one marked
    /// `translate="no"`.
    pub fn is_untranslated(self, e: &Element, source: &str) -> bool {
        let skip = match self {
            Vocabulary::Xhtml => xhtml::SKIP,
            Vocabulary::Cnxml => cnxml::SKIP,
            Vocabulary::Pretext => pretext::SKIP,
        };
        skip.contains(&e.local()) || e.attr(source, "translate") == Some("no")
    }

    /// The sentence-ending mark a display equation inside a paragraph ends
    /// with (PreTeXt's `me`, `men`, `md`, `mdn` hold the sentence's period).
    pub fn ends_sentence(self, e: &Element, source: &str) -> Option<char> {
        fn last_text<'a>(source: &'a str, nodes: &[Node]) -> Option<&'a str> {
            nodes.iter().rev().find_map(|n| match n {
                Node::Text(r) => Some(source[r.clone()].trim_end()).filter(|t| !t.is_empty()),
                Node::Element(c) => last_text(source, &c.children),
                Node::Other(_) => None,
            })
        }
        if self != Vocabulary::Pretext || !pretext::DISPLAY_MATH.contains(&e.local()) {
            return None;
        }
        last_text(source, &e.children)
            .and_then(|t| t.chars().last())
            .filter(|c| matches!(c, '.' | '?' | '!'))
    }

    /// Inline elements whose text is code: shown to the model, never changed.
    pub fn is_code(self, e: &Element, source: &str) -> bool {
        let name = e.local();
        match self {
            Vocabulary::Xhtml => xhtml::CODE.contains(&name),
            Vocabulary::Cnxml => {
                cnxml::CODE.contains(&name)
                    || (name == "span"
                        && cnxml::CODE_CLASSES.iter().any(|c| e.has_class(source, c)))
            }
            Vocabulary::Pretext => pretext::CODE.contains(&name),
        }
    }

    /// Whether the attribute's value is translated.
    pub fn translates_attr(self, name: &str) -> bool {
        match self {
            Vocabulary::Xhtml => xhtml::ATTRS.contains(&name),
            Vocabulary::Cnxml => cnxml::ATTRS.contains(&name),
            Vocabulary::Pretext => pretext::ATTRS.contains(&name),
        }
    }

    /// A container read only for its title children (XHTML's `head`,
    /// CNXML's `metadata` with its `md:title`): the child's local name.
    pub fn head_title(self, container: &str) -> Option<&'static str> {
        match (self, container) {
            (Vocabulary::Xhtml, "head") | (Vocabulary::Cnxml, "metadata") => Some("title"),
            _ => None,
        }
    }

    /// A skipped element whose descendants of this name hold text to
    /// translate: MathML's `mtext` in CNXML.
    pub fn island(self, skipped: &str) -> Option<&'static str> {
        match (self, skipped) {
            (Vocabulary::Cnxml, "math") => Some("mtext"),
            _ => None,
        }
    }

    pub fn inline_kind(self, e: &Element, source: &str) -> InlineKind {
        let name = e.local();
        let listed = |italic: &[&str], bold: &[&str], link: &[&str]| {
            if italic.contains(&name) {
                InlineKind::Italic
            } else if bold.contains(&name) {
                InlineKind::Bold
            } else if link.contains(&name) {
                InlineKind::Link
            } else {
                InlineKind::Other
            }
        };
        match self {
            Vocabulary::Xhtml => listed(xhtml::ITALIC, xhtml::BOLD, &["a"]),
            Vocabulary::Cnxml => match name {
                // CNXML's default effect is bold.
                "emphasis" => match e.attr(source, "effect").unwrap_or("bold") {
                    "bold" => InlineKind::Bold,
                    "italics" => InlineKind::Italic,
                    _ => InlineKind::Other,
                },
                "term" => InlineKind::Bold,
                "foreign" | "cite-title" => InlineKind::Italic,
                "link" => InlineKind::Link,
                _ => InlineKind::Other,
            },
            Vocabulary::Pretext => listed(pretext::ITALIC, pretext::BOLD, pretext::LINK),
        }
    }

    /// Opaque although it has content: XHTML's footnote reference, a link
    /// whose only content is `<sup class="footnote">`.
    pub fn is_opaque_inline(self, e: &Element, source: &str) -> bool {
        if self != Vocabulary::Xhtml || e.local() != "a" {
            return false;
        }
        let elements: Vec<&Element> = e
            .children
            .iter()
            .filter_map(|n| match n {
                Node::Element(c) => Some(c),
                _ => None,
            })
            .collect();
        elements.len() == 1
            && elements[0].local() == "sup"
            && elements[0].has_class(source, "footnote")
            && e.children
                .iter()
                .all(|n| !matches!(n, Node::Text(r) if !source[r.clone()].trim().is_empty()))
    }

    fn tables(self) -> Tables {
        match self {
            Vocabulary::Xhtml => Tables {
                table: &["table"],
                row: &["tr"],
                cell: &["td", "th"],
                head: &["thead"],
            },
            Vocabulary::Cnxml => Tables {
                table: &["table"],
                row: &["row"],
                cell: &["entry"],
                head: &["thead"],
            },
            Vocabulary::Pretext => Tables {
                table: &["tabular"],
                row: &["row"],
                cell: &["cell"],
                head: &[],
            },
        }
    }

    pub fn is_table(self, name: &str) -> bool {
        self.tables().table.contains(&name)
    }

    pub fn is_row(self, name: &str) -> bool {
        self.tables().row.contains(&name)
    }

    pub fn is_cell(self, name: &str) -> bool {
        self.tables().cell.contains(&name)
    }

    /// A table section whose rows are label rows.
    pub fn is_table_head(self, name: &str) -> bool {
        self.tables().head.contains(&name)
    }

    /// Whether the element counts toward a title's heading level.
    pub fn is_sectioning(self, name: &str) -> bool {
        match self {
            Vocabulary::Xhtml => false,
            Vocabulary::Cnxml => cnxml::SECTIONING.contains(&name),
            Vocabulary::Pretext => pretext::SECTIONING.contains(&name),
        }
    }

    pub fn block_kind(self, name: &str) -> BlockKind {
        if self.is_cell(name) {
            return BlockKind::TableCell;
        }
        match self {
            Vocabulary::Xhtml => match name {
                "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                    BlockKind::Heading(name[1..].parse().ok())
                }
                "li" | "dt" | "dd" => BlockKind::ListItem,
                "blockquote" => BlockKind::BlockQuote,
                _ => BlockKind::Paragraph,
            },
            Vocabulary::Cnxml => match name {
                "title" => BlockKind::Title,
                "item" => BlockKind::ListItem,
                "quote" => BlockKind::BlockQuote,
                _ => BlockKind::Paragraph,
            },
            Vocabulary::Pretext => match name {
                _ if pretext::TITLE.contains(&name) => BlockKind::Title,
                "li" => BlockKind::ListItem,
                "blockquote" => BlockKind::BlockQuote,
                _ => BlockKind::Paragraph,
            },
        }
    }

    /// Emphasis that Korean word order may split in two: the skeleton only
    /// asks that it be there.
    pub fn is_emphasis(self, e: &Element, source: &str) -> bool {
        matches!(
            self.inline_kind(e, source),
            InlineKind::Italic | InlineKind::Bold
        ) && !self.is_code(e, source)
    }
}
