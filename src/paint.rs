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

use crate::dom::{Dom, NodeId, NodeKind};
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
    if layout_box.styled_node.is_none() {
        return;
    }

    render_background(layout_box, display_list, dom, source);
    render_borders(layout_box, display_list, dom, source);

    if layout_box.children.is_empty() {
        render_text(layout_box, dom, source, display_list);
        render_image(layout_box, dom, source, display_list);
    }

    for child in &layout_box.children {
        render_layout_box(child, dom, source, display_list);
    }
}

fn find_link_url(dom: &Dom, source: &[u8], node_id: Option<NodeId>) -> Option<String> {
    let mut curr = node_id;
    while let Some(id) = curr {
        let node = dom.get(id);
        if let NodeKind::Element { tag_start, tag_end } = node.kind {
            let tag_name =
                std::str::from_utf8(&source[tag_start as usize..tag_end as usize]).unwrap_or("");
            if tag_name.eq_ignore_ascii_case("a") {
                for &(ns, ne, vs, ve) in &node.attributes {
                    let attr_name =
                        std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap_or("");
                    if attr_name.eq_ignore_ascii_case("href") {
                        return Some(
                            std::str::from_utf8(&source[vs as usize..ve as usize])
                                .unwrap_or("")
                                .to_string(),
                        );
                    }
                }
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
        let link_url = find_link_url(
            dom,
            source,
            layout_box.styled_node.map(|s| s.node_id),
        );
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
        let link_url = find_link_url(
            dom,
            source,
            layout_box.styled_node.map(|s| s.node_id),
        );
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
    if let NodeKind::Text { start, end } = node.kind {
        let text = std::str::from_utf8(&source[start as usize..end as usize]).unwrap_or("");
        let trimmed_text = text.trim();
        if !trimmed_text.is_empty() {
            let rect = layout_box.dimensions.content;
            let link_url = find_link_url(dom, source, Some(styled.node_id));
            display_list.push(DisplayCommand::Text {
                text: text.to_string(),
                x: rect.x,
                y: rect.y,
                target_width: rect.width,
                font_size: styled.styles.font_size,
                color: styled.styles.color,
                link_url,
            });
        }
    }
}

fn render_image(layout_box: &LayoutBox, dom: &Dom, source: &[u8], display_list: &mut DisplayList) {
    let Some(styled) = layout_box.styled_node else {
        return;
    };

    let node = dom.get(styled.node_id);
    if let NodeKind::Element { tag_start, tag_end } = node.kind {
        let tag_name =
            std::str::from_utf8(&source[tag_start as usize..tag_end as usize]).unwrap_or("");
        if tag_name.eq_ignore_ascii_case("img") {
            let mut src = None;
            for &(ns, ne, vs, ve) in &node.attributes {
                let attr_name =
                    std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap_or("");
                if attr_name.eq_ignore_ascii_case("src") {
                    src =
                        Some(std::str::from_utf8(&source[vs as usize..ve as usize]).unwrap_or(""));
                    break;
                }
            }
            if let Some(src_val) = src {
                let rect = layout_box.dimensions.content;
                let link_url = find_link_url(dom, source, Some(styled.node_id));
                display_list.push(DisplayCommand::Image {
                    image_id: src_val.to_string(),
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                    link_url,
                });
            }
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
                color,
                link_url,
                ..
            } => {
                write!(
                    f,
                    "Text \"{}\" at (x: {:.1}, y: {:.1}) font_size={:.1}px color={} link={:?}",
                    text, x, y, font_size, color, link_url
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
