// hehe paint engin e here i am coding manually i will debug with ai though.
// ─── Asteria Paint Engine & Display List Generator ──────────────────
//
// Consumes a positioned LayoutBox tree and generates a backend-agnostic
// DisplayList containing visual 2D drawing primitives (SolidColor, Border, Text).

use std::fmt;

use crate::dom::{Dom, NodeKind};
use crate::layout::{EdgeSizes, LayoutBox, Rect};
use crate::values::Color;

//a single visual drawing command in the display list .

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayCommand {
    /// here i have to write the solidcolor enum variant for the display command
    SolidColor {
        color: Color,
        rect: Rect,
        link_url: Option<String>,
    },
    /// drawing the ofour edges
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
        font_size: f32,
        color: Color,
        link_url: Option<String>,
    },
    /// Image rendering command
    Image {
        image_id: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        link_url: Option<String>,
    },
}

/// Ordered collection of drawing commands representing the visual page.
#[derive(Debug, Clone, Default)]
pub struct DisplayList {
    pub commands: Vec<DisplayCommand>,
}

impl DisplayList {
    pub fn new() -> Self {
        DisplayList {
            commands: Vec::new(),
        }
    }

    pub fn push(&mut self, command: DisplayCommand) {
        self.commands.push(command);
    }
}

// ─── Paint Engine Logic & Traversal ───────────────────────────────

/// Find if a node is inside an `<a>` tag and extract its `href` attribute.
fn find_link_url(
    dom: &Dom,
    source: &[u8],
    mut current_node: Option<crate::dom::NodeId>,
) -> Option<String> {
    while let Some(id) = current_node {
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
        current_node = node.parent;
    }
    None
}

/// Generate a complete DisplayList for a layout tree in CSS paint order.
pub fn build_display_list<'a>(
    layout_root: &'a LayoutBox<'a>,
    dom: &Dom,
    source: &[u8],
) -> DisplayList {
    let mut display_list = DisplayList::new();
    render_children(layout_root, dom, source, &mut display_list);
    display_list
}

fn render_children(
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
        render_children(child, dom, source, display_list);
    }
}

fn render_background(
    layout_box: &LayoutBox,
    display_list: &mut DisplayList,
    dom: &Dom,
    source: &[u8],
) {
    let Some(styled) = layout_box.styled_node else {
        return;
    };
    let bg_color = styled.styles.background_color;

    if bg_color != Color::TRANSPARENT {
        let rect = layout_box.dimensions.padding_box();
        let link_url = find_link_url(dom, source, Some(styled.node_id));
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
    let Some(styled) = layout_box.styled_node else {
        return;
    };
    let border_color = styled.styles.border_color;
    let border_width = layout_box.dimensions.border;

    let has_border = border_width.top > 0.0
        || border_width.right > 0.0
        || border_width.bottom > 0.0
        || border_width.left > 0.0;

    if border_color != Color::TRANSPARENT && has_border {
        let rect = layout_box.dimensions.border_box();
        let link_url = find_link_url(dom, source, Some(styled.node_id));
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
        let text = std::str::from_utf8(&source[start as usize..end as usize])
            .unwrap_or("")
            .trim()
            .to_string();

        if !text.is_empty() {
            let text_color = styled.styles.color;
            let font_size = styled.styles.font_size;
            let rect = layout_box.dimensions.content;
            let link_url = find_link_url(dom, source, Some(styled.node_id));

            display_list.push(DisplayCommand::Text {
                text,
                x: rect.x,
                y: rect.y,
                font_size,
                color: text_color,
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

// ─── Display Formatting & Inspection ─────────────────────────────

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
                    "SolidColor rect=(x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}) color={} link={:?}",
                    rect.x, rect.y, rect.width, rect.height, color, link_url
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
                    "Border rect=(x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}) widths=(L{:.1} R{:.1} T{:.1} B{:.1}) color={} link={:?}",
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
