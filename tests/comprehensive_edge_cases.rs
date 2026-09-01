// ─── Comprehensive Edge Cases Test Suite ───────────────────────────
//
// Rigorous end-to-end edge case testing across all Asteria engine layers:
//   1. HTML Tokenizer & Parser
//   2. CSS Tokenizer & Parser
//   3. Style Resolution & Cascade
//   4. Layout Engine Box Model Geometry

use asteria::css_parser::Stylesheet;
use asteria::dom::{Dom, NodeKind};
use asteria::layout::{BoxType, LayoutBox, layout_document};
use asteria::paint::build_display_list;
use asteria::scene::build_scene_graph;
use asteria::style::{StyledNode, resolve_styles, resolve_styles_with_viewport};
use asteria::values::Color;

/// Helper: parse HTML + CSS → (LayoutBox, Dom, bytes, StyledNode)
fn parse_and_layout_full<'a>(
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

// ═══════════════════════════════════════════════════════════════════
// 1. HTML TOKENIZER & PARSER EDGE CASES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_html_void_and_self_closing_elements() {
    let html = r#"<html><body><img src="logo.png" alt="logo" /><br><input disabled type=text><hr/></body></html>"#;
    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    // Check DOM tree node count and structure: Document -> html -> body
    let html_id = dom.nodes[0].children[0];
    let body_id = dom.get(html_id).children[0];
    let body = dom.get(body_id);
    let children_kinds: Vec<&str> = body
        .children
        .iter()
        .map(|&cid| match &dom.get(cid).kind {
            NodeKind::Element { tag_start, tag_end } => {
                std::str::from_utf8(&bytes[*tag_start as usize..*tag_end as usize]).unwrap()
            }
            _ => "other",
        })
        .collect();

    assert_eq!(children_kinds, vec!["img", "br", "input", "hr"]);
}

#[test]
fn test_html_unquoted_single_quoted_boolean_attributes() {
    let html = r#"<div id=main class='card active' disabled data-ref="123">Content</div>"#;
    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let div = dom.get(dom.nodes[0].children[0]);
    assert_eq!(div.attributes.len(), 4);

    // Read attribute names and values
    let attrs: Vec<(String, String)> = div
        .attributes
        .iter()
        .map(|&(ns, ne, vs, ve)| {
            let name = std::str::from_utf8(&bytes[ns as usize..ne as usize]).unwrap();
            let val = if vs == 0 && ve == 0 {
                "".to_string()
            } else {
                std::str::from_utf8(&bytes[vs as usize..ve as usize])
                    .unwrap()
                    .to_string()
            };
            (name.to_string(), val)
        })
        .collect();

    assert_eq!(
        attrs,
        vec![
            ("id".to_string(), "main".to_string()),
            ("class".to_string(), "card active".to_string()),
            ("disabled".to_string(), "".to_string()),
            ("data-ref".to_string(), "123".to_string())
        ]
    );
}

#[test]
fn test_html_comments_with_dashes() {
    let html = r#"<div><!-- comment - with - dashes inside --></div>"#;
    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let div = dom.get(dom.nodes[0].children[0]);
    assert_eq!(div.children.len(), 1);
    let comment_node = dom.get(div.children[0]);
    if let NodeKind::Comment { start, end } = comment_node.kind {
        let content = std::str::from_utf8(&bytes[start as usize..end as usize]).unwrap();
        assert_eq!(content.trim(), "comment - with - dashes inside");
    } else {
        panic!("Expected comment node");
    }
}

#[test]
fn test_html_deeply_nested_structure() {
    let html = "<div><div><div><div><div><span>Deep</span></div></div></div></div></div>";
    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    // Verify 6 nested element levels + Document root
    let mut current_id = dom.root();
    let mut depth = 0;
    while !dom.get(current_id).children.is_empty() {
        current_id = dom.get(current_id).children[0];
        depth += 1;
    }
    // Document -> div1 -> div2 -> div3 -> div4 -> div5 -> span -> text
    assert_eq!(depth, 7);
}

// ═══════════════════════════════════════════════════════════════════
// 2. CSS TOKENIZER & PARSER EDGE CASES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_css_comments_and_whitespace_resilience() {
    let css = r#"
        /* Top level comment */
        h1 /* target header */ , p.intro {
            /* Property comment */
            color /* val */ : red ;
            background-color: #00ff00;
        }
    "#;
    let stylesheet = Stylesheet::parse(css.as_bytes());
    assert_eq!(stylesheet.rules.len(), 1);
    let rule = &stylesheet.rules[0];
    assert_eq!(rule.selectors.len(), 2);
    assert_eq!(rule.declarations.len(), 2);
    assert_eq!(rule.declarations[0].property, "color");
    assert_eq!(rule.declarations[0].value, "red");
}

#[test]
fn test_css_at_rule_skipping() {
    let css = r#"
        @import url("styles.css");
        @media print { body { color: black; } }
        div { width: 100px; }
    "#;
    let stylesheet = Stylesheet::parse(css.as_bytes());
    // Skips @import and @media without failing
    assert!(!stylesheet.rules.is_empty());
    let last_rule = stylesheet.rules.last().unwrap();
    assert_eq!(last_rule.declarations[0].property, "width");
}

#[test]
fn test_css_hex_color_variants() {
    let css = r#"
        .c1 { color: #f00; }
        .c2 { color: #00ff00; }
        .c3 { color: blue; }
        .c4 { color: transparent; }
    "#;
    let stylesheet = Stylesheet::parse(css.as_bytes());
    let dom = Dom::new(); // Dummy DOM
    let html = b"<html></html>";

    let _styled = resolve_styles(&dom, &stylesheet, html);
    // Parse verifies hex and named color handling
    assert_eq!(stylesheet.rules.len(), 4);
}

// ═══════════════════════════════════════════════════════════════════
// 3. STYLE ENGINE CASCADE & INHERITANCE EDGE CASES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_style_inline_attribute_override() {
    let html = r#"<html><body><div id="hero" class="card" style="color: green; width: 300px;">Text</div></body></html>"#;
    let css = r#"
        #hero { color: red; width: 100px; }
        .card { color: blue; width: 200px; }
        div { color: black; }
    "#;
    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();
    let stylesheet = Stylesheet::parse(css.as_bytes());

    let styled = resolve_styles(&dom, &stylesheet, bytes);
    let html_styled = &styled.children[0];
    let body_styled = &html_styled.children[0];
    let div_styled = &body_styled.children[0];

    // Inline style must win over #hero (color: green = rgb(0, 128, 0))
    assert_eq!(div_styled.styles.color, Color::rgb(0, 128, 0));
    assert_eq!(
        div_styled.styles.width,
        asteria::values::LengthOrPercentage::Px(300.0)
    );
}

#[test]
fn test_style_em_rem_percent_cascade() {
    let html = r#"<html><body style="font-size: 20px;"><div style="font-size: 1.5em;"><p style="font-size: 0.5em;">Nested</p></div></body></html>"#;
    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();
    let stylesheet = Stylesheet::parse(b"");

    let styled = resolve_styles(&dom, &stylesheet, bytes);
    let body_styled = &styled.children[0].children[0];
    let div_styled = &body_styled.children[0];
    let p_styled = &div_styled.children[0];

    // Body: 20px
    assert_eq!(body_styled.styles.font_size, 20.0);
    // Div: 1.5 * 20 = 30px
    assert_eq!(div_styled.styles.font_size, 30.0);
    // P: 0.5 * 30 = 15px
    assert_eq!(p_styled.styles.font_size, 15.0);
}

// ═══════════════════════════════════════════════════════════════════
// 4. LAYOUT ENGINE EDGE CASES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_layout_margin_auto_horizontal_centering() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="centered"></div></body></html>"#;
    let css = r#"body { margin: 0; } #centered { width: 400px; margin-left: auto; margin-right: auto; padding: 0; border: 0px; }"#;

    let layout = parse_and_layout_full(
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
    let div_box = &body_box.children[0];

    // Viewport width = 800px, box width = 400px
    // Remaining space = 400px -> split equally -> margin-left = 200px, margin-right = 200px
    assert_eq!(div_box.dimensions.content.width, 400.0);
    assert_eq!(div_box.dimensions.margin.left, 200.0);
    assert_eq!(div_box.dimensions.margin.right, 200.0);
    assert_eq!(div_box.dimensions.content.x, 200.0);
}

#[test]
fn test_layout_nested_coordinate_accumulation() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="parent"><div id="child"></div></div></body></html>"#;
    let css = r#"
        body { margin: 0; padding: 0; }
        #parent { margin-left: 50px; margin-top: 30px; padding-left: 20px; padding-top: 10px; border-left-width: 5px; border-top-width: 5px; border-right-width: 0px; border-bottom-width: 0px; }
        #child { margin-left: 15px; margin-top: 15px; width: 100px; height: 50px; border-left-width: 0px; border-top-width: 0px; border-right-width: 0px; border-bottom-width: 0px; padding: 0px; }
    "#;

    let layout = parse_and_layout_full(
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

    // Parent content position:
    // x = 0 + 50 (margin) + 5 (border) + 20 (padding) = 75px
    // y = 0 + 30 (margin) + 5 (border) + 10 (padding) = 45px
    assert_eq!(parent_box.dimensions.content.x, 75.0);
    assert_eq!(parent_box.dimensions.content.y, 45.0);

    // Child content position:
    // x = 75 (parent content x) + 15 (child margin) = 90px
    // y = 45 (parent content y) + 15 (child margin) = 60px
    assert_eq!(child_box.dimensions.content.x, 90.0);
    assert_eq!(child_box.dimensions.content.y, 60.0);
}

#[test]
fn test_layout_anonymous_block_generation() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div><span>Inline 1</span><div>Block</div><span>Inline 2</span></div></body></html>"#;
    let css = r#""#;

    let layout = parse_and_layout_full(
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
    let outer_div = &body_box.children[0];

    // Outer div contains a mix of inline and block elements:
    // <span>Inline 1</span> -> AnonymousBlock 1
    // <div>Block</div>     -> BlockNode
    // <span>Inline 2</span> -> AnonymousBlock 2
    assert_eq!(outer_div.children.len(), 3);
    assert_eq!(outer_div.children[0].box_type, BoxType::AnonymousBlock);
    assert_eq!(outer_div.children[1].box_type, BoxType::BlockNode);
    assert_eq!(outer_div.children[2].box_type, BoxType::AnonymousBlock);
}

#[test]
fn test_layout_explicit_height_override() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="fixed-height"><div>Child 1</div><div>Child 2</div></div></body></html>"#;
    let css = r#"#fixed-height { height: 350px; }"#;

    let layout = parse_and_layout_full(
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
    let container_box = &body_box.children[0];

    // Explicit height of 350px overrides the natural sum of child heights
    assert_eq!(container_box.dimensions.content.height, 350.0);
}

// ═══════════════════════════════════════════════════════════════════
// 5. SECTION 8 TEST COVERAGE EXTENSIONS
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_full_pipeline_parse_style_layout_paint_scene() {
    let html = r#"<html><head></head><body><div id="card" style="background-color: #ff0000; margin: 10px; padding: 20px;"><a href="https://example.com"><h1>Header</h1></a><p>Content text</p></div></body></html>"#;
    let css = r#"
        body { margin: 0; padding: 0; }
        h1 { font-size: 20px; color: #00ff00; }
        p { font-size: 14px; color: #0000ff; }
    "#;

    let bytes = html.as_bytes().to_vec();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(&bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, &bytes);

    let layout_root = layout_document(&styled, &dom, &bytes, 1024.0, 768.0).unwrap();

    // 2. Paint -> DisplayList
    let display_list = build_display_list(&layout_root, &dom, &bytes);
    assert!(!display_list.commands.is_empty());

    // 3. Scene Graph
    let scene = build_scene_graph(&display_list, 256.0);
    assert!(!scene.is_empty());

    // Verify solid color command exists for the card background
    let has_bg_rect = display_list.commands.iter().any(|cmd| {
        matches!(cmd, asteria::paint::DisplayCommand::SolidColor { color, .. } if *color == Color::rgb(255, 0, 0))
    });
    assert!(
        has_bg_rect,
        "Expected red background rectangle in display list"
    );

    // Verify text command with link URL attached
    let has_linked_text = display_list.commands.iter().any(|cmd| {
        matches!(cmd, asteria::paint::DisplayCommand::Text { text, link_url: Some(url), .. } if text == "Header" && url == "https://example.com")
    });
    assert!(
        has_linked_text,
        "Expected linked Header text in display list"
    );
}

#[test]
fn test_shorthand_expansion_and_cascade_override() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="box">Box</div></body></html>"#;
    let css = r#"
        #box {
            margin: 10px 20px;
            margin-top: 50px;
            padding: 5px 10px 15px 20px;
            width: 100px;
            height: 100px;
        }
    "#;

    let layout = parse_and_layout_full(
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
    let box_elem = &body_box.children[0];
    let style = box_elem.styled_node.unwrap();

    // Margin: margin-top was explicitly overridden to 50px, while others come from shorthand
    assert_eq!(style.styles.margin.top, Some(50.0));
    assert_eq!(style.styles.margin.right, Some(20.0));
    assert_eq!(style.styles.margin.bottom, Some(10.0));
    assert_eq!(style.styles.margin.left, Some(20.0));

    // Padding: 5px top, 10px right, 15px bottom, 20px left
    assert_eq!(style.styles.padding.top, 5.0);
    assert_eq!(style.styles.padding.right, 10.0);
    assert_eq!(style.styles.padding.bottom, 15.0);
    assert_eq!(style.styles.padding.left, 20.0);
}

#[test]
fn test_css_var_substitution_and_fallbacks() {
    let html = r#"<html><body><div id="target">Text</div></body></html>"#;
    let css = r#"
        :root {
            --custom-color: #ff5500;
        }
        #target {
            color: var(--custom-color, #000000);
            background-color: var(--undefined-var, #00ff00);
        }
    "#;

    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let layout = parse_and_layout_full(
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
    let target_box = &body_box.children[0];
    let style = target_box.styled_node.unwrap();

    // Variable substituted
    assert_eq!(style.styles.color, Color::rgb(255, 85, 0));
    // Missing variable resolved to fallback value
    assert_eq!(style.styles.background_color, Color::rgb(0, 255, 0));
}

#[test]
fn test_media_query_viewport_boundary_conditions() {
    let html = r#"<html><body><div id="responsive">Responsive</div></body></html>"#;
    let css = r#"
        #responsive { color: #000000; }
        @media (min-width: 600px) {
            #responsive { color: #ff0000; }
        }
    "#;

    let bytes = html.as_bytes().to_vec();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(&bytes, true);
    let dom = processor.finish();
    let stylesheet = Stylesheet::parse(css.as_bytes());

    // 1. Viewport width 599px -> Under min-width (rule inactive)
    let styled_599 = resolve_styles_with_viewport(&dom, &stylesheet, &bytes, 599.0);
    let target_599 = &styled_599.children[0].children[0].children[0];
    assert_eq!(target_599.styles.color, Color::rgb(0, 0, 0));

    // 2. Viewport width 600px -> Exact boundary (rule active)
    let styled_600 = resolve_styles_with_viewport(&dom, &stylesheet, &bytes, 600.0);
    let target_600 = &styled_600.children[0].children[0].children[0];
    assert_eq!(target_600.styles.color, Color::rgb(255, 0, 0));

    // 3. Viewport width 601px -> Above min-width (rule active)
    let styled_601 = resolve_styles_with_viewport(&dom, &stylesheet, &bytes, 601.0);
    let target_601 = &styled_601.children[0].children[0].children[0];
    assert_eq!(target_601.styles.color, Color::rgb(255, 0, 0));
}

#[test]
fn test_malformed_html_tokenizer_fuzzing() {
    // Malformed HTML inputs that must not panic or loop infinitely
    let malformed_inputs = [
        "<<<<div >>>>>",
        "<div class=\"unclosed string",
        "<!-- unclosed comment --",
        "<!doctype html <html <body >",
        "<div <span <<p>Text</div",
        "<div>\0\0\0<p>Null bytes</p></div>",
        "<&*!#$%>",
        "<a href=\"\"\"\"\"\" target=\"_blank\">Link</a>",
        "<div id=123 class=abc style=\"color: red;\">Test</div>",
    ];

    for input in malformed_inputs {
        let bytes = input.as_bytes();
        let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
        let _ = processor.receive_network_chunk(bytes, true);
        let dom = processor.finish();
        // Ensure DOM root document exists and parsing succeeded without panic
        assert!(!dom.nodes.is_empty());
    }
}

#[test]
fn test_attribute_selector_operators() {
    let html = "<html><body><div class=\"btn-primary\">Prefix</div><div class=\"card-large\">Substring</div><a href=\"doc.png\">Suffix</a><span data-tags=\"news tech featured\">Word</span><p lang=\"en-US\">Dash</p></body></html>";
    let css = r#"
        [class^="btn-"] { color: rgb(255, 0, 0); }
        [class*="-large"] { color: rgb(0, 255, 0); }
        [href$=".png"] { color: rgb(0, 0, 255); }
        [data-tags~="tech"] { font-size: 20px; }
        [lang|="en"] { line-height: 28px; }
    "#;

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();
    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, bytes);

    let body = &styled.children[0].children[0];
    let btn = &body.children[0];
    let card = &body.children[1];
    let link = &body.children[2];
    let span = &body.children[3];
    let p = &body.children[4];

    assert_eq!(btn.styles.color, Color::rgb(255, 0, 0));
    assert_eq!(card.styles.color, Color::rgb(0, 255, 0));
    assert_eq!(link.styles.color, Color::rgb(0, 0, 255));
    assert_eq!(span.styles.font_size, 20.0);
    assert_eq!(p.styles.line_height, 28.0);
}

#[test]
fn test_html_entity_decoding() {
    assert_eq!(
        asteria::dom::decode_html_entities("Hello &amp; World"),
        "Hello & World"
    );
    assert_eq!(asteria::dom::decode_html_entities("&lt;div&gt;"), "<div>");
    assert_eq!(
        asteria::dom::decode_html_entities("&quot;quoted&quot;"),
        "\"quoted\""
    );
    assert_eq!(
        asteria::dom::decode_html_entities("It&#39;s work"),
        "It's work"
    );
    assert_eq!(
        asteria::dom::decode_html_entities("&#x41;&#x42;&#x43;"),
        "ABC"
    );
    assert_eq!(
        asteria::dom::decode_html_entities("No entities here"),
        "No entities here"
    );
}

#[test]
fn test_relative_font_weight() {
    use asteria::values::parse_font_weight_relative;
    assert_eq!(parse_font_weight_relative("lighter", 400.0), 300.0);
    assert_eq!(parse_font_weight_relative("lighter", 100.0), 100.0); // minimum clamp
    assert_eq!(parse_font_weight_relative("bolder", 400.0), 500.0);
    assert_eq!(parse_font_weight_relative("bolder", 900.0), 900.0); // maximum clamp
    assert_eq!(parse_font_weight_relative("bold", 400.0), 700.0);
    assert_eq!(parse_font_weight_relative("normal", 700.0), 400.0);
}

#[test]
fn test_keyframe_modulo_looping() {
    use asteria::animation::{ActiveAnimation, AnimationManager};
    use asteria::dom::NodeId;
    use asteria::values::AnimationTimingFunction;

    let mut manager = AnimationManager::new();
    let anim = ActiveAnimation {
        node_id: NodeId(1),
        name: "spin".to_string(),
        duration: 2.0,
        elapsed: 0.0,
        iteration_count: 0.0, // Infinite loop
        timing_function: AnimationTimingFunction::Linear,
    };
    manager.start_animation(anim);

    // After 1 second (50% into first cycle)
    let updates = manager.tick(1.0);
    assert!((updates[0].2 - 0.5).abs() < 1e-4);

    // After 2 more seconds (total 3s -> 50% into second cycle)
    let updates2 = manager.tick(2.0);
    assert!((updates2[0].2 - 0.5).abs() < 1e-4);
}

#[test]
fn test_vertical_margin_collapsing() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="box1"></div><div id="box2"></div></body></html>"#;
    let css = r#"
        body { margin: 0; padding: 0; }
        #box1 { height: 50px; margin-top: 10px; margin-bottom: 30px; padding: 0; border-width: 0; }
        #box2 { height: 40px; margin-top: 20px; margin-bottom: 10px; padding: 0; border-width: 0; }
    "#;

    let layout = parse_and_layout_full(
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
    let box1 = &body_box.children[0];
    let box2 = &body_box.children[1];

    // Box 1 starts at y = 10px, height = 50px -> bottom at y = 60px
    assert_eq!(box1.dimensions.content.y, 10.0);
    assert_eq!(box1.dimensions.content.height, 50.0);

    // Box 2 top margin is 20px, Box 1 bottom margin is 30px.
    // Collapsed margin = max(30, 20) = 30px.
    // Box 2 starts at 60 + 30 = 90px.
    assert_eq!(box2.dimensions.content.y, 90.0);
    assert_eq!(box2.dimensions.content.height, 40.0);
}

#[test]
fn test_https_host_header_port_omission() {
    use asteria::net::http::{HttpMethod, HttpRequest, Url};

    let url = Url::parse("https://example.com/api").unwrap();
    let req = HttpRequest {
        method: HttpMethod::Get,
        url,
        headers: vec![],
    };
    let raw = String::from_utf8(req.to_request_bytes()).unwrap();

    assert!(raw.contains("Host: example.com\r\n"));
    assert!(!raw.contains("Host: example.com:443\r\n"));
}

#[test]
fn test_grid_gap_property_display() {
    use asteria::properties::PropertyId;
    use asteria::values::ComputedStyle;

    let mut style = ComputedStyle::default();
    style.set_property(PropertyId::GridGap, "10px", 16.0, 16.0);
    assert_eq!(style.get_property_display(PropertyId::GridGap), "10px");

    style.set_property(PropertyId::GridGap, "15px 25px", 16.0, 16.0);
    assert_eq!(style.get_property_display(PropertyId::GridGap), "15px 25px");
}

#[test]
fn test_margin_collapsing_negative_and_mixed() {
    use asteria::layout::collapse_margins;

    // CSS 2.1 §8.3.1 rules:
    // Positive + Positive: max(p1, p2)
    assert_eq!(collapse_margins(20.0, 30.0), 30.0);
    // Positive + Negative: max(pos) + min(neg)
    assert_eq!(collapse_margins(20.0, -15.0), 5.0);
    assert_eq!(collapse_margins(-15.0, 20.0), 5.0);
    assert_eq!(collapse_margins(10.0, -25.0), -15.0);
    // Negative + Negative: min(neg) (the most negative)
    assert_eq!(collapse_margins(-10.0, -25.0), -25.0);
    assert_eq!(collapse_margins(-30.0, -10.0), -30.0);
    // Zeros
    assert_eq!(collapse_margins(0.0, -10.0), -10.0);
    assert_eq!(collapse_margins(10.0, 0.0), 10.0);
    assert_eq!(collapse_margins(0.0, 0.0), 0.0);

    // Layout execution test for mixed & negative margin collapsing
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html = r#"<html><body><div id="b1"></div><div id="b2"></div></body></html>"#;
    let css = r#"
        body { margin: 0; padding: 0; }
        #b1 { height: 50px; margin-top: 10px; margin-bottom: 20px; padding: 0; border-width: 0; }
        #b2 { height: 40px; margin-top: -15px; margin-bottom: 0; padding: 0; border-width: 0; }
    "#;

    let layout = parse_and_layout_full(
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
    let b1 = &body_box.children[0];
    let b2 = &body_box.children[1];

    // b1: starts at y = 10px, height = 50px -> bottom at y = 60px
    assert_eq!(b1.dimensions.content.y, 10.0);
    assert_eq!(b1.dimensions.content.height, 50.0);

    // Collapsed margin = collapse_margins(20, -15) = 5px.
    // b2: starts at 60 + 5 = 65px.
    assert_eq!(b2.dimensions.content.y, 65.0);
    assert_eq!(b2.dimensions.content.height, 40.0);
}

#[test]
fn test_grid_gap_four_value_and_parsing() {
    use asteria::properties::PropertyId;
    use asteria::values::{ComputedStyle, Edges, parse_gap};

    // 1-value parse
    let g1 = parse_gap("12px", 16.0, 16.0);
    assert_eq!(g1, Edges::uniform(12.0));

    // 2-value parse
    let g2 = parse_gap("10px 20px", 16.0, 16.0);
    assert_eq!(
        g2,
        Edges {
            top: 10.0,
            right: 20.0,
            bottom: 10.0,
            left: 20.0,
        }
    );

    // Invalid (>2 values) parse returns zero
    let g_inv = parse_gap("10px 20px 30px", 16.0, 16.0);
    assert_eq!(g_inv, Edges::ZERO);

    // 4-value custom edge representation serialization
    let style = ComputedStyle {
        grid_gap: Edges {
            top: 1.0,
            right: 2.0,
            bottom: 3.0,
            left: 4.0,
        },
        ..Default::default()
    };
    assert_eq!(
        style.get_property_display(PropertyId::GridGap),
        "1px 2px 3px 4px"
    );
}

#[test]
fn test_margin_auto_vs_explicit_zero_centering() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    let html =
        r#"<html><body><div id="centered"></div><div id="left_aligned"></div></body></html>"#;
    let css = r#"
        body { margin: 0; padding: 0; }
        #centered { width: 400px; height: 50px; margin-left: auto; margin-right: auto; padding: 0; border-width: 0; }
        #left_aligned { width: 400px; height: 50px; margin-left: 0; margin-right: auto; padding: 0; border-width: 0; }
    "#;

    let layout = parse_and_layout_full(
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
    let centered = &body_box.children[0];
    let left_aligned = &body_box.children[1];

    // Centered: 800 - 400 = 400 underflow / 2 = 200px each margin
    assert_eq!(centered.dimensions.margin.left, 200.0);
    assert_eq!(centered.dimensions.margin.right, 200.0);
    assert_eq!(centered.dimensions.content.x, 200.0);

    // Left aligned: margin-left is explicit 0, margin-right absorbs all 400px underflow
    assert_eq!(left_aligned.dimensions.margin.left, 0.0);
    assert_eq!(left_aligned.dimensions.margin.right, 400.0);
    assert_eq!(left_aligned.dimensions.content.x, 0.0);
}

#[test]
fn test_horizontal_constraint_negative_underflow_residual() {
    let mut dom_store = None;
    let mut bytes_store = Vec::new();
    let mut styled_store = None;

    // Element wider than containing block: 900px in 800px viewport with margin-left: 50px, margin-right: 50px
    let html = r#"<html><body><div id="overflowing"></div></body></html>"#;
    let css = r#"
        body { margin: 0; padding: 0; }
        #overflowing { width: 900px; height: 50px; margin-left: 50px; margin-right: 50px; padding: 0; border-width: 0; }
    "#;

    let layout = parse_and_layout_full(
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
    let el = &body_box.children[0];

    // ml = 50, mr = 50, w = 900 -> total = 1000px in 800px container -> underflow = -200px
    // Over-constrained: margin-right becomes mr + underflow = 50 + (-200) = -150px
    assert_eq!(el.dimensions.content.width, 900.0);
    assert_eq!(el.dimensions.margin.left, 50.0);
    assert_eq!(el.dimensions.margin.right, -150.0);
    // Constraint equation: 50 + 900 + (-150) = 800px exactly satisfies constraint!
    assert_eq!(
        el.dimensions.margin.left + el.dimensions.content.width + el.dimensions.margin.right,
        800.0
    );
}

#[test]
fn test_z_index_stacking_order_in_display_list() {
    let html = r#"<html><body>
        <div id="container">
            <div id="pos_high" style="position: relative; z-index: 10; background-color: rgb(1, 1, 1);"></div>
            <div id="in_flow" style="background-color: rgb(2, 2, 2);"></div>
            <div id="pos_neg" style="position: relative; z-index: -5; background-color: rgb(3, 3, 3);"></div>
            <div id="pos_low" style="position: relative; z-index: 2; background-color: rgb(4, 4, 4);"></div>
        </div>
    </body></html>"#;
    let css = "div { width: 100px; height: 20px; }";

    let bytes = html.as_bytes().to_vec();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(&bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, &bytes);
    let layout = asteria::layout::layout_document(&styled, &dom, &bytes, 800.0, 600.0).unwrap();

    let display_list = asteria::paint::build_display_list(&layout, &dom, &bytes);

    // Extract SolidColor command colors in paint order
    let colors: Vec<Color> = display_list
        .commands
        .iter()
        .filter_map(|cmd| match cmd {
            asteria::paint::DisplayCommand::SolidColor { color, .. } => Some(*color),
            _ => None,
        })
        .collect();

    // Stacking order within container:
    // 1. pos_neg (z-index: -5) -> rgb(3, 3, 3)
    // 2. in_flow (normal flow) -> rgb(2, 2, 2)
    // 3. pos_low (z-index: 2) -> rgb(4, 4, 4)
    // 4. pos_high (z-index: 10) -> rgb(1, 1, 1)
    let relevant_colors: Vec<Color> = colors
        .into_iter()
        .filter(|c| *c != Color::rgb(248, 250, 252)) // filter UA body background if any
        .collect();

    assert_eq!(
        relevant_colors,
        vec![
            Color::rgb(3, 3, 3), // negative z-index painted first
            Color::rgb(2, 2, 2), // in-flow content painted next
            Color::rgb(4, 4, 4), // lower positive z-index
            Color::rgb(1, 1, 1), // higher positive z-index painted on top
        ]
    );
}

#[test]
fn test_hr_user_agent_stylesheet_defaults() {
    let html = "<html><body><hr id=\"divider\"></body></html>";
    let css = "";

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, bytes);

    let html_node = &styled.children[0];
    let body_node = &html_node.children[0];
    let hr_node = &body_node.children[0];

    assert_eq!(hr_node.styles.display, asteria::values::Display::Block);
    assert_eq!(
        hr_node.styles.border_style,
        asteria::values::BorderStyleValue::Solid
    );
    assert_eq!(
        hr_node.styles.height,
        asteria::values::LengthOrPercentage::Px(0.0)
    );
    assert_eq!(hr_node.styles.margin.top, Some(8.0));
    assert_eq!(hr_node.styles.margin.bottom, Some(8.0));
}

#[test]
fn test_shorthand_em_font_size_dependency_ordering() {
    let html = "<html><body><div id=\"box\">Text</div></body></html>";
    // Element has font-size: 20px and margin: 2em
    // 2em in margin must resolve against the element's own computed font-size (20px * 2 = 40px), not parent default (16px)
    let css = "#box { font-size: 20px; margin: 2em; padding: 1.5em; }";

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, bytes);

    let html_node = &styled.children[0];
    let body_node = &html_node.children[0];
    let box_node = &body_node.children[0];

    assert_eq!(box_node.styles.font_size, 20.0);
    assert_eq!(box_node.styles.margin.top, Some(40.0));
    assert_eq!(box_node.styles.margin.right, Some(40.0));
    assert_eq!(box_node.styles.margin.bottom, Some(40.0));
    assert_eq!(box_node.styles.margin.left, Some(40.0));
    assert_eq!(box_node.styles.padding.top, 30.0);
    assert_eq!(box_node.styles.padding.left, 30.0);
}

#[test]
fn test_length_or_percentage_and_z_index_parsing() {
    use asteria::values::{
        LengthOrPercentage, ZIndex, parse_length_or_percentage, parse_z_index, try_parse_z_index,
    };

    assert_eq!(parse_z_index("auto"), None);
    assert_eq!(parse_z_index("10"), Some(10));
    assert_eq!(parse_z_index("-5"), Some(-5));
    assert_eq!(parse_z_index("invalid"), None);

    assert_eq!(try_parse_z_index("auto"), Some(ZIndex::Auto));
    assert_eq!(try_parse_z_index("42"), Some(ZIndex::Integer(42)));
    assert_eq!(try_parse_z_index("-9"), Some(ZIndex::Integer(-9)));
    assert_eq!(try_parse_z_index("invalid"), None);

    assert_eq!(
        parse_length_or_percentage("auto", 16.0, 16.0),
        LengthOrPercentage::Auto
    );
    assert_eq!(
        parse_length_or_percentage("50%", 16.0, 16.0),
        LengthOrPercentage::Percentage(50.0)
    );
    assert_eq!(
        parse_length_or_percentage("25px", 16.0, 16.0),
        LengthOrPercentage::Px(25.0)
    );
    assert_eq!(
        parse_length_or_percentage("2em", 20.0, 16.0),
        LengthOrPercentage::Px(40.0)
    );
}

#[test]
fn test_percentage_width_and_height_layout_resolution() {
    let html = "<html><body><div id=\"parent\"><div id=\"child\">Inner</div></div></body></html>";
    let css = "#parent { width: 800px; height: 400px; display: block; } #child { width: 50%; height: 25%; display: block; }";

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, bytes);
    let layout_root = layout_document(&styled, &dom, bytes, 800.0, 600.0).unwrap();

    let html_box = &layout_root.children[0];
    let body_box = &html_box.children[0];
    let parent_box = &body_box.children[0];
    let child_box = &parent_box.children[0];

    assert_eq!(parent_box.dimensions.content.width, 800.0);
    assert_eq!(parent_box.dimensions.content.height, 400.0);
    assert_eq!(child_box.dimensions.content.width, 400.0); // 50% of 800px
    assert_eq!(child_box.dimensions.content.height, 100.0); // 25% of 400px
}

#[test]
fn test_hr_partial_margin_override() {
    let html = "<html><body><hr id=\"divider\" style=\"margin-top: 24px;\"></body></html>";
    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(b"");
    let styled = resolve_styles(&dom, &stylesheet, bytes);

    let html_node = &styled.children[0];
    let body_node = &html_node.children[0];
    let hr_node = &body_node.children[0];

    // Specified margin-top is preserved
    assert_eq!(hr_node.styles.margin.top, Some(24.0));
    // Default margin-bottom (8px) is still applied independently
    assert_eq!(hr_node.styles.margin.bottom, Some(8.0));
}

#[test]
fn test_invalid_z_index_cascade_discard() {
    let html = "<html><body><div id=\"target\" class=\"item\">Content</div></body></html>";
    // .item sets valid z-index: 10
    // Later rule sets invalid z-index: not-a-number, which should be discarded and not override 10
    let css = ".item { z-index: 10; } #target { z-index: not-a-number; }";

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, bytes);

    let html_node = &styled.children[0];
    let body_node = &html_node.children[0];
    let target_node = &body_node.children[0];

    assert_eq!(target_node.styles.z_index, Some(10));
}

#[test]
fn test_nested_positioned_descendants_deferred_in_stacking_context() {
    let html = r#"<html><body style="position: relative;">
        <div id="in-flow-container">
            <p id="in-flow-text">Normal Text</p>
            <div id="nested-pos" style="position: absolute; z-index: 10; background-color: rgb(255, 0, 0);">Nested Pos</div>
        </div>
        <div id="later-in-flow" style="background-color: rgb(0, 255, 0);">Later In Flow</div>
    </body></html>"#;

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(b"");
    let styled = resolve_styles(&dom, &stylesheet, bytes);
    let layout_root = layout_document(&styled, &dom, bytes, 800.0, 600.0).unwrap();
    let display_list = asteria::paint::build_display_list(&layout_root, &dom, bytes);

    let mut red_rect_idx = None;
    let mut green_rect_idx = None;

    for (idx, cmd) in display_list.commands.iter().enumerate() {
        if let asteria::paint::DisplayCommand::SolidColor { color, .. } = cmd {
            if color.r == 255 && color.g == 0 && color.b == 0 {
                red_rect_idx = Some(idx);
            } else if color.r == 0 && color.g == 255 && color.b == 0 {
                green_rect_idx = Some(idx);
            }
        }
    }

    assert!(
        red_rect_idx.is_some(),
        "Red rect (nested-pos) should be in display list"
    );
    assert!(
        green_rect_idx.is_some(),
        "Green rect (later-in-flow) should be in display list"
    );
    // Positioned element with z-index: 10 should be painted AFTER in-flow green element
    assert!(
        red_rect_idx.unwrap() > green_rect_idx.unwrap(),
        "Positioned z-index 10 should paint after normal in-flow content"
    );
}

#[test]
fn test_tls_parse_server_name_control_char_rejection() {
    use asteria::net::tls::TlsConnector;

    assert!(TlsConnector::parse_server_name("example.com").is_ok());
    assert!(TlsConnector::parse_server_name("example.com\n").is_err());
    assert!(TlsConnector::parse_server_name("example.com\r\n").is_err());
    assert!(TlsConnector::parse_server_name("\texample.com").is_err());
    assert!(TlsConnector::parse_server_name("ex\0ample.com").is_err());
}
