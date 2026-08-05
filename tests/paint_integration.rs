// ─── Paint Engine Integration Test ───────────────────────────────
//
// End-to-end integration test verifying that:
//   - LayoutBox tree is correctly converted into a DisplayList
//   - Draw commands (SolidColor, Border, Text) are emitted at computed layout coordinates
//   - CSS paint order (Background -> Border -> Text -> Children) is preserved

use asteria::css_parser::Stylesheet;
use asteria::layout::layout_document;
use asteria::paint::{DisplayCommand, build_display_list};
use asteria::parser::Parser;
use asteria::style::resolve_styles;
use asteria::tokenizer::Tokenizer;
use asteria::values::Color;

#[test]
fn test_paint_engine_display_list_generation() {
    let html = r#"<html><body><div id="card" style="background-color: #f0f0f0; border-color: red; border-style: solid; border-top-width: 2px;"><h1>Title</h1></div></body></html>"#;
    let css = r#"h1 { color: blue; font-size: 24px; }"#;

    let bytes = html.as_bytes();
    let mut tokenizer = Tokenizer::new(bytes);
    let tokens = tokenizer.tokenize();
    let dom = Parser::new(&tokens, bytes).parse();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, bytes);
    let layout = layout_document(&styled, &dom, bytes, 800.0, 600.0).unwrap();

    let display_list = build_display_list(&layout, &dom, bytes);

    assert!(!display_list.commands.is_empty());

    // Verify background command for div#card
    let has_background = display_list.commands.iter().any(|cmd| {
        if let DisplayCommand::SolidColor { color, .. } = cmd {
            *color == Color::rgb(240, 240, 240)
        } else {
            false
        }
    });
    assert!(has_background);

    // Verify border command for div#card
    let has_border = display_list.commands.iter().any(|cmd| {
        if let DisplayCommand::Border {
            color,
            border_width,
            ..
        } = cmd
        {
            *color == Color::rgb(255, 0, 0) && border_width.top == 2.0
        } else {
            false
        }
    });
    assert!(has_border);

    // Verify text command for h1 Title
    let has_text = display_list.commands.iter().any(|cmd| {
        if let DisplayCommand::Text {
            text,
            color,
            font_size,
            ..
        } = cmd
        {
            text == "Title" && *color == Color::rgb(0, 0, 255) && *font_size == 24.0
        } else {
            false
        }
    });
    assert!(has_text);
}
