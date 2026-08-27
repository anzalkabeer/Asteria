// ─── Phase 5: Display List Paint Engine ───────────────────────────
//
// Milestone 10: HTML <a> link URL extraction and propagation to DisplayCommands.
//
// Converts a 2D LayoutBox tree into a flat, ordered DisplayList containing
// visual draw commands: SolidColor, Border, Text, and Image.
//
// CSS Paint Order per box:
//   1. Background (SolidColor)
//   2. Borders (Border)
//   3. Text content (Text)
//   4. Child layout boxes (recursive)

use std::fmt;

use crate::dom::{Dom, NodeId};
use crate::layout::{EdgeSizes, LayoutBox, Rect};
use crate::values::Color;

// ─── Display Commands ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum DisplayCommand {
    SolidColor {
        color: Color,
        rect: Rect,
        link_url: Option<String>,
    },
    Border {
        color: Color,
        rect: Rect,
        border_width: EdgeSizes,
        link_url: Option<String>,
    },
    Text {
        text: String,
        x: f32,
        y: f32,
        target_width: f32,
        font_size: f32,
        line_height: f32,
        color: Color,
        link_url: Option<String>,
    },
    Image {
        image_id: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        link_url: Option<String>,
    },
}

impl DisplayCommand {
    pub fn link_url(&self) -> Option<&str> {
        match self {
            DisplayCommand::SolidColor { link_url, .. } => link_url.as_deref(),
            DisplayCommand::Border { link_url, .. } => link_url.as_deref(),
            DisplayCommand::Text { link_url, .. } => link_url.as_deref(),
            DisplayCommand::Image { link_url, .. } => link_url.as_deref(),
        }
    }
}

// ─── Display List ─────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct DisplayList {
    pub commands: Vec<DisplayCommand>,
}

impl DisplayList {
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn push(&mut self, command: DisplayCommand) {
        self.commands.push(command);
    }
}

// ─── Paint List Builder ───────────────────────────────────────────

pub fn build_display_list(layout_root: &LayoutBox, dom: &Dom, source: &[u8]) -> DisplayList {
    let mut list = DisplayList::new();
    render_layout_box(layout_root, dom, source, &mut list);
    list
}

fn render_layout_box(
    layout_box: &LayoutBox,
    dom: &Dom,
    source: &[u8],
    display_list: &mut DisplayList,
) {
    // Anonymous blocks (styled_node = None) skip their own rendering
    // but MUST still recurse into children to paint wrapped inline content.
    if layout_box.styled_node.is_some() {
        render_background(layout_box, display_list, dom, source);
        render_borders(layout_box, display_list, dom, source);

        if layout_box.children.is_empty() {
            render_text(layout_box, dom, source, display_list);
            render_image(layout_box, dom, source, display_list);
        }
    }

    // Paint normal flow children first, then positioned overlay children on top
    let mut normal_children = Vec::new();
    let mut positioned_children = Vec::new();

    for child in &layout_box.children {
        let is_positioned = child
            .styled_node
            .map(|n| n.styles.position != crate::values::Position::Static)
            .unwrap_or(false);
        if is_positioned {
            positioned_children.push(child);
        } else {
            normal_children.push(child);
        }
    }

    for child in normal_children {
        render_layout_box(child, dom, source, display_list);
    }
    for child in positioned_children {
        render_layout_box(child, dom, source, display_list);
    }
}

/// Validate whether a link or resource URL is safe for browser navigation/loading.
/// Disallows control characters, null bytes, and dangerous pseudo-schemes (e.g., `javascript:`, `data:`, `vbscript:`).
pub fn is_safe_link_url(raw_url: &str) -> bool {
    let trimmed = raw_url.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Reject control characters or null bytes
    if trimmed.chars().any(|c| c.is_control() || c == '\0') {
        return false;
    }

    // If a scheme is present (`scheme:`), only allow standard browser protocols
    if let Some(colon_pos) = trimmed.find(':') {
        let scheme = &trimmed[..colon_pos].to_ascii_lowercase();
        if scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        {
            return matches!(scheme.as_str(), "http" | "https" | "file");
        }
    }

    // Safe relative URLs (paths starting with `/`, `./`, `../`, `#`, `?`, or relative filenames)
    true
}

fn find_link_url(dom: &Dom, source: &[u8], node_id: Option<NodeId>) -> Option<String> {
    let mut curr = node_id;
    while let Some(id) = curr {
        let node = dom.get(id);
        if node.tag_name(source).eq_ignore_ascii_case("a")
            && let Some(href) = node.get_attribute("href", source)
        {
            let trimmed = href.trim();
            if is_safe_link_url(trimmed) {
                return Some(trimmed.to_string());
            } else {
                return None;
            }
        }
        curr = node.parent;
    }
    None
}

fn render_background(
    layout_box: &LayoutBox,
    display_list: &mut DisplayList,
    dom: &Dom,
    source: &[u8],
) {
    let style = layout_box.styled_node.map(|n| &n.styles);
    let bg_color = style.map_or(Color::TRANSPARENT, |s| s.background_color);

    if bg_color != Color::TRANSPARENT {
        let rect = layout_box.dimensions.border_box();
        let link_url = find_link_url(dom, source, layout_box.styled_node.map(|s| s.node_id));
        display_list.push(DisplayCommand::SolidColor {
            color: bg_color,
            rect,
            link_url,
        });
    }
}

fn render_borders(
    layout_box: &LayoutBox,
    display_list: &mut DisplayList,
    dom: &Dom,
    source: &[u8],
) {
    let style = layout_box.styled_node.map(|n| &n.styles);
    let border_color = style.map_or(Color::TRANSPARENT, |s| s.border_color);
    let border_width = layout_box.dimensions.border;

    let has_border = border_width.top > 0.0
        || border_width.right > 0.0
        || border_width.bottom > 0.0
        || border_width.left > 0.0;

    if border_color != Color::TRANSPARENT && has_border {
        let rect = layout_box.dimensions.border_box();
        let link_url = find_link_url(dom, source, layout_box.styled_node.map(|s| s.node_id));
        display_list.push(DisplayCommand::Border {
            color: border_color,
            rect,
            border_width,
            link_url,
        });
    }
}

fn render_text(layout_box: &LayoutBox, dom: &Dom, source: &[u8], display_list: &mut DisplayList) {
    let Some(styled) = layout_box.styled_node else {
        return;
    };

    let node = dom.get(styled.node_id);
    let text = node.text_content(source);
    let trimmed_text = text.trim();
    if !trimmed_text.is_empty() {
        let decoded = crate::dom::decode_html_entities(trimmed_text);
        let rect = layout_box.dimensions.content;
        let link_url = find_link_url(dom, source, Some(styled.node_id));
        display_list.push(DisplayCommand::Text {
            text: decoded.into_owned(),
            x: rect.x,
            y: rect.y,
            target_width: rect.width,
            font_size: styled.styles.font_size,
            line_height: styled.styles.line_height,
            color: styled.styles.color,
            link_url,
        });
    }
}

fn render_image(layout_box: &LayoutBox, dom: &Dom, source: &[u8], display_list: &mut DisplayList) {
    let Some(styled) = layout_box.styled_node else {
        return;
    };

    let node = dom.get(styled.node_id);
    if node.tag_name(source).eq_ignore_ascii_case("img")
        && let Some(src_val) = node.get_attribute("src", source)
    {
        let trimmed_src = src_val.trim();
        if is_safe_link_url(trimmed_src) {
            let rect = layout_box.dimensions.content;
            let link_url = find_link_url(dom, source, Some(styled.node_id));
            display_list.push(DisplayCommand::Image {
                image_id: trimmed_src.to_string(),
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
                link_url,
            });
        }
    }
}

// ─── Display List Pretty Printing ─────────────────────────────────

impl fmt::Display for DisplayCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisplayCommand::SolidColor {
                color,
                rect,
                link_url,
            } => {
                write!(
                    f,
                    "SolidColor {} at (x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}) link={:?}",
                    color, rect.x, rect.y, rect.width, rect.height, link_url
                )
            }
            DisplayCommand::Border {
                color,
                rect,
                border_width,
                link_url,
            } => {
                write!(
                    f,
                    "Border (x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}) widths=[L:{:.1}, R:{:.1}, T:{:.1}, B:{:.1}] color={} link={:?}",
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    border_width.left,
                    border_width.right,
                    border_width.top,
                    border_width.bottom,
                    color,
                    link_url
                )
            }
            DisplayCommand::Text {
                text,
                x,
                y,
                font_size,
                line_height,
                color,
                link_url,
                ..
            } => {
                write!(
                    f,
                    "Text \"{}\" at (x: {:.1}, y: {:.1}) font_size={:.1}px line_height={:.1}px color={} link={:?}",
                    text, x, y, font_size, line_height, color, link_url
                )
            }
            DisplayCommand::Image {
                image_id,
                x,
                y,
                width,
                height,
                link_url,
            } => {
                write!(
                    f,
                    "Image \"{}\" at (x: {:.1}, y: {:.1}) size={:.1}x{:.1} link={:?}",
                    image_id, x, y, width, height, link_url
                )
            }
        }
    }
}

pub fn print_display_list(list: &DisplayList) {
    println!(
        "── Display List ({} commands) ─────────────────\n",
        list.commands.len()
    );
    for cmd in &list.commands {
        println!("  {}", cmd);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_safe_link_url_allowed_schemes() {
        assert!(is_safe_link_url("http://example.com"));
        assert!(is_safe_link_url("https://example.com/path?query=1#hash"));
        assert!(is_safe_link_url("file:///tmp/index.html"));
        assert!(is_safe_link_url("/relative/path"));
        assert!(is_safe_link_url("./local.html"));
        assert!(is_safe_link_url("../parent.html"));
        assert!(is_safe_link_url("#anchor"));
        assert!(is_safe_link_url("?search=test"));
        assert!(is_safe_link_url("image.png"));
    }

    #[test]
    fn test_is_safe_link_url_blocked_schemes() {
        assert!(!is_safe_link_url("javascript:alert(1)"));
        assert!(!is_safe_link_url("JAVASCRIPT:void(0)"));
        assert!(!is_safe_link_url(
            "data:text/html,<script>alert(1)</script>"
        ));
        assert!(!is_safe_link_url("vbscript:msgbox(1)"));
        assert!(!is_safe_link_url("blob:http://example.com/uuid"));
        assert!(!is_safe_link_url("custom-scheme://test"));
    }

    #[test]
    fn test_is_safe_link_url_control_chars() {
        assert!(!is_safe_link_url(""));
        assert!(!is_safe_link_url("   "));
        assert!(!is_safe_link_url("https://example.com\0evil"));
        assert!(!is_safe_link_url("https://example.com\r\nHeader: evil"));
        assert!(!is_safe_link_url("http://example.com/\x08test"));
    }
}
