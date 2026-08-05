// ─── Style Engine Integration Test ───────────────────────────────
//
// End-to-end test that parses real HTML + CSS, resolves styles,
// and verifies the DOM is getting properly styled with:
//   - Specificity-based cascade
//   - Property inheritance
//   - em → px computation
//   - Default/initial values
//   - Inline style override
//   - Typed ComputedStyle output

use asteria::css_parser::Stylesheet;
use asteria::dom::Dom;
use asteria::parser::Parser;
use asteria::style::{StyledNode, resolve_styles};
use asteria::tokenizer::Tokenizer;
use asteria::values::{Color, Display, Edges, TextAlign};

/// Helper: parse HTML + CSS → styled tree
fn styled_tree(html: &str, css: &str) -> (StyledNode, Dom, Vec<u8>) {
    let html_bytes = html.as_bytes().to_vec();
    let mut tokenizer = Tokenizer::new(&html_bytes);
    let tokens = tokenizer.tokenize();
    let parser = Parser::new(&tokens, &html_bytes);
    let dom = parser.parse();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, &html_bytes);

    (styled, dom, html_bytes)
}

// ─── Full Page Integration Test ──────────────────────────────────

#[test]
fn test_full_page_styling() {
    // Compact HTML — no whitespace between tags to avoid text node children
    let html = r#"<html><body><div id="header" class="section"><h1>Title</h1><p>Subtitle</p></div><div class="content"><p class="intro">Hello</p></div></body></html>"#;

    let css = r#"
        body { color: navy; font-size: 18px; }
        #header { background-color: #f0f0f0; margin: 20px; }
        .section { color: red; }
        h1 { font-size: 2em; font-weight: bold; }
        p { margin: 10px; }
        .intro { color: green; text-align: center; }
    "#;

    let (styled, _, _) = styled_tree(html, css);

    // Navigate: Document → html → body
    let html_node = &styled.children[0];
    let body = &html_node.children[0];

    // body: color=navy (named → rgb(0,0,128)), font-size=18px
    assert_eq!(body.styles.color, Color::rgb(0, 0, 128));
    assert_eq!(body.styles.font_size, 18.0);

    // #header div: background-color from id selector, color from .section
    let header = &body.children[0];
    assert_eq!(header.styles.background_color, Color::rgb(240, 240, 240));
    // .section { color: red } — specificity (0,1,0) — sets color
    assert_eq!(header.styles.color, Color::rgb(255, 0, 0));
    assert_eq!(header.styles.margin, Edges::uniform(20.0));

    // h1 inside #header: font-size: 2em relative to inherited 18px = 36px
    let h1 = &header.children[0];
    assert_eq!(h1.styles.font_size, 36.0);
    assert_eq!(h1.styles.font_weight, 700.0);
    // h1 inherits color from #header (.section) = red
    assert_eq!(h1.styles.color, Color::rgb(255, 0, 0));

    // p inside #header: inherits color=red from parent, own margin=10px
    let p_subtitle = &header.children[1];
    assert_eq!(p_subtitle.styles.color, Color::rgb(255, 0, 0)); // inherited
    assert_eq!(p_subtitle.styles.margin, Edges::uniform(10.0)); // own rule

    // .content div: inherits color=navy from body (no own color rule)
    let content = &body.children[1];
    assert_eq!(content.styles.color, Color::rgb(0, 0, 128));

    // p.intro: color=green (own rule beats inherited navy), text-align=center
    let intro = &content.children[0];
    assert_eq!(intro.styles.color, Color::rgb(0, 128, 0));
    assert_eq!(intro.styles.text_align, TextAlign::Center);
    // margin comes from p { margin: 10px }
    assert_eq!(intro.styles.margin, Edges::uniform(10.0));
}

// ─── Specificity Cascade Test ────────────────────────────────────

#[test]
fn test_specificity_cascade_ordering() {
    // Three rules targeting the same element with different specificities
    let (styled, _, _) = styled_tree(
        r#"<div id="main" class="container">Content</div>"#,
        r#"
            div { color: red; font-size: 10px; }
            .container { color: green; font-size: 20px; }
            #main { color: blue; }
        "#,
    );

    let div = &styled.children[0];

    // color: #main(1,0,0) beats .container(0,1,0) beats div(0,0,1)
    assert_eq!(div.styles.color, Color::rgb(0, 0, 255));

    // font-size: .container(0,1,0) beats div(0,0,1), #main doesn't set it
    assert_eq!(div.styles.font_size, 20.0);
}

// ─── Inheritance Chain Test ──────────────────────────────────────

#[test]
fn test_deep_inheritance_chain() {
    // color and font-size should propagate through multiple levels
    let (styled, _, _) = styled_tree(
        "<div><section><article><p>Deep</p></article></section></div>",
        "div { color: purple; font-size: 20px; }",
    );

    let div = &styled.children[0];
    let section = &div.children[0];
    let article = &section.children[0];
    let p = &article.children[0];

    // All should inherit div's color and font-size
    assert_eq!(div.styles.color, Color::rgb(128, 0, 128));
    assert_eq!(section.styles.color, Color::rgb(128, 0, 128));
    assert_eq!(article.styles.color, Color::rgb(128, 0, 128));
    assert_eq!(p.styles.color, Color::rgb(128, 0, 128));

    assert_eq!(div.styles.font_size, 20.0);
    assert_eq!(p.styles.font_size, 20.0);

    // margin should NOT inherit (stays at default 0)
    assert_eq!(p.styles.margin, Edges::ZERO);
}

// ─── em Computation Through Inheritance ──────────────────────────

#[test]
fn test_em_chain() {
    // Each level doubles font-size via 2em
    let (styled, _, _) = styled_tree(
        "<div><p><span>Text</span></p></div>",
        "div { font-size: 10px; } p { font-size: 2em; } span { font-size: 2em; }",
    );

    let div = &styled.children[0];
    let p = &div.children[0];
    let span = &p.children[0];

    assert_eq!(div.styles.font_size, 10.0);
    assert_eq!(p.styles.font_size, 20.0); // 2 * 10
    assert_eq!(span.styles.font_size, 40.0); // 2 * 20
}

// ─── Inline Style Override Test ──────────────────────────────────

#[test]
fn test_inline_beats_high_specificity() {
    // Inline style should beat even an #id selector
    let (styled, _, _) = styled_tree(
        r#"<div id="main" style="color: green">Content</div>"#,
        "#main { color: red; }",
    );

    let div = &styled.children[0];
    // Inline (Origin::Inline) beats #main (Origin::Author)
    assert_eq!(div.styles.color, Color::rgb(0, 128, 0));
}

// ─── Non-Inherited Properties Stay Initial ───────────────────────

#[test]
fn test_non_inherited_defaults() {
    let (styled, _, _) = styled_tree(
        "<div><span>Hello</span></div>",
        "div { display: block; width: 500px; padding: 15px; }",
    );

    let div = &styled.children[0];
    let span = &div.children[0];

    // div has explicit values
    assert_eq!(div.styles.display, Display::Block);
    assert_eq!(div.styles.width, Some(500.0));
    assert_eq!(div.styles.padding, Edges::uniform(15.0));

    // span: display, width, padding do NOT inherit — get initial values
    assert_eq!(span.styles.display, Display::Inline); // initial
    assert_eq!(span.styles.width, None); // initial = auto
    assert_eq!(span.styles.padding, Edges::ZERO); // initial
}

// ─── Shorthand Expansion Test ────────────────────────────────────

#[test]
fn test_shorthand_with_longhand_override() {
    // longhand should override shorthand
    let (styled, _, _) = styled_tree(
        "<div>Content</div>",
        "div { margin: 10px; margin-left: 30px; }",
    );

    let div = &styled.children[0];
    assert_eq!(div.styles.margin.top, 10.0);
    assert_eq!(div.styles.margin.right, 10.0);
    assert_eq!(div.styles.margin.bottom, 10.0);
    // longhand override
    assert_eq!(div.styles.margin.left, 30.0);
}

// ─── Display None Test ───────────────────────────────────────────

#[test]
fn test_display_none_typed() {
    let (styled, _, _) = styled_tree("<div>Hidden</div>", "div { display: none; }");

    let div = &styled.children[0];
    assert_eq!(div.styles.display, Display::None);
}

// ─── Styled Tree Printer Sanity Check ────────────────────────────

#[test]
fn test_styled_tree_output_contains_typed_values() {
    let (styled, dom, source) = styled_tree(
        r#"<h1 id="title">Hello</h1>"#,
        "#title { color: red; font-size: 24px; font-weight: bold; }",
    );

    let output = styled.format_tree(&dom, &source);

    // Should contain typed color output, not raw "red"
    assert!(
        output.contains("rgb(255,0,0)"),
        "Expected rgb color in output: {}",
        output
    );
    assert!(
        output.contains("24px"),
        "Expected 24px font-size in output: {}",
        output
    );
    assert!(
        output.contains("700"),
        "Expected 700 font-weight in output: {}",
        output
    );
}

#[test]
fn test_unsupported_media_queries_filtered() {
    let stylesheet = Stylesheet::parse(
        b"@media print { body { color: red; } }\n@media not screen { div { color: blue; } }",
    );
    assert_eq!(stylesheet.media_rules.len(), 0);
}

#[test]
fn test_descendant_selector_backtracking() {
    let (styled, _, _) = styled_tree(
        "<div class='outer'><div><div class='inner'><p>Test</p></div></div></div>",
        ".outer p { color: blue; }",
    );

    let p_node = &styled.children[0].children[0].children[0].children[0];
    assert_eq!(p_node.styles.color, asteria::values::Color::rgb(0, 0, 255));
}

#[test]
fn test_document_position_cascade_order() {
    let (styled, _, _) = styled_tree(
        "<div>Text</div>",
        "@media (min-width: 100px) { div { color: red; } }\ndiv { color: blue; }",
    );

    let div = &styled.children[0];
    // Later top-level rule overrides earlier media rule at equal specificity
    assert_eq!(div.styles.color, asteria::values::Color::rgb(0, 0, 255));
}
