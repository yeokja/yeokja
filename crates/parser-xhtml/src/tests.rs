use super::*;

fn wrap(body: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" ?>\n<!DOCTYPE html PUBLIC \"-//W3C//DTD XHTML 1.1//EN\" \"x\">\n<html xmlns=\"http://www.w3.org/1999/xhtml\" xml:lang=\"en\">\n<head>\n<title>Doc Title</title>\n</head>\n<body>\n{body}\n</body>\n</html>\n"
    )
}

fn segments(source: &str) -> Vec<String> {
    XhtmlParser
        .parse_checked(source)
        .unwrap()
        .all_segments()
        .iter()
        .map(|s| s.source.clone())
        .collect()
}

fn blocks(source: &str) -> Vec<Block> {
    XhtmlParser
        .parse_checked(source)
        .unwrap()
        .sections
        .into_iter()
        .flat_map(|s| s.blocks)
        .collect()
}

#[test]
fn title_and_paragraphs() {
    let src = wrap("<h1 id=\"a\">Heading</h1>\n<p class=\"indent\">One. Two.</p>");
    assert_eq!(segments(&src), vec!["Doc Title", "Heading", "One.", "Two."]);
    let b = blocks(&src);
    assert_eq!(b[1].block_type, BlockType::Heading);
    assert_eq!(b[1].heading_level, Some(1));
}

#[test]
fn mixed_content_runs_end_at_block_children() {
    let src = wrap(
        "<ul><li>Item text<ul><li>Sub item</li></ul></li></ul><div class=\"textbox\">Loose text<br />more<p>Para</p></div>",
    );
    assert_eq!(
        segments(&src)[1..],
        ["Item text", "Sub item", "Loose text<br />more", "Para"]
    );
    let b = blocks(&src);
    assert_eq!(b[1].block_type, BlockType::ListItem);
    assert_eq!(b[3].block_type, BlockType::Paragraph);
}

#[test]
fn no_split_inside_open_inline_elements() {
    let src = wrap("<p><em>Stop. Go.</em> After <strong>this. And</strong> that. Next one.</p>");
    assert_eq!(
        segments(&src)[1..],
        [
            "<em>Stop. Go.</em>",
            "After <strong>this. And</strong> that.",
            "Next one."
        ]
    );
}

#[test]
fn split_after_closing_inline_element() {
    let src = wrap("<p>First <em>ends here.</em> Second <a href=\"x.html\">link</a>.</p>");
    assert_eq!(
        segments(&src)[1..],
        [
            "First <em>ends here.</em>",
            "Second <a href=\"x.html\">link</a>."
        ]
    );
}

#[test]
fn empty_and_code_only_runs_are_skipped() {
    let src = wrap(
        "<p class=\"indent\">&nbsp;</p><p><code>x.y()</code></p><pre>Code here.</pre><p>Real.</p>",
    );
    assert_eq!(segments(&src)[1..], ["Real."]);
}

#[test]
fn translate_no_is_skipped() {
    let src = wrap("<p translate=\"no\">Keep me.</p><p>Yes.</p>");
    assert_eq!(segments(&src)[1..], ["Yes."]);
}

#[test]
fn attributes_are_literal_blocks() {
    let src = wrap(
        "<p>See <img src=\"a.png\" alt=\"A &quot;nice&quot; chart\" /> here.</p><div class=\"wp-caption\" title=\"Tip\"><img src=\"b.png\" alt=\"\" /></div>",
    );
    let b = blocks(&src);
    let literal: Vec<&str> = b
        .iter()
        .filter(|b| b.role == BlockRole::Literal)
        .map(|b| b.segments[0].source.as_str())
        .collect();
    assert_eq!(literal, vec!["A &quot;nice&quot; chart", "Tip"]);
}

#[test]
fn table_cells_record_position() {
    let src = wrap(
        "<table><tbody><tr><td>H1</td><td>H2</td></tr><tr><td>a</td><td>b</td></tr></tbody></table>",
    );
    let b = blocks(&src);
    assert!(matches!(
        b[2].role,
        BlockRole::TableCell {
            table: 0,
            column: 1,
            label_row: true,
            ..
        }
    ));
    assert!(matches!(
        b[3].role,
        BlockRole::TableCell {
            table: 0,
            column: 0,
            label_row: false,
            ..
        }
    ));
    assert_eq!(b[3].block_type, BlockType::Table);
}

#[test]
fn malformed_document_is_an_error() {
    assert!(
        XhtmlParser
            .parse_checked(&wrap("<p>a &bogus; b</p>"))
            .is_err()
    );
    assert!(XhtmlParser.parse_checked(&wrap("<p>a <em>b</p>")).is_err());
}

#[test]
fn identity_reconstruction_is_byte_exact() {
    let src = wrap(
        "<p>One <em>two</em>. Three&nbsp;four.</p>\n<ul>\n  <li>Item <img src=\"x\" alt=\"Alt text\" /> end</li>\n</ul>",
    );
    let doc = XhtmlParser.parse_checked(&src).unwrap();
    let identity: TranslationMap = doc
        .all_segments()
        .iter()
        .map(|s| (s.id.clone(), s.source.clone()))
        .collect();
    assert_eq!(XhtmlParser.reconstruct(&doc, &identity), src);
    assert_eq!(XhtmlParser.reconstruct(&doc, &TranslationMap::new()), src);
}

#[test]
fn reconstruction_rewrites_attributes_inside_translated_runs() {
    let src = wrap(
        "<p>Look <img src=\"x\" alt=\"Chart\" title=\"Hint\" /> now.</p><img src=\"y\" alt=\"Other\" />",
    );
    let doc = XhtmlParser.parse_checked(&src).unwrap();
    let mut t = TranslationMap::new();
    for seg in doc.all_segments() {
        let ko = match seg.source.as_str() {
            "Look <img src=\"x\" alt=\"Chart\" title=\"Hint\" /> now." => {
                "지금 <img src=\"x\" alt=\"Chart\" title=\"Hint\" /> 보세요."
            }
            "Chart" => "도표",
            "Hint" => "힌트",
            "Other" => "다른 그림",
            _ => continue,
        };
        t.insert(seg.id.clone(), ko.to_string());
    }
    let out = XhtmlParser.reconstruct(&doc, &t);
    assert!(
        out.contains("<p>지금 <img src=\"x\" alt=\"도표\" title=\"힌트\" /> 보세요.</p>"),
        "{out}"
    );
    assert!(out.contains("<img src=\"y\" alt=\"다른 그림\" />"), "{out}");
    assert_eq!(skeleton(&out).unwrap(), skeleton(&src).unwrap());
}

#[test]
fn skeleton_ignores_text_and_translated_attributes() {
    let a = wrap("<p class=\"x\">Hello <em>there</em><img src=\"a\" alt=\"one\" /></p>");
    let b = wrap("<p class=\"x\">안녕 <em>거기</em><img src=\"a\" alt=\"하나\" /></p>");
    let c = wrap("<p class=\"y\">Hello <em>there</em><img src=\"a\" alt=\"one\" /></p>");
    assert_eq!(skeleton(&a).unwrap(), skeleton(&b).unwrap());
    assert_ne!(skeleton(&a).unwrap(), skeleton(&c).unwrap());
}

#[test]
fn skeleton_allows_reordered_inline_elements_and_split_emphasis() {
    let a = wrap(
        "<p>A <span class=\"x\">b</span> <span class=\"y\">c</span> <em>d e</em>.</p><p>Next</p>",
    );
    let b = wrap(
        "<p><span class=\"y\">다</span> <span class=\"x\">나</span> <em>라</em> 가 <em>마</em>.</p><p>다음</p>",
    );
    let dropped = wrap("<p><span class=\"y\">다</span> 나 <em>라</em>.</p><p>다음</p>");
    let moved_block = wrap(
        "<p>다음</p><p><span class=\"y\">다</span> <span class=\"x\">나</span> <em>라</em>.</p>",
    );
    assert_eq!(skeleton(&a).unwrap(), skeleton(&b).unwrap());
    assert_ne!(skeleton(&a).unwrap(), skeleton(&dropped).unwrap());
    assert_ne!(skeleton(&a).unwrap(), skeleton(&moved_block).unwrap());
}

fn segments_in(parser: &dyn DocumentParser, source: &str) -> Vec<String> {
    parser
        .parse_checked(source)
        .unwrap()
        .all_segments()
        .iter()
        .map(|s| s.source.clone())
        .collect()
}

#[test]
fn inline_element_holding_a_block_is_walked_as_one() {
    // Without this an element outside the block list would be one run.
    let src = "<document><content><unknown><para>One.</para><para>Two.</para></unknown></content></document>";
    assert_eq!(segments_in(&CnxmlParser, src), ["One.", "Two."]);
    let src = wrap("<div><span class=\"x\"><p>One.</p><p>Two.</p></span></div>");
    assert_eq!(segments(&src)[1..], ["One.", "Two."]);
}

#[test]
fn source_whitespace_between_segments_survives_translation() {
    let src = "<document><content><para>One.  Two.\n   Three.</para></content></document>";
    let doc = CnxmlParser.parse_checked(src).unwrap();
    let t: TranslationMap = doc
        .all_segments()
        .iter()
        .map(|s| (s.id.clone(), format!("[{}]", s.source)))
        .collect();
    assert_eq!(
        CnxmlParser.reconstruct(&doc, &t),
        "<document><content><para>[One.]  [Two.]\n   [Three.]</para></content></document>"
    );
}

#[test]
fn pretext_restrict_version_inside_a_paragraph_stays_in_the_run() {
    let src = "<section><p>Some text <restrict-version versions=\"1553\">for one version</restrict-version> and more.</p></section>";
    assert_eq!(
        segments_in(&PretextParser, src),
        [
            "Some text <restrict-version versions=\"1553\">for one version</restrict-version> and more."
        ]
    );
}

#[test]
fn pretext_display_math_holding_the_period_ends_the_sentence() {
    let src = "<section><p>We have\n<md><mrow>x \\amp = 1</mrow><mrow>y \\amp = 2.</mrow></md>\nThen <m>x+y=3</m>, so\n<me>z = 3,</me>\nwhich is odd.</p></section>";
    assert_eq!(
        segments_in(&PretextParser, src),
        [
            "We have\n<md><mrow>x \\amp = 1</mrow><mrow>y \\amp = 2.</mrow></md>",
            "Then <m>x+y=3</m>, so\n<me>z = 3,</me>\nwhich is odd."
        ]
    );
}

#[test]
fn pretext_code_only_and_math_only_runs_are_skipped() {
    let src = "<section><title><m>\\R^n</m></title><p><m>x</m></p><p><c>f()</c></p><p>Real.</p></section>";
    assert_eq!(segments_in(&PretextParser, src), ["Real."]);
}

#[test]
fn cnxml_mtext_keeps_its_spaces() {
    let src = "<document><content><equation><math><mn>3</mn><mtext> per year </mtext></math></equation></content></document>";
    let doc = CnxmlParser.parse_checked(src).unwrap();
    assert_eq!(segments_in(&CnxmlParser, src), ["per year"]);
    let t: TranslationMap = doc
        .all_segments()
        .iter()
        .map(|s| (s.id.clone(), "연간".to_string()))
        .collect();
    assert!(
        CnxmlParser
            .reconstruct(&doc, &t)
            .contains("<mtext> 연간 </mtext>")
    );
}

#[test]
fn cnxml_skeleton_lets_emphasis_split() {
    let a = "<document><content><para>A <emphasis effect=\"italics\">b c</emphasis> d <link url=\"u\">e</link>.</para></content></document>";
    let b = "<document><content><para><link url=\"u\">마</link> <emphasis effect=\"italics\">나</emphasis> 가 <emphasis effect=\"italics\">다</emphasis>.</para></content></document>";
    let dropped = "<document><content><para>가 <emphasis effect=\"italics\">나</emphasis>.</para></content></document>";
    let skel = |s| skeleton_with(Vocabulary::Cnxml, s).unwrap();
    assert_eq!(skel(a), skel(b));
    assert_ne!(skel(a), skel(dropped));
}
