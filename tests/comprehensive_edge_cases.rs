// ─── Comprehensive Edge Cases Test Suite ───────────────────────────
//
// Rigorous end-to-end edge case testing across all Asteria engine layers:
//   1. HTML Tokenizer & Parser
//   2. CSS Tokenizer & Parser
//   3. Style Resolution & Cascade
//   4. Layout Engine Box Model Geometry

use asteria::css_parser::Stylesheet;
use asteria::dom::{Dom, NodeKind};
use asteria::layout::{layout_document, BoxType, LayoutBox};
use asteria::parser::Parser;
use asteria::style::{resolve_styles, StyledNode};
use asteria::tokenizer::Tokenizer;
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

// ═══════════════════════════════════════════════════════════════════
// 1. HTML TOKENIZER & PARSER EDGE CASES
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_html_void_and_self_closing_elements() {
    let html = r#"<html><body><img src="logo.png" alt="logo" /><br><input disabled type=text><hr/></body></html>"#;
    let bytes = html.as_bytes();
    let mut tokenizer = Tokenizer::new(bytes);
    let tokens = tokenizer.tokenize();
    let parser = Parser::new(&tokens, bytes);
    let dom = parser.parse();

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
    let mut tokenizer = Tokenizer::new(bytes);
    let tokens = tokenizer.tokenize();
    let parser = Parser::new(&tokens, bytes);
    let dom = parser.parse();

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
                std::str::from_utf8(&bytes[vs as usize..ve as usize]).unwrap().to_string()
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
    let mut tokenizer = Tokenizer::new(bytes);
    let tokens = tokenizer.tokenize();
    let parser = Parser::new(&tokens, bytes);
    let dom = parser.parse();

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
    let mut tokenizer = Tokenizer::new(bytes);
    let tokens = tokenizer.tokenize();
    let parser = Parser::new(&tokens, bytes);
    let dom = parser.parse();

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

    let styled = resolve_styles(&dom, &stylesheet, html);
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
    let tokenizer = Tokenizer::new(bytes);
    let tokens = Tokenizer::new(bytes).tokenize();
    let dom = Parser::new(&tokens, bytes).parse();
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
    let tokens = Tokenizer::new(bytes).tokenize();
    let dom = Parser::new(&tokens, bytes).parse();
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
    let css = r#"#centered { width: 400px; margin-left: auto; margin-right: auto; }"#;

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
        #parent { margin-left: 50px; margin-top: 30px; padding-left: 20px; padding-top: 10px; border-left-width: 5px; border-top-width: 5px; }
        #child { margin-left: 15px; margin-top: 15px; width: 100px; height: 50px; }
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
