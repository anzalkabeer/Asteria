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
use asteria::layout::{layout_document, BoxType, LayoutBox};
use asteria::parser::Parser;
use asteria::style::{resolve_styles, StyledNode};
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

    layout_document(styled_store.as_ref().unwrap(), viewport_width, viewport_height).unwrap()
}

#[test]
fn test_block_layout_width_expansion() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="container"></div></body></html>"#;
    let css = r#"#container { margin: 10px; padding: 20px; }"#;

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
    // Viewport: 800px. div#container margin=10px, padding=20px, border=0px.
    // Content width = 800 - (10+10) - (20+20) = 740px
    assert_eq!(container_box.dimensions.content.width, 740.0);
    // X position = 0 (body) + 10 (margin) + 20 (padding) = 30px
    assert_eq!(container_box.dimensions.content.x, 30.0);
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
