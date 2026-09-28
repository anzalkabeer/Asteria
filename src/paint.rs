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
    RoundedRect {
        color: Color,
        rect: Rect,
        radius: crate::values::BorderRadius,
        link_url: Option<String>,
    },
    Border {
        color: Color,
        rect: Rect,
        border_width: EdgeSizes,
        link_url: Option<String>,
    },
    BoxShadow {
        rect: Rect,
        shadow: crate::values::BoxShadow,
        link_url: Option<String>,
    },
    PushClip {
        rect: Rect,
    },
    PopClip,
    PushTransform {
        transform: [f32; 6],
    },
    PopTransform,
    Text {
        text: String,
        x: f32,
        y: f32,
        target_width: f32,
        font_size: f32,
        line_height: f32,
        font_family: Vec<String>,
        font_weight: f32,
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
            DisplayCommand::RoundedRect { link_url, .. } => link_url.as_deref(),
            DisplayCommand::Border { link_url, .. } => link_url.as_deref(),
            DisplayCommand::BoxShadow { link_url, .. } => link_url.as_deref(),
            DisplayCommand::PushClip { .. }
            | DisplayCommand::PopClip
            | DisplayCommand::PushTransform { .. }
            | DisplayCommand::PopTransform => None,
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
    render_stacking_context(layout_root, dom, source, &mut list, 1.0);
    list
}

fn render_stacking_context(
    layout_box: &LayoutBox,
    dom: &Dom,
    source: &[u8],
    display_list: &mut DisplayList,
    parent_opacity: f32,
) {
    let node_opacity = layout_box
        .styled_node
        .map(|s| s.styles.opacity)
        .unwrap_or(1.0);
    let current_opacity = (parent_opacity * node_opacity).clamp(0.0, 1.0);

    let is_clipped = layout_box
        .styled_node
        .map(|s| s.styles.overflow == crate::values::Overflow::Hidden)
        .unwrap_or(false);

    if is_clipped {
        display_list.push(DisplayCommand::PushClip {
            rect: layout_box.dimensions.padding_box(),
        });
    }

    let has_transform = layout_box
        .styled_node
        .map(|s| !s.styles.transform.is_empty())
        .unwrap_or(false);

    if has_transform {
        let rect = layout_box.dimensions.border_box();
        let ox = rect.x + rect.width * 0.5;
        let oy = rect.y + rect.height * 0.5;
        let m = crate::values::compose_transform(&layout_box.styled_node.unwrap().styles.transform);
        let tx_eff = m[4] + ox - m[0] * ox - m[2] * oy;
        let ty_eff = m[5] + oy - m[1] * ox - m[3] * oy;
        let m_origin = [m[0], m[1], m[2], m[3], tx_eff, ty_eff];
        display_list.push(DisplayCommand::PushTransform {
            transform: m_origin,
        });
    }

    // 1. Render stacking context root element's background and borders
    if layout_box.styled_node.is_some() {
        render_background(layout_box, display_list, dom, source, current_opacity);
        render_borders(layout_box, display_list, dom, source, current_opacity);

        if layout_box.children.is_empty() {
            render_text(layout_box, dom, source, display_list, current_opacity);
            render_image(layout_box, dom, source, display_list);
        }
    }

    // 2. Partition descendants of this stacking context:
    let mut neg_positioned = Vec::new();
    let mut in_flow_commands = DisplayList::new();
    let mut pos_positioned = Vec::new();
    let mut active_clips = Vec::new();

    collect_stacking_context_descendants(
        layout_box,
        &mut neg_positioned,
        &mut in_flow_commands,
        &mut pos_positioned,
        dom,
        source,
        current_opacity,
        &mut active_clips,
    );

    // 3. Negative z-index positioned descendants (< 0), sorted by z-index
    neg_positioned.sort_by_key(|(z, _, _, _)| *z);
    for (_, child, child_op, clips) in neg_positioned {
        for clip in &clips {
            display_list.push(DisplayCommand::PushClip { rect: *clip });
        }
        render_stacking_context(child, dom, source, display_list, child_op);
        for _ in 0..clips.len() {
            display_list.push(DisplayCommand::PopClip);
        }
    }

    // 4. Normal in-flow descendant content
    display_list.commands.extend(in_flow_commands.commands);

    // 5. Auto / non-negative z-index positioned descendants (>= 0), sorted by z-index
    pos_positioned.sort_by_key(|(z, _, _, _)| *z);
    for (_, child, child_op, clips) in pos_positioned {
        for clip in &clips {
            display_list.push(DisplayCommand::PushClip { rect: *clip });
        }
        render_stacking_context(child, dom, source, display_list, child_op);
        for _ in 0..clips.len() {
            display_list.push(DisplayCommand::PopClip);
        }
    }

    if has_transform {
        display_list.push(DisplayCommand::PopTransform);
    }

    if is_clipped {
        display_list.push(DisplayCommand::PopClip);
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_stacking_context_descendants<'a>(
    parent: &'a LayoutBox<'a>,
    neg_positioned: &mut Vec<(i32, &'a LayoutBox<'a>, f32, Vec<Rect>)>,
    in_flow_commands: &mut DisplayList,
    pos_positioned: &mut Vec<(i32, &'a LayoutBox<'a>, f32, Vec<Rect>)>,
    dom: &Dom,
    source: &[u8],
    parent_opacity: f32,
    active_clips: &mut Vec<Rect>,
) {
    for child in &parent.children {
        let child_opacity = (parent_opacity
            * child.styled_node.map(|s| s.styles.opacity).unwrap_or(1.0))
        .clamp(0.0, 1.0);

        let is_positioned = child
            .styled_node
            .map(|n| {
                n.styles.position != crate::values::Position::Static
                    || !n.styles.transform.is_empty()
            })
            .unwrap_or(false);

        if is_positioned {
            let z = child
                .styled_node
                .and_then(|n| n.styles.z_index)
                .unwrap_or(0);
            if z < 0 {
                neg_positioned.push((z, child, parent_opacity, active_clips.clone()));
            } else {
                pos_positioned.push((z, child, parent_opacity, active_clips.clone()));
            }
        } else {
            let is_child_clipped = child
                .styled_node
                .map(|s| s.styles.overflow == crate::values::Overflow::Hidden)
                .unwrap_or(false);

            if is_child_clipped {
                let clip_rect = child.dimensions.padding_box();
                in_flow_commands.push(DisplayCommand::PushClip { rect: clip_rect });
                active_clips.push(clip_rect);
            }

            // Normal flow: render in-flow box decorations and text
            if child.styled_node.is_some() {
                render_background(child, in_flow_commands, dom, source, child_opacity);
                render_borders(child, in_flow_commands, dom, source, child_opacity);

                if child.children.is_empty() {
                    render_text(child, dom, source, in_flow_commands, child_opacity);
                    render_image(child, dom, source, in_flow_commands);
                }
            }

            // Recurse into in-flow descendants to collect further nested content
            collect_stacking_context_descendants(
                child,
                neg_positioned,
                in_flow_commands,
                pos_positioned,
                dom,
                source,
                child_opacity,
                active_clips,
            );

            if is_child_clipped {
                active_clips.pop();
                in_flow_commands.push(DisplayCommand::PopClip);
            }
        }
    }
}

/// Validate whether a link or resource URL is safe for browser navigation/loading.
/// Disallows control characters, null bytes, and dangerous pseudo-schemes (e.g., `javascript:`, `data:`, `vbscript:`).
/// Accepts an optional document scheme to restrict `file:` URLs to file-based documents only.
pub fn is_safe_link_url(raw_url: &str, doc_scheme: Option<&str>) -> bool {
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
            if scheme == "http" || scheme == "https" {
                return true;
            }
            if scheme == "file" {
                return doc_scheme == Some("file");
            }
            return false;
        }
    }

    // Safe relative URLs (paths starting with `/`, `./`, `../`, `#`, `?`, or relative filenames)
    true
}

fn detect_document_scheme<'a>(dom: &Dom, source: &'a [u8]) -> Option<&'a str> {
    for i in 0..dom.nodes.len() {
        let node = dom.get(crate::dom::NodeId(i as u32));
        if node.tag_name(source).eq_ignore_ascii_case("base")
            && let Some(scheme) = node
                .get_attribute("href", source)
                .and_then(|href| href.split_once(':'))
                .map(|(scheme, _)| scheme.trim())
        {
            return Some(scheme);
        }
    }
    None
}

fn find_link_url(dom: &Dom, source: &[u8], node_id: Option<NodeId>) -> Option<String> {
    let doc_scheme = detect_document_scheme(dom, source);
    let mut curr = node_id;
    while let Some(id) = curr {
        let node = dom.get(id);
        if node.tag_name(source).eq_ignore_ascii_case("a")
            && let Some(href) = node.get_attribute("href", source)
        {
            let trimmed = href.trim();
            if is_safe_link_url(trimmed, doc_scheme) {
                return Some(trimmed.to_string());
            }
            return None;
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
    opacity: f32,
) {
    let style = layout_box.styled_node.map(|n| &n.styles);
    let link_url = find_link_url(dom, source, layout_box.styled_node.map(|s| s.node_id));
    let rect = layout_box.dimensions.border_box();

    // 1. Box shadows (rendered before background per CSS spec)
    if let Some(s) = style {
        for shadow in &s.box_shadow {
            let shadow_color = shadow.color.with_alpha_multiplier(opacity);
            let shadow_rect = Rect {
                x: rect.x + shadow.offset_x - shadow.spread_radius,
                y: rect.y + shadow.offset_y - shadow.spread_radius,
                width: (rect.width + 2.0 * shadow.spread_radius).max(0.0),
                height: (rect.height + 2.0 * shadow.spread_radius).max(0.0),
            };
            display_list.push(DisplayCommand::BoxShadow {
                rect: shadow_rect,
                shadow: crate::values::BoxShadow {
                    color: shadow_color,
                    ..*shadow
                },
                link_url: link_url.clone(),
            });
        }
    }

    // 2. Background color
    let bg_color = style.map_or(Color::TRANSPARENT, |s| s.background_color);
    if bg_color != Color::TRANSPARENT {
        let final_color = bg_color.with_alpha_multiplier(opacity);
        let border_radius = style
            .map(|s| s.border_radius)
            .unwrap_or(crate::values::BorderRadius::ZERO);
        if border_radius.is_zero() {
            display_list.push(DisplayCommand::SolidColor {
                color: final_color,
                rect,
                link_url,
            });
        } else {
            display_list.push(DisplayCommand::RoundedRect {
                color: final_color,
                rect,
                radius: border_radius,
                link_url,
            });
        }
    }
}

fn render_borders(
    layout_box: &LayoutBox,
    display_list: &mut DisplayList,
    dom: &Dom,
    source: &[u8],
    opacity: f32,
) {
    let style = layout_box.styled_node.map(|n| &n.styles);
    let border_color = style.map_or(Color::TRANSPARENT, |s| s.border_color);
    let border_width = layout_box.dimensions.border;

    let has_border = border_width.top > 0.0
        || border_width.right > 0.0
        || border_width.bottom > 0.0
        || border_width.left > 0.0;

    if border_color != Color::TRANSPARENT && has_border {
        let final_color = border_color.with_alpha_multiplier(opacity);
        let rect = layout_box.dimensions.border_box();
        let link_url = find_link_url(dom, source, layout_box.styled_node.map(|s| s.node_id));
        display_list.push(DisplayCommand::Border {
            color: final_color,
            rect,
            border_width,
            link_url,
        });
    }
}

fn render_text(
    layout_box: &LayoutBox,
    dom: &Dom,
    source: &[u8],
    display_list: &mut DisplayList,
    opacity: f32,
) {
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
        let final_color = styled.styles.color.with_alpha_multiplier(opacity);
        display_list.push(DisplayCommand::Text {
            text: decoded.into_owned(),
            x: rect.x,
            y: rect.y,
            target_width: rect.width,
            font_size: styled.styles.font_size,
            line_height: styled.styles.line_height,
            font_family: styled.styles.font_family.clone(),
            font_weight: styled.styles.font_weight,
            color: final_color,
            link_url,
        });
    }
}

fn render_image(layout_box: &LayoutBox, dom: &Dom, source: &[u8], display_list: &mut DisplayList) {
    let Some(styled) = layout_box.styled_node else {
        return;
    };

    let doc_scheme = detect_document_scheme(dom, source);
    let node = dom.get(styled.node_id);
    if node.tag_name(source).eq_ignore_ascii_case("img")
        && let Some(src_val) = node.get_attribute("src", source)
    {
        let trimmed_src = src_val.trim();
        if is_safe_link_url(trimmed_src, doc_scheme) {
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
            DisplayCommand::RoundedRect {
                color,
                rect,
                radius,
                link_url,
            } => {
                write!(
                    f,
                    "RoundedRect {} at (x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}) radius={} link={:?}",
                    color, rect.x, rect.y, rect.width, rect.height, radius, link_url
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
            DisplayCommand::BoxShadow {
                rect,
                shadow,
                link_url,
            } => {
                write!(
                    f,
                    "BoxShadow at (x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}) color={} blur={:.1} spread={:.1} inset={} link={:?}",
                    rect.x,
                    rect.y,
                    rect.width,
                    rect.height,
                    shadow.color,
                    shadow.blur_radius,
                    shadow.spread_radius,
                    shadow.inset,
                    link_url
                )
            }
            DisplayCommand::PushClip { rect } => {
                write!(
                    f,
                    "PushClip (x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1})",
                    rect.x, rect.y, rect.width, rect.height
                )
            }
            DisplayCommand::PopClip => write!(f, "PopClip"),
            DisplayCommand::PushTransform { transform } => {
                write!(f, "PushTransform {:?}", transform)
            }
            DisplayCommand::PopTransform => write!(f, "PopTransform"),
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
        assert!(is_safe_link_url("http://example.com", None));
        assert!(is_safe_link_url(
            "https://example.com/path?query=1#hash",
            None
        ));
        assert!(is_safe_link_url("file:///tmp/index.html", Some("file")));
        assert!(!is_safe_link_url("file:///tmp/index.html", Some("http")));
        assert!(!is_safe_link_url("file:///tmp/index.html", None));
        assert!(is_safe_link_url("/relative/path", None));
        assert!(is_safe_link_url("./local.html", None));
        assert!(is_safe_link_url("../parent.html", None));
        assert!(is_safe_link_url("#anchor", None));
        assert!(is_safe_link_url("?search=test", None));
        assert!(is_safe_link_url("image.png", None));
    }

    #[test]
    fn test_is_safe_link_url_blocked_schemes() {
        assert!(!is_safe_link_url("javascript:alert(1)", None));
        assert!(!is_safe_link_url("JAVASCRIPT:void(0)", None));
        assert!(!is_safe_link_url(
            "data:text/html,<script>alert(1)</script>",
            None
        ));
        assert!(!is_safe_link_url("vbscript:msgbox(1)", None));
        assert!(!is_safe_link_url("blob:http://example.com/uuid", None));
        assert!(!is_safe_link_url("custom-scheme://test", None));
    }

    #[test]
    fn test_is_safe_link_url_control_chars() {
        assert!(!is_safe_link_url("", None));
        assert!(!is_safe_link_url("   ", None));
        assert!(!is_safe_link_url("https://example.com\0evil", None));
        assert!(!is_safe_link_url(
            "https://example.com\r\nHeader: evil",
            None
        ));
        assert!(!is_safe_link_url("http://example.com/\x08test", None));
    }
}
