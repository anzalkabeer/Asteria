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
    },
    /// drawing the ofour edges
    Border {
        color: Color,
        rect: Rect,
        border_width: EdgeSizes,
    },
    Text {
        text: String,
        x: f32,
        y: f32,
        font_size: f32,
        color: Color,
    },
    /// Image rendering command
    Image {
        image_id: String,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
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

/// Generate a complete DisplayList for a layout tree in CSS paint order.
pub fn build_display_list<'a>(
    layout_root: &'a LayoutBox<'a>,
    dom: &Dom,
    source: &[u8],
) -> DisplayList {
    let mut list = DisplayList::new();
    render_layout_box(layout_root, dom, source, &mut list);
    list
}

/// Recursively render a single LayoutBox and its descendants in CSS Paint Order:
///   1. Background
///   2. Borders
///   3. Text
///   4. Children
fn render_layout_box<'a>(
    layout_box: &'a LayoutBox<'a>,
    dom: &Dom,
    source: &[u8],
    display_list: &mut DisplayList,
) {
    render_background(layout_box, display_list);
    render_borders(layout_box, display_list);
    render_text(layout_box, dom, source, display_list);
    render_children(layout_box, dom, source, display_list);
}

fn render_background(layout_box: &LayoutBox, display_list: &mut DisplayList) {
    let style = layout_box.styled_node.map(|n| &n.styles);
    let bg_color = style.map_or(Color::TRANSPARENT, |s| s.background_color);

    if bg_color != Color::TRANSPARENT {
        let rect = layout_box.dimensions.border_box();
        display_list.push(DisplayCommand::SolidColor {
            color: bg_color,
            rect,
        });
    }
}

fn render_borders(layout_box: &LayoutBox, display_list: &mut DisplayList) {
    let style = layout_box.styled_node.map(|n| &n.styles);
    let border_color = style.map_or(Color::TRANSPARENT, |s| s.border_color);
    let border_width = layout_box.dimensions.border;

    let has_border = border_width.top > 0.0
        || border_width.right > 0.0
        || border_width.bottom > 0.0
        || border_width.left > 0.0;

    if border_color != Color::TRANSPARENT && has_border {
        let rect = layout_box.dimensions.border_box();
        display_list.push(DisplayCommand::Border {
            color: border_color,
            rect,
            border_width,
        });
    }
}

fn render_text(
    layout_box: &LayoutBox,
    dom: &Dom,
    source: &[u8],
    display_list: &mut DisplayList,
) {
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

            display_list.push(DisplayCommand::Text {
                text,
                x: rect.x,
                y: rect.y,
                font_size,
                color: text_color,
            });
        }
    }
}

fn render_children<'a>(
    layout_box: &'a LayoutBox<'a>,
    dom: &Dom,
    source: &[u8],
    display_list: &mut DisplayList,
) {
    for child in &layout_box.children {
        render_background(child, display_list);
        render_borders(child, display_list);
        render_text(child, dom, source, display_list);
        render_children(child, dom, source, display_list); // recursively render the children of the current box
    }
}

// ─── Display Formatting & Inspection ─────────────────────────────

impl fmt::Display for DisplayCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisplayCommand::SolidColor { color, rect } => {
                write!(
                    f,
                    "SolidColor rect=(x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}) color={}",
                    rect.x, rect.y, rect.width, rect.height, color
                )
            }
            DisplayCommand::Border {
                color,
                rect,
                border_width,
            } => {
                write!(
                    f,
                    "Border rect=(x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}) widths=(L{:.1} R{:.1} T{:.1} B{:.1}) color={}",
                    rect.x, rect.y, rect.width, rect.height, border_width.left, border_width.right, border_width.top, border_width.bottom, color
                )
            }
            DisplayCommand::Text {
                text,
                x,
                y,
                font_size,
                color,
            } => {
                write!(
                    f,
                    "Text \"{}\" at (x: {:.1}, y: {:.1}) font_size={:.1}px color={}",
                    text, x, y, font_size, color
                )
            }
            DisplayCommand::Image {
                image_id,
                x,
                y,
                width,
                height,
            } => {
                write!(
                    f,
                    "Image \"{}\" at (x: {:.1}, y: {:.1}) size={:.1}x{:.1}",
                    image_id, x, y, width, height
                )
            }
        }
    }
}

pub fn print_display_list(list: &DisplayList) {
    //printing header
    println!("── Display List ({} commands) ─────────────────\n", list.commands.len());
    for (i, command) in list.commands.iter().enumerate() {
        println!("  [{:>2}] {}", i, command);
    }
}