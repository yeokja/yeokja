use yeokja_core::parser::{DocumentParser, TranslationMap};
use yeokja_parser_markdeep::MarkdeepParser;

#[test]
fn indented_markdeep_title_and_listing_caption_are_translatable() {
    let source = "            **Ray Tracing**\n\n<div class='together'>\nRead this.\n\n    ~~~~~ C++\n    int value = 1;\n    ~~~ C++ highlight\n    int other = 2;\n    ~~~~~\n    [Listing [example]: <kbd>[main.cc]</kbd> Create an image]\n\n</div>\n";
    let doc = MarkdeepParser.parse(source);
    let segments = doc.translatable_segments();
    assert!(segments.iter().any(|s| s.source.contains("Ray Tracing")));
    assert!(
        segments
            .iter()
            .any(|s| s.source.contains("Create an image"))
    );
    assert!(
        !segments
            .iter()
            .any(|s| s.source.contains("int value") || s.source.contains("int other"))
    );
    let translations = segments
        .iter()
        .map(|s| (s.id.clone(), "번역문".to_owned()))
        .collect::<TranslationMap>();
    let output = MarkdeepParser.reconstruct(&doc, &translations);
    assert!(output.contains("    int value = 1;\n    ~~~ C++ highlight\n    int other = 2;"));
    assert!(output.contains("[Listing [example]: <kbd>[main.cc]</kbd> 번역문]"));
}

#[test]
fn display_equations_and_scripts_are_never_translated() {
    let source = "Prose.\n\n  $$ x = \\sqrt{2} $$\n\n  $$\n  f(x) = x + 1\n  $$\n\n<script>window.foo = 'text';</script>\n\nMore prose.\n";
    let doc = MarkdeepParser.parse(source);
    let segments = doc.translatable_segments();
    assert_eq!(
        segments
            .iter()
            .map(|s| s.source.as_str())
            .collect::<Vec<_>>(),
        vec!["Prose.", "More prose."]
    );
    assert_eq!(
        MarkdeepParser.reconstruct(&doc, &TranslationMap::new()),
        source
    );
}

#[test]
fn indented_list_continuation_stays_in_its_paragraph() {
    let source = "  - Our style uses simple C++, but we take\n    advantage of modern features.\n";
    let doc = MarkdeepParser.parse(source);
    assert_eq!(
        doc.translatable_segments()
            .iter()
            .map(|s| s.source.as_str())
            .collect::<Vec<_>>(),
        vec!["Our style uses simple C++, but we take advantage of modern features."]
    );
}

#[test]
fn multiline_captions_keep_identifiers_and_image_attributes_outside_translations() {
    let source = "    [Listing [example]: <kbd>[main.cc]</kbd> Create an\n    image]\n\n  ![Figure [ray]: A ray\n  through a pixel](../images/ray.png width='50%')\n\n  ![<span class='num'>Image 1:</span> First PPM image\n  ](../images/ppm.png class='pixel')\n";
    let doc = MarkdeepParser.parse(source);
    let segments = doc.translatable_segments();
    assert_eq!(
        segments
            .iter()
            .map(|s| s.source.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Create an image",
            "A ray through a pixel",
            "First PPM image"
        ]
    );
    let translations = segments
        .iter()
        .map(|s| (s.id.clone(), "번역문".to_owned()))
        .collect();
    assert_eq!(
        MarkdeepParser.reconstruct(&doc, &translations),
        "    [Listing [example]: <kbd>[main.cc]</kbd> 번역문]\n\n  ![Figure [ray]: 번역문](../images/ray.png width='50%')\n\n  ![<span class='num'>Image 1:</span> 번역문\n  ](../images/ppm.png class='pixel')\n"
    );
}

#[test]
fn insert_directives_remain_executable_markdeep_syntax() {
    let source = "Acknowledgments\n===============\n\n    (insert acknowledgments.md.html here)\n\nMore prose.\n";
    let doc = MarkdeepParser.parse(source);
    assert!(
        !doc.translatable_segments()
            .iter()
            .any(|s| s.source.contains("insert"))
    );
    let translations = doc
        .translatable_segments()
        .iter()
        .map(|s| (s.id.clone(), "번역문".to_owned()))
        .collect();
    assert!(
        MarkdeepParser
            .reconstruct(&doc, &translations)
            .contains("    (insert acknowledgments.md.html here)")
    );
}

#[test]
fn deeply_indented_numbered_lists_keep_markers_and_continuations() {
    let source = "    1. Calculate the ray\n       through the pixel.\n    2. Compute a color.\n";
    let doc = MarkdeepParser.parse(source);
    assert_eq!(
        doc.translatable_segments()
            .iter()
            .map(|s| s.source.as_str())
            .collect::<Vec<_>>(),
        vec!["Calculate the ray through the pixel.", "Compute a color."]
    );
    let translations = doc
        .translatable_segments()
        .iter()
        .map(|s| (s.id.clone(), "번역문".to_owned()))
        .collect();
    assert_eq!(
        MarkdeepParser.reconstruct(&doc, &translations),
        "    1. 번역문\n    2. 번역문\n"
    );
}

#[test]
fn author_initials_do_not_split_the_graphics_reading_recommendation() {
    let source = "We assume a little bit of familiarity with vectors (like dot product and vector addition). If you\ndon’t know that, do a little review. If you need that review, or to learn it for the first time,\ncheck out the online [_Graphics Codex_][gfx-codex] by Morgan McGuire, _Fundamentals of Computer\nGraphics_ by Steve Marschner and Peter Shirley, or _Computer Graphics: Principles and Practice_\nby J.D. Foley and Andy Van Dam.\n";
    let doc = MarkdeepParser.parse(source);
    let segments = doc.translatable_segments();
    assert_eq!(segments.len(), 3);
    assert!(
        segments[2]
            .source
            .ends_with("by J.D. Foley and Andy Van Dam.")
    );
    assert_eq!(segments[2].source.matches('_').count(), 6);
    assert_eq!(
        segments[2].source_hash,
        yeokja_core::hash::content_hash(&segments[2].source)
    );
    assert_eq!(segments[2].id.position(), Some((0, 0, 2)));
    assert_eq!(
        MarkdeepParser.reconstruct(&doc, &TranslationMap::new()),
        source
    );
}

#[test]
fn backtick_code_fences_do_not_expose_indented_code() {
    let source = "```cpp\n    print(\"Do not translate\");\n```\n\nProse.\n";
    let doc = MarkdeepParser.parse(source);
    assert_eq!(
        doc.translatable_segments()
            .iter()
            .map(|s| s.source.as_str())
            .collect::<Vec<_>>(),
        vec!["Prose."]
    );
}

#[test]
fn text_after_an_image_remains_translatable() {
    let source = "![A picture](a.png) This sentence also needs translation.\n";
    let doc = MarkdeepParser.parse(source);
    assert_eq!(
        doc.translatable_segments()
            .iter()
            .map(|s| s.source.as_str())
            .collect::<Vec<_>>(),
        vec!["A picture", "This sentence also needs translation."]
    );
}

#[test]
fn translated_image_caption_and_following_prose_keep_the_destination() {
    let source = "![A picture](assets/a(b).png) Following prose.\n";
    let doc = MarkdeepParser.parse(source);
    let translations = doc
        .translatable_segments()
        .iter()
        .map(|s| (s.id.clone(), "번역문".to_owned()))
        .collect();
    assert_eq!(
        MarkdeepParser.reconstruct(&doc, &translations),
        "![번역문](assets/a(b).png) 번역문\n"
    );
}
