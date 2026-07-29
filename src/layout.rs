// ─── Asteria Layout Engine ─────────────────────────────────────────
//
// The layout engine takes a Styled DOM (tree of StyledNode carrying
// resolved ComputedStyle values) and computes 2D coordinates (x, y)
// and box model dimensions (width, height, margins, padding, borders)
// for every visible element.
//
// Supports both Block Formatting Context (vertical block stacking) and
// Inline Formatting Context (horizontal left-to-right line boxes with line wrapping).

use crate::dom::{Dom, NodeKind};
use crate::style::StyledNode;
use crate::values::Display;

// ─── Geometry & Box Model ─────────────────────────────────────────

/// A 2D floating-point rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Border, padding, or margin sizes for the four edges of a box.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct EdgeSizes {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

/// Full CSS Box Model geometry for a layout box.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Dimensions {
    /// Position and size of content area relative to document origin
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

impl Dimensions {
    /// Rectangle enclosing content + padding
    pub fn padding_box(&self) -> Rect {
        Rect {
            x: self.content.x - self.padding.left,
            y: self.content.y - self.padding.top,
            width: self.content.width + self.padding.left + self.padding.right,
            height: self.content.height + self.padding.top + self.padding.bottom,
        }
    }

    /// Rectangle enclosing content + padding + border
    pub fn border_box(&self) -> Rect {
        let p = self.padding_box();
        Rect {
            x: p.x - self.border.left,
            y: p.y - self.border.top,
            width: p.width + self.border.left + self.border.right,
            height: p.height + self.border.top + self.border.bottom,
        }
    }

    /// Rectangle enclosing content + padding + border + margin
    pub fn margin_box(&self) -> Rect {
        let b = self.border_box();
        Rect {
            x: b.x - self.margin.left,
            y: b.y - self.margin.top,
            width: b.width + self.margin.left + self.margin.right,
            height: b.height + self.margin.top + self.margin.bottom,
        }
    }
}

// ─── Layout Box Types ─────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxType {
    BlockNode,
    InlineNode,
    AnonymousBlock,
}

/// A node in the layout tree. Holds computed dimensions, box classification,
/// reference to the underlying StyledNode (if any), and child layout boxes.
pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub box_type: BoxType,
    pub styled_node: Option<&'a StyledNode>,
    pub children: Vec<LayoutBox<'a>>,
}

impl<'a> LayoutBox<'a> {
    pub fn new(box_type: BoxType, styled_node: Option<&'a StyledNode>) -> Self {
        LayoutBox {
            dimensions: Dimensions::default(),
            box_type,
            styled_node,
            children: Vec::new(),
        }
    }

    /// Recursively compute geometry and position for this box and its subtree.
    pub fn layout(&mut self, containing_block: Dimensions, dom: &Dom, source: &[u8]) {
        match self.box_type {
            BoxType::BlockNode | BoxType::AnonymousBlock => {
                self.layout_block(containing_block, dom, source);
            }
            BoxType::InlineNode => {
                self.layout_inline(containing_block, dom, source);
            }
        }
    }

    // ─── Block Layout Algorithm ───────────────────────────────────

    fn layout_block(&mut self, containing_block: Dimensions, dom: &Dom, source: &[u8]) {
        // Step 1: Calculate horizontal width and margins
        self.calculate_block_width(containing_block);

        // Step 2: Calculate position (x, y) relative to document origin
        self.calculate_block_position(containing_block);

        // Step 3: Lay out children (block stacking or inline line-box formatting context)
        self.layout_block_children(dom, source);

        // Step 4: Calculate explicit height if specified
        self.calculate_block_height();
    }

    /// Calculate width, padding, border, and margins for a block box
    /// using W3C width constraint equations.
    #[allow(clippy::unnecessary_map_or)]
    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.styled_node.map(|n| &n.styles);

        // Read values or defaults
        let auto_width = style.map(|s| s.width.is_none()).unwrap_or(true);
        let mut width = style.and_then(|s| s.width).unwrap_or(0.0);

        let mut margin_left = style.map(|s| s.margin.left).unwrap_or(0.0);
        let mut margin_right = style.map(|s| s.margin.right).unwrap_or(0.0);
        let margin_top = style.map(|s| s.margin.top).unwrap_or(0.0);
        let margin_bottom = style.map(|s| s.margin.bottom).unwrap_or(0.0);

        let padding_left = style.map(|s| s.padding.left).unwrap_or(0.0);
        let padding_right = style.map(|s| s.padding.right).unwrap_or(0.0);
        let padding_top = style.map(|s| s.padding.top).unwrap_or(0.0);
        let padding_bottom = style.map(|s| s.padding.bottom).unwrap_or(0.0);

        let border_left = style.map(|s| s.border_width.left).unwrap_or(0.0);
        let border_right = style.map(|s| s.border_width.right).unwrap_or(0.0);
        let border_top = style.map(|s| s.border_width.top).unwrap_or(0.0);
        let border_bottom = style.map(|s| s.border_width.bottom).unwrap_or(0.0);

        let total_non_width =
            margin_left + margin_right + padding_left + padding_right + border_left + border_right;

        // Constraint solving: if width is auto, expand content width to fill containing block
        if auto_width {
            let available_width = (containing_block.content.width - total_non_width).max(0.0);
            width = available_width;
        } else {
            // Specified width: underflow goes to margin-right per CSS spec (unless margin-left/right are auto)
            let underflow = containing_block.content.width - (width + total_non_width);

            // Auto margin centering occurs when both margins are 0.0/auto
            if margin_left == 0.0 && margin_right == 0.0 && underflow > 0.0 {
                margin_left = underflow / 2.0;
                margin_right = underflow / 2.0;
            } else if underflow > 0.0 {
                margin_right += underflow;
            }
        }

        // Store computed values into box dimensions
        self.dimensions.content.width = width;

        self.dimensions.margin = EdgeSizes {
            top: margin_top,
            right: margin_right,
            bottom: margin_bottom,
            left: margin_left,
        };

        self.dimensions.padding = EdgeSizes {
            top: padding_top,
            right: padding_right,
            bottom: padding_bottom,
            left: padding_left,
        };

        self.dimensions.border = EdgeSizes {
            top: border_top,
            right: border_right,
            bottom: border_bottom,
            left: border_left,
        };
    }

    /// Calculate 2D position (x, y) in document space
    fn calculate_block_position(&mut self, containing_block: Dimensions) {
        self.dimensions.content.x = containing_block.content.x
            + self.dimensions.margin.left
            + self.dimensions.border.left
            + self.dimensions.padding.left;

        self.dimensions.content.y = containing_block.content.y
            + containing_block.content.height
            + self.dimensions.margin.top
            + self.dimensions.border.top
            + self.dimensions.padding.top;
    }

    /// Layout children inside this box.
    /// If children are InlineNodes, format them in a horizontal line box context.
    /// If children are BlockNodes, stack them vertically.
    fn layout_block_children(&mut self, dom: &Dom, source: &[u8]) {
        let is_inline_context = self
            .children
            .iter()
            .all(|c| c.box_type == BoxType::InlineNode);

        if is_inline_context && !self.children.is_empty() {
            // ─── Inline Formatting Context (Horizontal Line Flow) ───────────
            let mut cursor_x = self.dimensions.content.x;
            let mut cursor_y = self.dimensions.content.y;
            let mut current_line_height: f32 = 0.0;
            let container_max_w = self.dimensions.content.width;

            for child in &mut self.children {
                let style = child.styled_node.map(|n| &n.styles);

                let margin_left = style.map_or(0.0, |s| s.margin.left);
                let margin_right = style.map_or(0.0, |s| s.margin.right);
                let margin_top = style.map_or(0.0, |s| s.margin.top);
                let margin_bottom = style.map_or(0.0, |s| s.margin.bottom);

                let padding_left = style.map_or(0.0, |s| s.padding.left);
                let padding_right = style.map_or(0.0, |s| s.padding.right);
                let padding_top = style.map_or(0.0, |s| s.padding.top);
                let padding_bottom = style.map_or(0.0, |s| s.padding.bottom);

                let border_left = style.map_or(0.0, |s| s.border_width.left);
                let border_right = style.map_or(0.0, |s| s.border_width.right);
                let border_top = style.map_or(0.0, |s| s.border_width.top);
                let border_bottom = style.map_or(0.0, |s| s.border_width.bottom);

                let content_w = compute_intrinsic_inline_width(child.styled_node, dom, source);
                let content_h = style.map_or(19.2, |s| s.line_height);

                let outer_w = margin_left
                    + border_left
                    + padding_left
                    + content_w
                    + padding_right
                    + border_right
                    + margin_right;
                let outer_h = margin_top
                    + border_top
                    + padding_top
                    + content_h
                    + padding_bottom
                    + border_bottom
                    + margin_bottom;

                // Horizontal Line Wrap Check
                if container_max_w > 0.0
                    && (cursor_x + outer_w > self.dimensions.content.x + container_max_w)
                    && (cursor_x > self.dimensions.content.x)
                {
                    cursor_y += current_line_height;
                    cursor_x = self.dimensions.content.x;
                    current_line_height = 0.0;
                }

                // Position child horizontally on the current line box
                child.dimensions.content.x = cursor_x + margin_left + border_left + padding_left;
                child.dimensions.content.y = cursor_y + margin_top + border_top + padding_top;
                child.dimensions.content.width = content_w;
                child.dimensions.content.height = content_h;

                child.dimensions.margin = EdgeSizes {
                    top: margin_top,
                    right: margin_right,
                    bottom: margin_bottom,
                    left: margin_left,
                };
                child.dimensions.padding = EdgeSizes {
                    top: padding_top,
                    right: padding_right,
                    bottom: padding_bottom,
                    left: padding_left,
                };
                child.dimensions.border = EdgeSizes {
                    top: border_top,
                    right: border_right,
                    bottom: border_bottom,
                    left: border_left,
                };

                // Recursively layout child's descendants
                child.layout(child.dimensions, dom, source);

                // Advance horizontal cursor
                cursor_x += outer_w;
                current_line_height = current_line_height.max(outer_h);
            }

            self.dimensions.content.height =
                (cursor_y + current_line_height) - self.dimensions.content.y;
        } else {
            // ─── Block Formatting Context (Vertical Stack Flow) ──────────────
            let mut content_height = 0.0;

            for child in &mut self.children {
                let mut container = self.dimensions;
                container.content.height = content_height;

                child.layout(container, dom, source);

                content_height += child.dimensions.margin_box().height;
            }

            self.dimensions.content.height = content_height;
        }
    }

    /// Override content height if explicitly specified on the element's style
    fn calculate_block_height(&mut self) {
        if let Some(h) = self.styled_node.and_then(|n| n.styles.height) {
            self.dimensions.content.height = h;
        }
    }

    // ─── Inline Layout Handling ────────────────────────────────────

    fn layout_inline(&mut self, _containing_block: Dimensions, dom: &Dom, source: &[u8]) {
        // Inner inline children layout logic
        self.layout_block_children(dom, source);
    }
}

/// Compute intrinsic width for an inline styled node (text content length or child sum)
fn compute_intrinsic_inline_width(
    styled_node: Option<&StyledNode>,
    dom: &Dom,
    source: &[u8],
) -> f32 {
    let Some(styled) = styled_node else {
        return 0.0;
    };

    if let Some(w) = styled.styles.width {
        return w;
    }

    let node = dom.get(styled.node_id);
    match &node.kind {
        NodeKind::Text { start, end } => {
            let font_size = styled.styles.font_size;
            let text = std::str::from_utf8(&source[*start as usize..*end as usize]).unwrap_or("");
            let trimmed_len = text.trim_matches(|c: char| c == '\r' || c == '\n').len() as f32;
            (trimmed_len * font_size * 0.55).max(0.0)
        }
        NodeKind::Element { .. } => {
            let mut sum = 0.0;
            for child_styled in &styled.children {
                sum += compute_intrinsic_inline_width(Some(child_styled), dom, source);
            }
            sum
        }
        _ => 0.0,
    }
}

// ─── Layout Tree Builder ───────────────────────────────────────────

/// Build a layout tree from a StyledNode root.
/// Filters out `display: none` elements and groups mixed inline/block children.
pub fn build_layout_tree<'a>(styled_node: &'a StyledNode) -> Option<LayoutBox<'a>> {
    // Filter display: none
    if styled_node.styles.display == Display::None {
        return None;
    }

    let box_type = match styled_node.styles.display {
        Display::Block => BoxType::BlockNode,
        Display::Inline | Display::InlineBlock => BoxType::InlineNode,
        Display::None => unreachable!(),
    };

    let mut root_box = LayoutBox::new(box_type, Some(styled_node));

    // Recursively build children
    let mut child_boxes = Vec::new();
    for child in &styled_node.children {
        if let Some(child_box) = build_layout_tree(child) {
            child_boxes.push(child_box);
        }
    }

    // Process children to wrap inline nodes in anonymous block boxes if mixed
    let contains_blocks = child_boxes
        .iter()
        .any(|b| b.box_type == BoxType::BlockNode || b.box_type == BoxType::AnonymousBlock);

    if contains_blocks {
        let mut final_children = Vec::new();
        let mut anonymous_buffer: Option<LayoutBox<'a>> = None;

        for child in child_boxes {
            if child.box_type == BoxType::BlockNode || child.box_type == BoxType::AnonymousBlock {
                if let Some(anon) = anonymous_buffer.take() {
                    final_children.push(anon);
                }
                final_children.push(child);
            } else {
                let anon = anonymous_buffer
                    .get_or_insert_with(|| LayoutBox::new(BoxType::AnonymousBlock, None));
                anon.children.push(child);
            }
        }
        if let Some(anon) = anonymous_buffer {
            final_children.push(anon);
        }
        root_box.children = final_children;
    } else {
        root_box.children = child_boxes;
    }

    Some(root_box)
}

/// Compute top-level layout for a document given viewport dimensions.
pub fn layout_document<'a>(
    styled_root: &'a StyledNode,
    dom: &Dom,
    source: &[u8],
    viewport_width: f32,
    _viewport_height: f32,
) -> Option<LayoutBox<'a>> {
    let mut layout_root = build_layout_tree(styled_root)?;

    let initial_containing_block = Dimensions {
        content: Rect {
            x: 0.0,
            y: 0.0,
            width: viewport_width,
            height: 0.0,
        },
        ..Default::default()
    };

    layout_root.layout(initial_containing_block, dom, source);
    Some(layout_root)
}

// ─── Layout Inspector & ASCII Visualizer ──────────────────────────

impl<'a> LayoutBox<'a> {
    /// Print formatted ASCII layout tree to stdout.
    pub fn print_tree(&self, dom: &Dom, source: &[u8]) {
        println!("{}", self.format_tree(dom, source));
    }

    /// Format layout tree into a structured string.
    pub fn format_tree(&self, dom: &Dom, source: &[u8]) -> String {
        let mut output = String::new();
        self.format_node(dom, source, 0, &mut output);
        output
    }

    fn format_node(&self, dom: &Dom, source: &[u8], depth: usize, output: &mut String) {
        let indent = "  ".repeat(depth);
        let c = &self.dimensions.content;
        let m = &self.dimensions.margin;

        let tag_name = if let Some(styled) = self.styled_node {
            let node = dom.get(styled.node_id);
            match &node.kind {
                NodeKind::Document => "Document".to_string(),
                NodeKind::Element { tag_start, tag_end } => {
                    std::str::from_utf8(&source[*tag_start as usize..*tag_end as usize])
                        .unwrap_or("???")
                        .to_string()
                }
                NodeKind::Text { start, end } => {
                    let txt = std::str::from_utf8(&source[*start as usize..*end as usize])
                        .unwrap_or("???")
                        .trim();
                    format!("\"{}\"", txt)
                }
                NodeKind::Comment { .. } => "Comment".to_string(),
            }
        } else {
            "AnonymousBlock".to_string()
        };

        output.push_str(&format!(
            "{}{:?} <{}> (x: {:.1}, y: {:.1}, w: {:.1}, h: {:.1}) [margin: L{:.1} R{:.1} T{:.1} B{:.1}]\n",
            indent, self.box_type, tag_name, c.x, c.y, c.width, c.height, m.left, m.right, m.top, m.bottom
        ));

        for child in &self.children {
            child.format_node(dom, source, depth + 1, output);
        }
    }
}
