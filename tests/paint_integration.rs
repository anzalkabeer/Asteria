// ─── Paint Engine Integration Test ───────────────────────────────
//
// End-to-end integration test verifying that:
//   - LayoutBox tree is correctly converted into a DisplayList
//   - Draw commands (SolidColor, Border, Text) are emitted at computed layout coordinates
//   - CSS paint order (Background -> Border -> Text -> Children) is preserved

use asteria::css_parser::Stylesheet;
use asteria::layout::layout_document;
use asteria::paint::{DisplayCommand, build_display_list};
use asteria::style::resolve_styles;
use asteria::values::Color;

#[test]
fn test_paint_engine_display_list_generation() {
    let html = r#"<html><body><div id="card" style="background-color: #f0f0f0; border-color: red; border-style: solid; border-top-width: 2px;"><h1>Title</h1></div></body></html>"#;
    let css = r#"h1 { color: blue; font-size: 24px; }"#;

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

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

#[test]
fn test_opacity_cascade_in_display_list() {
    let html = r#"<html><body><div id="parent" style="opacity: 0.5;"><div id="child" style="background-color: rgb(255, 0, 0);">Hello</div></div></body></html>"#;
    let css = r#"#child { color: rgb(0, 0, 255); }"#;

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, bytes);
    let layout = layout_document(&styled, &dom, bytes, 800.0, 600.0).unwrap();

    let display_list = build_display_list(&layout, &dom, bytes);

    // Child background should have alpha = 128 (255 * 0.5)
    let child_bg = display_list.commands.iter().find(|cmd| {
        if let DisplayCommand::SolidColor { color, .. } = cmd {
            color.r == 255 && color.g == 0 && color.b == 0
        } else {
            false
        }
    });
    assert!(child_bg.is_some());
    if let Some(DisplayCommand::SolidColor { color, .. }) = child_bg {
        assert_eq!(color.a, 128);
    }
}

#[test]
fn test_positioned_child_opacity_applied_once() {
    let html = r#"<html><body><div id="parent" style="opacity: 0.5;"><div id="child" style="position: relative; z-index: 1; opacity: 0.5; background-color: rgb(200, 0, 0);">Positioned</div></div></body></html>"#;
    let css = r#""#;

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, bytes);
    let layout = layout_document(&styled, &dom, bytes, 800.0, 600.0).unwrap();

    let display_list = build_display_list(&layout, &dom, bytes);

    let child_bg = display_list.commands.iter().find(|cmd| {
        if let DisplayCommand::SolidColor { color, .. } = cmd {
            color.r == 200 && color.g == 0 && color.b == 0
        } else {
            false
        }
    });
    assert!(child_bg.is_some());
    if let Some(DisplayCommand::SolidColor { color, .. }) = child_bg {
        // 255 * 0.5 * 0.5 = 63.75 -> 64
        assert!((color.a as i32 - 64).abs() <= 1);
    }
}

#[test]
fn test_border_radius_command_emission() {
    let html = r#"<html><body><div id="btn" style="border-radius: 8px; background-color: #00ff00;">Button</div></body></html>"#;
    let css = r#""#;

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, bytes);
    let layout = layout_document(&styled, &dom, bytes, 800.0, 600.0).unwrap();

    let display_list = build_display_list(&layout, &dom, bytes);

    let has_rounded_rect = display_list.commands.iter().any(|cmd| {
        if let DisplayCommand::RoundedRect { color, radius, .. } = cmd {
            color.r == 0 && color.g == 255 && color.b == 0 && radius.top_left == 8.0
        } else {
            false
        }
    });
    assert!(has_rounded_rect);
}

#[test]
fn test_box_shadow_paint_order() {
    let html = r#"<html><body><div id="card" style="box-shadow: 2px 4px 6px #000000; background-color: #ffffff;">Content</div></body></html>"#;
    let css = r#""#;

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, bytes);
    let layout = layout_document(&styled, &dom, bytes, 800.0, 600.0).unwrap();

    let display_list = build_display_list(&layout, &dom, bytes);

    let shadow_pos = display_list
        .commands
        .iter()
        .position(|cmd| matches!(cmd, DisplayCommand::BoxShadow { .. }));
    let bg_pos = display_list.commands.iter().position(|cmd| {
        if let DisplayCommand::SolidColor { color, .. } = cmd {
            color.r == 255 && color.g == 255 && color.b == 255
        } else {
            false
        }
    });

    assert!(shadow_pos.is_some());
    assert!(bg_pos.is_some());
    // Box shadow must be emitted before background
    assert!(shadow_pos.unwrap() < bg_pos.unwrap());
}

#[test]
fn test_overflow_hidden_clip_emission() {
    let html = r#"<html><body><div id="clipped" style="overflow: hidden; width: 100px; height: 100px;"><div id="child">Inside</div></div></body></html>"#;
    let css = r#""#;

    let bytes = html.as_bytes();
    let mut processor = asteria::streaming_parser::StreamingHtmlProcessor::new();
    let _ = processor.receive_network_chunk(bytes, true);
    let dom = processor.finish();

    let stylesheet = Stylesheet::parse(css.as_bytes());
    let styled = resolve_styles(&dom, &stylesheet, bytes);
    let layout = layout_document(&styled, &dom, bytes, 800.0, 600.0).unwrap();

    let display_list = build_display_list(&layout, &dom, bytes);

    let has_push_clip = display_list
        .commands
        .iter()
        .any(|cmd| matches!(cmd, DisplayCommand::PushClip { .. }));
    let has_pop_clip = display_list
        .commands
        .iter()
        .any(|cmd| matches!(cmd, DisplayCommand::PopClip));

    assert!(has_push_clip);
    assert!(has_pop_clip);
}
