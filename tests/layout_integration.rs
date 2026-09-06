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
use asteria::style::{StyledNode, resolve_styles};

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
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes_store, true);
    *dom_store = Some(processor.finish());

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

    // Span 1 width = proportional width of "Hello" (0.65+0.52+0.28+0.28+0.52) * 16 = 36px
    assert_eq!(span1.dimensions.content.width, 36.0);

    // Span 1 and Span 2 are on the SAME line y
    assert_eq!(span1.dimensions.content.y, p_box.dimensions.content.y);
    assert_eq!(span2.dimensions.content.y, p_box.dimensions.content.y);

    // Span 2 x is horizontally offset by Span 1 width (x = p_x + 36px)
    assert_eq!(span1.dimensions.content.x, p_box.dimensions.content.x);
    assert_eq!(
        span2.dimensions.content.x,
        p_box.dimensions.content.x + 36.0
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

#[test]
fn test_relative_positioning_offsets() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="first"></div><div id="rel"></div><div id="third"></div></body></html>"#;
    let css = r#"
        div { margin: 0px; padding: 0px; border: 0px; }
        #first { width: 100px; height: 50px; }
        #rel { position: relative; top: 15px; left: 25px; width: 100px; height: 50px; }
        #third { width: 100px; height: 50px; }
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
    let first = &body_box.children[0];
    let rel = &body_box.children[1];
    let third = &body_box.children[2];

    let body_x = body_box.dimensions.content.x;
    let body_y = body_box.dimensions.content.y;

    assert_eq!(first.dimensions.content.x, body_x);
    assert_eq!(first.dimensions.content.y, body_y);

    assert_eq!(rel.dimensions.content.x, body_x + 25.0);
    assert_eq!(rel.dimensions.content.y, body_y + 50.0 + 15.0);

    assert_eq!(third.dimensions.content.x, body_x);
    assert_eq!(third.dimensions.content.y, body_y + 100.0);
}

#[test]
fn test_absolute_positioning_insets() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="parent"><div id="child"></div></div></body></html>"#;
    let css = r#"
        div { margin: 0px; border: 0px; }
        #parent { position: relative; width: 400px; height: 300px; padding: 10px; }
        #child { position: absolute; top: 20px; right: 30px; width: 100px; height: 60px; padding: 0px; }
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
    let parent_box = &body_box.children[0];
    let child_box = &parent_box.children[0];

    let pad_box = parent_box.dimensions.padding_box();
    assert_eq!(child_box.dimensions.content.y, pad_box.y + 20.0);
    assert_eq!(
        child_box.dimensions.content.x,
        pad_box.x + pad_box.width - 30.0 - 100.0
    );
    assert_eq!(child_box.dimensions.content.width, 100.0);
    assert_eq!(child_box.dimensions.content.height, 60.0);
}

#[test]
fn test_fixed_positioning_viewport_anchor() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="nested"><div id="fixed"></div></div></body></html>"#;
    let css = r#"
        div { margin: 0px; padding: 0px; border: 0px; }
        #nested { margin: 50px; padding: 20px; }
        #fixed { position: fixed; top: 10px; left: 15px; width: 120px; height: 40px; }
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
    let nested = &body_box.children[0];
    let fixed_child = &nested.children[0];

    assert_eq!(fixed_child.dimensions.content.x, 15.0);
    assert_eq!(fixed_child.dimensions.content.y, 10.0);
    assert_eq!(fixed_child.dimensions.content.width, 120.0);
    assert_eq!(fixed_child.dimensions.content.height, 40.0);
}

#[test]
fn test_flex_grow_distribution() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="container"><div id="b1"></div><div id="b2"></div></div></body></html>"#;
    let css = r#"
        div { margin: 0px; padding: 0px; border: 0px; }
        #container { display: flex; width: 600px; }
        #b1 { width: 100px; flex-grow: 1; }
        #b2 { width: 200px; flex-grow: 2; }
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
    let container = &body_box.children[0];
    let box1 = &container.children[0];
    let box2 = &container.children[1];

    assert_eq!(box1.dimensions.content.width, 200.0);
    assert_eq!(box2.dimensions.content.width, 400.0);
}

#[test]
fn test_flex_direction_column() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="container"><div id="item1"></div><div id="item2"></div></div></body></html>"#;
    let css = r#"
        div { margin: 0px; padding: 0px; border: 0px; }
        #container { display: flex; flex-direction: column; width: 300px; height: 400px; gap: 10px; }
        #item1 { height: 50px; }
        #item2 { height: 70px; }
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
    let container = &body_box.children[0];
    let item1 = &container.children[0];
    let item2 = &container.children[1];

    assert_eq!(item1.dimensions.content.y, container.dimensions.content.y);
    assert_eq!(
        item2.dimensions.content.y,
        container.dimensions.content.y + 50.0 + 10.0
    );
    assert_eq!(item1.dimensions.content.height, 50.0);
    assert_eq!(item2.dimensions.content.height, 70.0);
    assert_eq!(item1.dimensions.content.width, 300.0);
    assert_eq!(item2.dimensions.content.width, 300.0);
}

#[test]
fn test_flex_justify_content() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="c_center"><div class="item"></div><div class="item"></div></div><div id="c_between"><div class="item"></div><div class="item"></div></div></body></html>"#;
    let css = r#"
        div { margin: 0px; padding: 0px; border: 0px; }
        #c_center { display: flex; justify-content: center; width: 500px; }
        #c_between { display: flex; justify-content: space-between; width: 500px; }
        .item { width: 100px; height: 40px; }
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
    let c_center = &body_box.children[0];
    let c_between = &body_box.children[1];

    let center_item0 = &c_center.children[0];
    let center_item1 = &c_center.children[1];
    assert_eq!(
        center_item0.dimensions.content.x,
        c_center.dimensions.content.x + 150.0
    );
    assert_eq!(
        center_item1.dimensions.content.x,
        c_center.dimensions.content.x + 250.0
    );

    let between_item0 = &c_between.children[0];
    let between_item1 = &c_between.children[1];
    assert_eq!(
        between_item0.dimensions.content.x,
        c_between.dimensions.content.x
    );
    assert_eq!(
        between_item1.dimensions.content.x,
        c_between.dimensions.content.x + 400.0
    );
}

#[test]
fn test_flex_align_items_and_self() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="container"><div id="i1"></div><div id="i2"></div></div></body></html>"#;
    let css = r#"
        div { margin: 0px; padding: 0px; border: 0px; }
        #container { display: flex; align-items: center; width: 400px; height: 200px; }
        #i1 { width: 100px; height: 60px; }
        #i2 { width: 100px; height: 60px; align-self: flex-end; }
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
    let container = &body_box.children[0];
    let i1 = &container.children[0];
    let i2 = &container.children[1];

    assert_eq!(
        i1.dimensions.content.y,
        container.dimensions.content.y + 70.0
    );
    assert_eq!(
        i2.dimensions.content.y,
        container.dimensions.content.y + 140.0
    );
}

#[test]
fn test_nested_positioned_ancestor_propagation() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    // Grandparent is relative; Parent is static in-flow wrapper; Child is absolute
    let html = r#"<html><body><div id="grandparent"><div id="parent"><div id="child"></div></div></div></body></html>"#;
    let css = r#"
        div { margin: 0px; padding: 0px; border: 0px; }
        #grandparent { position: relative; width: 400px; height: 300px; padding: 20px; }
        #parent { width: 200px; height: 150px; padding: 10px; margin: 5px; }
        #child { position: absolute; top: 15px; left: 25px; width: 80px; height: 50px; }
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
    let gp_box = &body_box.children[0];
    let p_box = &gp_box.children[0];
    let child_box = &p_box.children[0];

    // Child must be positioned relative to grandparent's padding box, NOT parent!
    let gp_pad = gp_box.dimensions.padding_box();
    assert_eq!(child_box.dimensions.content.x, gp_pad.x + 25.0);
    assert_eq!(child_box.dimensions.content.y, gp_pad.y + 15.0);
    assert_eq!(child_box.dimensions.content.width, 80.0);
    assert_eq!(child_box.dimensions.content.height, 50.0);
}

#[test]
fn test_deeply_nested_fixed_position_uses_document_viewport() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html =
        r#"<html><body><div id="c1"><div id="c2"><div id="fixed"></div></div></div></body></html>"#;
    let css = r#"
        div { margin: 0px; padding: 0px; border: 0px; }
        #c1 { width: 300px; height: 200px; margin: 40px; }
        #c2 { width: 150px; height: 100px; margin: 20px; }
        #fixed { position: fixed; top: 30px; left: 40px; width: 50%; height: 25%; }
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
    let c1 = &body_box.children[0];
    let c2 = &c1.children[0];
    let fixed_child = &c2.children[0];

    // Fixed child must resolve against document viewport (800x600):
    // top = 30px, left = 40px, width = 50% of 800 = 400px, height = 25% of 600 = 150px
    assert_eq!(fixed_child.dimensions.content.x, 40.0);
    assert_eq!(fixed_child.dimensions.content.y, 30.0);
    assert_eq!(fixed_child.dimensions.content.width, 400.0);
    assert_eq!(fixed_child.dimensions.content.height, 150.0);
}

#[test]
fn test_flex_justify_content_start_and_end_in_row_reverse() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="c_start"><div id="s1"></div><div id="s2"></div></div><div id="c_flex_start"><div id="fs1"></div><div id="fs2"></div></div></body></html>"#;
    let css = r#"
        div { margin: 0px; padding: 0px; border: 0px; }
        #c_start { display: flex; flex-direction: row-reverse; justify-content: start; width: 400px; }
        #c_flex_start { display: flex; flex-direction: row-reverse; justify-content: flex-start; width: 400px; }
        #s1, #s2, #fs1, #fs2 { width: 100px; height: 50px; }
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
    let c_start = &body_box.children[0];
    let c_flex_start = &body_box.children[1];

    let s1 = &c_start.children[0];
    let s2 = &c_start.children[1];
    let fs1 = &c_flex_start.children[0];
    let fs2 = &c_flex_start.children[1];

    // In row-reverse:
    // flex-start keeps existing Asteria behavior: items start at container left, with last item (fs2) placed first
    assert_eq!(fs2.dimensions.content.x, c_flex_start.dimensions.content.x);
    assert_eq!(
        fs1.dimensions.content.x,
        c_flex_start.dimensions.content.x + 100.0
    );

    // start in row-reverse inverts offset to unused_main (400 - 200 = 200)
    assert_eq!(
        s2.dimensions.content.x,
        c_start.dimensions.content.x + 200.0
    );
    assert_eq!(
        s1.dimensions.content.x,
        c_start.dimensions.content.x + 300.0
    );
}

// ─── Grid Engine Integration Tests ─────────────────────────────────

#[test]
fn test_grid_repeat_and_fr_sizing() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html =
        r#"<html><body><div id="grid"><div>1</div><div>2</div><div>3</div></div></body></html>"#;
    let css = r#"
        * { margin: 0px; padding: 0px; border: 0px; }
        #grid { display: grid; width: 300px; grid-template-columns: repeat(3, 1fr); grid-gap: 0px; }
        #grid > div { height: 50px; }
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

    let grid = &layout.children[0].children[0].children[0];
    assert_eq!(grid.dimensions.content.width, 300.0);
    assert_eq!(grid.children.len(), 3);

    let c0 = &grid.children[0];
    let c1 = &grid.children[1];
    let c2 = &grid.children[2];

    assert!((c0.dimensions.content.width - 100.0).abs() < 1e-3);
    assert!((c1.dimensions.content.width - 100.0).abs() < 1e-3);
    assert!((c2.dimensions.content.width - 100.0).abs() < 1e-3);

    assert!((c0.dimensions.content.x - grid.dimensions.content.x).abs() < 1e-3);
    assert!((c1.dimensions.content.x - (grid.dimensions.content.x + 100.0)).abs() < 1e-3);
    assert!((c2.dimensions.content.x - (grid.dimensions.content.x + 200.0)).abs() < 1e-3);
}

#[test]
fn test_grid_gap_sizing() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html =
        r#"<html><body><div id="grid"><div>1</div><div>2</div><div>3</div></div></body></html>"#;
    let css = r#"
        * { margin: 0px; padding: 0px; border: 0px; }
        #grid { display: grid; width: 290px; grid-template-columns: repeat(3, 1fr); grid-gap: 10px; }
        #grid > div { height: 50px; }
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

    let grid = &layout.children[0].children[0].children[0];
    let c0 = &grid.children[0];
    let c1 = &grid.children[1];
    let c2 = &grid.children[2];

    // (290 - 2 * 10) / 3 = 90px
    assert!((c0.dimensions.content.width - 90.0).abs() < 1e-3);
    assert!((c1.dimensions.content.width - 90.0).abs() < 1e-3);
    assert!((c2.dimensions.content.width - 90.0).abs() < 1e-3);

    assert!((c0.dimensions.content.x - grid.dimensions.content.x).abs() < 1e-3);
    assert!((c1.dimensions.content.x - (grid.dimensions.content.x + 100.0)).abs() < 1e-3);
    assert!((c2.dimensions.content.x - (grid.dimensions.content.x + 200.0)).abs() < 1e-3);
}

#[test]
fn test_grid_explicit_and_negative_lines() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="grid"><div id="full">Full</div><div id="tail">Tail</div></div></body></html>"#;
    let css = r#"
        * { margin: 0px; padding: 0px; border: 0px; }
        #grid { display: grid; width: 300px; grid-template-columns: 100px 100px 100px; grid-gap: 0px; }
        #full { grid-column: 1 / -1; height: 40px; }
        #tail { grid-column: 2 / 4; height: 40px; }
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

    let grid = &layout.children[0].children[0].children[0];
    let full = &grid.children[0];
    let tail = &grid.children[1];

    // #full spans columns 1 to -1 (1 to 4 -> all 3 columns = 300px)
    assert_eq!(full.dimensions.content.x, grid.dimensions.content.x);
    assert_eq!(full.dimensions.content.width, 300.0);

    // #tail spans columns 2 to 4 (columns 2 and 3 = 200px)
    assert_eq!(tail.dimensions.content.x, grid.dimensions.content.x + 100.0);
    assert_eq!(tail.dimensions.content.width, 200.0);
}

#[test]
fn test_grid_2d_occupancy_auto_placement() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="grid">
        <div id="fixed">Fixed</div>
        <div id="auto1">A1</div>
        <div id="auto2">A2</div>
        <div id="auto3">A3</div>
    </div></body></html>"#;
    let css = r#"
        * { margin: 0px; padding: 0px; border: 0px; }
        #grid {
            display: grid;
            width: 300px;
            grid-template-columns: 100px 100px 100px;
            grid-template-rows: 50px 50px;
            grid-gap: 0px;
        }
        #fixed { grid-column: 2; grid-row: 1; height: 50px; }
        #auto1 { height: 50px; }
        #auto2 { height: 50px; }
        #auto3 { height: 50px; }
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

    let grid = &layout.children[0].children[0].children[0];
    let fixed = &grid.children[0];
    let a1 = &grid.children[1];
    let a2 = &grid.children[2];
    let a3 = &grid.children[3];

    let gx = grid.dimensions.content.x;
    let gy = grid.dimensions.content.y;

    // #fixed is at col 1, row 0
    assert_eq!(fixed.dimensions.content.x, gx + 100.0);
    assert_eq!(fixed.dimensions.content.y, gy);

    // #auto1 takes first free spot: col 0, row 0
    assert_eq!(a1.dimensions.content.x, gx);
    assert_eq!(a1.dimensions.content.y, gy);

    // #auto2 skips occupied col 1, row 0 and takes col 2, row 0
    assert_eq!(a2.dimensions.content.x, gx + 200.0);
    assert_eq!(a2.dimensions.content.y, gy);

    // #auto3 wraps to next row: col 0, row 1
    assert_eq!(a3.dimensions.content.x, gx);
    assert_eq!(a3.dimensions.content.y, gy + 50.0);
}

#[test]
fn test_grid_alignment_center_and_end() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html =
        r#"<html><body><div id="grid"><div id="i1"></div><div id="i2"></div></div></body></html>"#;
    let css = r#"
        * { margin: 0px; padding: 0px; border: 0px; }
        #grid {
            display: grid;
            width: 200px;
            grid-template-columns: 100px 100px;
            grid-template-rows: 100px;
            align-items: center;
            grid-gap: 0px;
        }
        #i1 { width: 80px; height: 40px; }
        #i2 { width: 80px; height: 60px; align-self: end; }
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

    let grid = &layout.children[0].children[0].children[0];
    let i1 = &grid.children[0];
    let i2 = &grid.children[1];

    let gy = grid.dimensions.content.y;

    // #i1 is aligned center in 100px row: (100 - 40) / 2 = 30px offset
    assert_eq!(i1.dimensions.content.y, gy + 30.0);

    // #i2 is aligned end: (100 - 60) = 40px offset
    assert_eq!(i2.dimensions.content.y, gy + 40.0);
}

#[test]
fn test_grid_minmax_flexible_track_sizing() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html =
        r#"<html><body><div id="grid"><div id="c1"></div><div id="c2"></div></div></body></html>"#;
    let css = r#"
        * { margin: 0px; padding: 0px; border: 0px; }
        #grid {
            display: grid;
            width: 400px;
            grid-template-columns: minmax(100px, 1fr) 1fr;
            grid-gap: 0px;
        }
        #c1, #c2 { height: 50px; }
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

    let grid = &layout.children[0].children[0].children[0];
    let c1 = &grid.children[0];
    let c2 = &grid.children[1];

    // Total space 400px, total_fr_col = 2.0.
    // minmax(100px, 1fr) receives max(100px, 400px * 0.5) = 200px.
    // c2 receives 400px * 0.5 = 200px.
    assert_eq!(c1.dimensions.content.width, 200.0);
    assert_eq!(c2.dimensions.content.width, 200.0);
}

#[test]
fn test_grid_minmax_flexible_track_preserves_min() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html =
        r#"<html><body><div id="grid"><div id="c1"></div><div id="c2"></div></div></body></html>"#;
    let css = r#"
        * { margin: 0px; padding: 0px; border: 0px; }
        #grid {
            display: grid;
            width: 150px;
            grid-template-columns: minmax(100px, 1fr) 1fr;
            grid-gap: 0px;
        }
        #c1, #c2 { height: 50px; }
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

    let grid = &layout.children[0].children[0].children[0];
    let c1 = &grid.children[0];
    let c2 = &grid.children[1];

    // Total space 150px, total_fr_col = 2.0.
    // c1 flex share is 75px, but min is 100px, so c1 gets max(100, 75) = 100px.
    // c2 gets 150 * 0.5 = 75px.
    assert_eq!(c1.dimensions.content.width, 100.0);
    assert_eq!(c2.dimensions.content.width, 75.0);
}

#[test]
fn test_grid_clamped_bounded_huge_placement() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html =
        r#"<html><body><div id="grid"><div id="c1"></div><div id="c2"></div></div></body></html>"#;
    let css = r#"
        * { margin: 0px; padding: 0px; border: 0px; }
        #grid {
            display: grid;
            width: 300px;
            grid-template-columns: 100px 100px 100px;
            grid-gap: 0px;
        }
        #c1 { grid-column: 1 / 1000000; height: 50px; }
        #c2 { grid-column: 999999; grid-row: 50000; height: 50px; }
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

    let grid = &layout.children[0].children[0].children[0];
    assert_eq!(grid.children.len(), 2);
    // Huge placement is bounded safely without crash or OOM
    let c1 = &grid.children[0];
    assert!(c1.dimensions.content.width > 0.0);
}
