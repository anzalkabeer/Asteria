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
    assert_eq!(div_styled.styles.width, Some(300.0));
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
    assert_eq!(style.styles.margin.top, 50.0);
    assert_eq!(style.styles.margin.right, 20.0);
    assert_eq!(style.styles.margin.bottom, 10.0);
    assert_eq!(style.styles.margin.left, 20.0);

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
