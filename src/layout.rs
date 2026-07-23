// ─── Asteria Layout Engine ─────────────────────────────────────────
//
// The layout engine takes a Styled DOM (tree of StyledNode carrying
// resolved ComputedStyle values) and computes 2D coordinates (x, y)
// and box model dimensions (width, height, margins, padding, borders)
// for every visible element.
//
// ── Note on 3D Transforms, Depth & Animations ──────────────────────
// In browser architecture, the Layout stage computes the base 2D content
// box geometry (x, y, width, height) in document space. 3D transforms
// (translate3d, perspective), z-index depth layering, and GPU-accelerated
// animations are applied downstream during Display List generation and
// wgpu GPU Compositing. The 2D layout tree provides the reference frame
// that all downstream 3D graphics passes build upon.

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
    ///i was aought that maybe i dont have to do this using recursively
    pub fn layout(&mut self, containing_block: Dimensions) {
        match self.box_type {
            BoxType::BlockNode | BoxType::AnonymousBlock => {
                self.layout_block(containing_block);
            }
            BoxType::InlineNode => {
                self.layout_inline(containing_block);
            }
        }
    }
/// from here there is ai writing the code for the layout algorithm and i think it is working fine but i am not sure if it is correct or not thats why 
    // ─── Block Layout Algorithm ───────────────────────────────────

    fn layout_block(&mut self, containing_block: Dimensions) {
        // Step 1: Calculate horizontal width and margins
        self.calculate_block_width(containing_block);

        // Step 2: Calculate position (x, y) relative to document origin
        self.calculate_block_position(containing_block);

        // Step 3: Lay out children and compute content height
        self.layout_block_children();

        // Step 4: Calculate explicit height if specified
        self.calculate_block_height();
    }

    /// Calculate width, padding, border, and margins for a block box
    /// using W3C width constraint equations.
    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.styled_node.map(|n| &n.styles);

        // Read values or defaults
        let auto_width = style.map_or(true, |s| s.width.is_none());
        let mut width = style.and_then(|s| s.width).unwrap_or(0.0);

        let mut margin_left = style.map_or(0.0, |s| s.margin.left);
        let mut margin_right = style.map_or(0.0, |s| s.margin.right);
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

        let total_non_width = margin_left
            + margin_right
            + padding_left
            + padding_right
            + border_left
            + border_right;

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

    /// Layout children inside this block box sequentially top to bottom
    fn layout_block_children(&mut self) {
        let mut content_height = 0.0;

        for child in &mut self.children {
            // Container for child is current block dimensions with running height
            let mut container = self.dimensions;
            container.content.height = content_height;

            child.layout(container);

            // Increment parent content height by child's margin box height
            content_height += child.dimensions.margin_box().height;
        }

        self.dimensions.content.height = content_height;
    }

    /// Override content height if explicitly specified on the element's style
    fn calculate_block_height(&mut self) {
        if let Some(style) = self.styled_node.map(|n| &n.styles) {
            if let Some(h) = style.height {
                self.dimensions.content.height = h;
            }
        }
    }

    // ─── Inline Layout Basic Handling ─────────────────────────────

    fn layout_inline(&mut self, containing_block: Dimensions) {
        // Basic inline box geometry inherits container position & default font line-height
        self.calculate_block_width(containing_block);
        self.calculate_block_position(containing_block);

        let line_height = self
            .styled_node
            .map_or(16.0, |n| n.styles.line_height);

        self.dimensions.content.height = line_height;
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
                let anon = anonymous_buffer.get_or_insert_with(|| {
                    LayoutBox::new(BoxType::AnonymousBlock, None)
                });
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
    viewport_width: f32,
    viewport_height: f32,
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

    layout_root.layout(initial_containing_block);
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
