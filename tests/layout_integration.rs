// ─── Layout Engine Integration Test ───────────────────────────────
//
// End-to-end integration test verifying that:
//   - HTML + CSS parse and style resolution generate correct StyledNodes
//   - Layout Engine correctly builds the Box Model geometry (width, height, x, y)
//   - Auto width expansion fills container width
//   - Nested elements compute proper relative coordinates
//   - Display: none elements are filtered out

use asteria::css_parser::Stylesheet;
use asteria::dom::Dom;
use asteria::layout::{BoxType, LayoutBox, layout_document};
use asteria::parser::Parser;
use asteria::style::{StyledNode, resolve_styles};
use asteria::tokenizer::Tokenizer;

fn parse_and_layout<'a>(
    html: &'a str,
    css: &'a str,
    viewport_width: f32,
    viewport_height: f32,
    dom_store: &'a mut Option<Dom>,
    bytes_store: &'a mut Vec<u8>,
    styled_store: &'a mut Option<StyledNode>,
) -> LayoutBox<'a> {
    *bytes_store = html.as_bytes().to_vec();
    let mut tokenizer = Tokenizer::new(bytes_store);
    let tokens = tokenizer.tokenize();
    let parser = Parser::new(&tokens, bytes_store);
    *dom_store = Some(parser.parse());

    let stylesheet = Stylesheet::parse(css.as_bytes());
    *styled_store = Some(resolve_styles(
        dom_store.as_ref().unwrap(),
        &stylesheet,
        bytes_store,
    ));

    layout_document(
        styled_store.as_ref().unwrap(),
        dom_store.as_ref().unwrap(),
        bytes_store,
        viewport_width,
        viewport_height,
    )
    .unwrap()
}

#[test]
fn test_block_layout_width_expansion() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="container"></div></body></html>"#;
    let css = r#"#container { margin: 10px; padding: 20px; border: 0px; }"#;

    let layout = parse_and_layout(
        html,
        css,
        800.0,
        600.0,
        &mut dom_store,
        &mut bytes_store,
        &mut styled_store,
    );

    // Document → html → body → div#container
    let html_box = &layout.children[0];
    let body_box = &html_box.children[0];
    let container_box = &body_box.children[0];

    assert_eq!(container_box.box_type, BoxType::BlockNode);
    // Viewport: 800px. body margin=8px (avail=784px). div#container margin=10px, padding=20px, border=0px.
    // Content width = 784 - (10+10) - (20+20) = 724px
    assert_eq!(container_box.dimensions.content.width, 724.0);
    // X position = 8 (body margin) + 10 (margin) + 20 (padding) = 38px
    assert_eq!(container_box.dimensions.content.x, 38.0);
}

#[test]
fn test_display_none_filtering() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div class="hidden">Secret</div><div class="visible">Shown</div></body></html>"#;
    let css = r#"
        .hidden { display: none; }
        .visible { display: block; }
    "#;

    let layout = parse_and_layout(
        html,
        css,
        800.0,
        600.0,
        &mut dom_store,
        &mut bytes_store,
        &mut styled_store,
    );

    let html_box = &layout.children[0];
    let body_box = &html_box.children[0];

    // body_box should only have 1 child (visible), since hidden was filtered out
    assert_eq!(body_box.children.len(), 1);
}

#[test]
fn test_layout_inline_horizontal_flow_and_line_wrap() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><p><span>SpanOne</span><span>SpanTwo</span></p></body></html>"#;
    let css = r#"p { width: 80px; }"#;

    let layout = parse_and_layout(
        html,
        css,
        800.0,
        600.0,
        &mut dom_store,
        &mut bytes_store,
        &mut styled_store,
    );

    let html_box = &layout.children[0];
    let body_box = &html_box.children[0];
    let p_box = &body_box.children[0];

    // p_box content width is 80px
    let span1 = &p_box.children[0];
    let span2 = &p_box.children[1];

    // Span 1 starts at p content origin (x = p_x, y = p_y)
    assert_eq!(span1.dimensions.content.x, p_box.dimensions.content.x);
    assert_eq!(span1.dimensions.content.y, p_box.dimensions.content.y);

    // Span 2: 61.6 + 61.6 = 123.2px > 80px container width -> line wraps to next line (x = p_x, y = p_y + 19.2px)!
    assert_eq!(span2.dimensions.content.x, p_box.dimensions.content.x);
    assert_eq!(
        span2.dimensions.content.y,
        p_box.dimensions.content.y + 19.2
    );
}

#[test]
fn test_layout_inline_side_by_side_flow() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><p><span>Hello</span><span>World</span></p></body></html>"#;
    let css = r#"p { width: 500px; }"#;

    let layout = parse_and_layout(
        html,
        css,
        800.0,
        600.0,
        &mut dom_store,
        &mut bytes_store,
        &mut styled_store,
    );

    let html_box = &layout.children[0];
    let body_box = &html_box.children[0];
    let p_box = &body_box.children[0];

    let span1 = &p_box.children[0];
    let span2 = &p_box.children[1];

    // Span 1 width = 5 chars * 16 * 0.55 = 44px
    assert_eq!(span1.dimensions.content.width, 44.0);

    // Span 1 and Span 2 are on the SAME line y
    assert_eq!(span1.dimensions.content.y, p_box.dimensions.content.y);
    assert_eq!(span2.dimensions.content.y, p_box.dimensions.content.y);

    // Span 2 x is horizontally offset by Span 1 width (x = p_x + 44px)
    assert_eq!(span1.dimensions.content.x, p_box.dimensions.content.x);
    assert_eq!(
        span2.dimensions.content.x,
        p_box.dimensions.content.x + 44.0
    );
}

#[test]
fn test_content_box_sizing_specified_width() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div class="box">Content</div></body></html>"#;
    let css = r#".box { width: 200px; padding: 16px; border: 1px solid black; }"#;

    let layout = parse_and_layout(
        html,
        css,
        800.0,
        600.0,
        &mut dom_store,
        &mut bytes_store,
        &mut styled_store,
    );

    let box_node = &layout.children[0].children[0].children[0];
    // Under CSS content-box semantics, specified width (200px) is assigned to content width
    assert_eq!(box_node.dimensions.content.width, 200.0);
    assert_eq!(box_node.dimensions.padding.left, 16.0);
    assert_eq!(box_node.dimensions.border.left, 1.0);
    assert_eq!(box_node.dimensions.border_box().width, 234.0);
}
