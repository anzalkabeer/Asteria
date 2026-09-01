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
use crate::values;
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
    FlexNode,
    GridNode,
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

    /// Calculate total number of layout boxes in this subtree.
    pub fn box_count(&self) -> usize {
        1 + self.children.iter().map(|c| c.box_count()).sum::<usize>()
    }

    /// Recursively compute geometry and position for this box and its subtree.
    pub fn layout(&mut self, containing_block: Dimensions, dom: &Dom, source: &[u8]) {
        match self.box_type {
            BoxType::BlockNode | BoxType::AnonymousBlock => {
                self.layout_block(containing_block, dom, source);
            }
            BoxType::FlexNode => {
                self.layout_flex(containing_block, dom, source);
            }
            BoxType::GridNode => {
                self.layout_grid(containing_block, dom, source);
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

        // Pre-calculate explicit height if specified so children can resolve percentage heights
        self.calculate_block_height(containing_block);

        // Step 3: Lay out children (block stacking or inline line-box formatting context)
        self.layout_block_children(dom, source);

        // Step 4: Calculate explicit height if specified
        self.calculate_block_height(containing_block);
    }

    /// Calculate width, padding, border, and margins for a block box
    /// using W3C width constraint equations.
    fn calculate_block_width(&mut self, containing_block: Dimensions) {
        let style = self.styled_node.map(|n| &n.styles);

        // Read values or defaults
        let auto_width = style.map(|s| s.width.is_auto()).unwrap_or(true);
        let specified_w = style
            .and_then(|s| s.width.resolve_against(containing_block.content.width))
            .unwrap_or(0.0);

        let left_is_auto = style.map_or(false, |s| s.margin.left.is_none());
        let right_is_auto = style.map_or(false, |s| s.margin.right.is_none());

        let ml = style.and_then(|s| s.margin.left).unwrap_or(0.0);
        let mr = style.and_then(|s| s.margin.right).unwrap_or(0.0);
        let margin_top = style.and_then(|s| s.margin.top).unwrap_or(0.0);
        let margin_bottom = style.and_then(|s| s.margin.bottom).unwrap_or(0.0);

        let padding_left = style.map(|s| s.padding.left).unwrap_or(0.0);
        let padding_right = style.map(|s| s.padding.right).unwrap_or(0.0);
        let padding_top = style.map(|s| s.padding.top).unwrap_or(0.0);
        let padding_bottom = style.map(|s| s.padding.bottom).unwrap_or(0.0);

        let border_left = style.map(|s| s.border_width.left).unwrap_or(0.0);
        let border_right = style.map(|s| s.border_width.right).unwrap_or(0.0);
        let border_top = style.map(|s| s.border_width.top).unwrap_or(0.0);
        let border_bottom = style.map(|s| s.border_width.bottom).unwrap_or(0.0);

        // box-sizing: border-box — specified width includes padding+border
        let box_sizing = style.map(|s| s.box_sizing).unwrap_or(values::BoxSizing::ContentBox);
        let content_width = if !auto_width && box_sizing == values::BoxSizing::BorderBox {
            (specified_w - padding_left - padding_right - border_left - border_right).max(0.0)
        } else {
            specified_w
        };

        let total_non_width = ml + mr + padding_left + padding_right + border_left + border_right;

        let (width, margin_left, margin_right) = if auto_width {
            // Auto width: any auto margins become 0 per CSS §10.3.3
            let used_ml = if left_is_auto { 0.0 } else { ml };
            let used_mr = if right_is_auto { 0.0 } else { mr };
            let non_width =
                used_ml + used_mr + padding_left + padding_right + border_left + border_right;
            let available = containing_block.content.width - non_width;

            if available < 0.0 {
                // Preserve signed residual in margin_right so constraint equation holds
                (0.0, used_ml, used_mr + available)
            } else {
                (available, used_ml, used_mr)
            }
        } else {
            // Specified width — calculate underflow
            let underflow = containing_block.content.width - (content_width + total_non_width);

            // CSS §10.3.3 — distribute underflow to auto margins
            let (ml_final, mr_final) = match (left_is_auto, right_is_auto) {
                (true, true) => {
                    if underflow < 0.0 {
                        // In LTR with both auto and negative underflow: margin-left becomes 0, margin-right gets underflow
                        (0.0, underflow)
                    } else {
                        let half = underflow / 2.0;
                        (half, half)
                    }
                }
                (true, false) => (underflow, mr),
                (false, true) => (ml, underflow),
                (false, false) => (ml, mr + underflow), // Over-constrained: apply residual to right margin
            };

            (content_width, ml_final, mr_final)
        };

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

                let margin_left = style.and_then(|s| s.margin.left).unwrap_or(0.0);
                let margin_right = style.and_then(|s| s.margin.right).unwrap_or(0.0);
                let margin_top = style.and_then(|s| s.margin.top).unwrap_or(0.0);
                let margin_bottom = style.and_then(|s| s.margin.bottom).unwrap_or(0.0);

                let padding_left = style.map(|s| s.padding.left).unwrap_or(0.0);
                let padding_right = style.map(|s| s.padding.right).unwrap_or(0.0);
                let padding_top = style.map(|s| s.padding.top).unwrap_or(0.0);
                let padding_bottom = style.map(|s| s.padding.bottom).unwrap_or(0.0);

                let border_left = style.map(|s| s.border_width.left).unwrap_or(0.0);
                let border_right = style.map(|s| s.border_width.right).unwrap_or(0.0);
                let border_top = style.map(|s| s.border_width.top).unwrap_or(0.0);
                let border_bottom = style.map(|s| s.border_width.bottom).unwrap_or(0.0);

                // Compute intrinsic content width and height from text / child layout
                let content_w = compute_intrinsic_inline_width(child.styled_node, dom, source);
                let font_size = style.map(|s| s.font_size).unwrap_or(16.0);
                let content_h = style.map(|s| s.line_height).unwrap_or(font_size * 1.2);

                let outer_w =
                    content_w + margin_left + margin_right + border_left + border_right + padding_left + padding_right;
                let outer_h =
                    content_h + margin_top + margin_bottom + border_top + border_bottom + padding_top + padding_bottom;

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

                // Recursively layout child's descendants if any
                if !child.children.is_empty() {
                    child.layout(child.dimensions, dom, source);
                }

                // Advance horizontal cursor
                cursor_x += outer_w;
                current_line_height = current_line_height.max(outer_h);
            }

            self.dimensions.content.height =
                (cursor_y + current_line_height) - self.dimensions.content.y;
        } else {
            // ─── Block Formatting Context (Vertical Stack Flow with Margin Collapsing) ────────
            let mut prev_border_box_bottom = 0.0;
            let mut prev_margin_bottom = 0.0;
            let mut is_first = true;
            let parent_content_height = self.dimensions.content.height;

            for child in &mut self.children {
                let child_margin_top = child
                    .styled_node
                    .and_then(|n| n.styles.margin.top)
                    .unwrap_or(0.0);

                let mut container = self.dimensions;
                container.content.height = parent_content_height;

                if is_first {
                    container.content.y = self.dimensions.content.y;
                    child.layout(container, dom, source);

                    prev_border_box_bottom =
                        child.dimensions.margin.top + child.dimensions.border_box().height;
                    prev_margin_bottom = child.dimensions.margin.bottom;
                    is_first = false;
                } else {
                    // Vertical margin collapsing: CSS 2.1 §8.3.1 (positive/negative/mixed)
                    let collapsed_margin = collapse_margins(prev_margin_bottom, child_margin_top);
                    container.content.y = self.dimensions.content.y
                        + prev_border_box_bottom
                        + collapsed_margin
                        - child_margin_top;
                    child.layout(container, dom, source);

                    prev_border_box_bottom +=
                        collapsed_margin + child.dimensions.border_box().height;
                    prev_margin_bottom = child.dimensions.margin.bottom;
                }
            }

            self.dimensions.content.height = if is_first {
                0.0
            } else {
                prev_border_box_bottom + prev_margin_bottom
            };
        }
    }

    /// Override content height if explicitly specified on the element's style
    fn calculate_block_height(&mut self, containing_block: Dimensions) {
        if let Some(h) = self
            .styled_node
            .and_then(|n| n.styles.height.resolve_against(containing_block.content.height))
        {
            self.dimensions.content.height = h;
        }
    }

    // ─── Flexbox Layout Handling ────────────────────────────────────

    fn layout_flex(&mut self, containing_block: Dimensions, dom: &Dom, source: &[u8]) {
        self.calculate_block_width(containing_block);
        self.calculate_block_position(containing_block);

        let start_x = self.dimensions.content.x;
        let container_max_x = start_x + self.dimensions.content.width;

        let mut cursor_x = start_x;
        let mut cursor_y = self.dimensions.content.y;
        let mut max_line_height: f32 = 0.0;
        let mut total_flex_height: f32 = 0.0;
        let gap = self
            .styled_node
            .map(|s| s.styles.grid_gap.right)
            .unwrap_or(0.0);

        let mut unconstrained_auto_count = 0;
        let mut fixed_or_intrinsic_width = 0.0;
        for child in &self.children {
            let margin_w = child.styled_node.map_or(0.0, |n| {
                n.styles.margin.left.unwrap_or(0.0) + n.styles.margin.right.unwrap_or(0.0)
            });
            let padding_w = child
                .styled_node
                .map_or(0.0, |n| n.styles.padding.left + n.styles.padding.right);
            let border_w = child.styled_node.map_or(0.0, |n| {
                n.styles.border_width.left + n.styles.border_width.right
            });
            let extra = margin_w + padding_w + border_w;

            if let Some(w) = child
                .styled_node
                .and_then(|n| n.styles.width.resolve_against(self.dimensions.content.width))
            {
                fixed_or_intrinsic_width += w + extra;
            } else {
                let intrinsic = compute_intrinsic_inline_width(child.styled_node, dom, source);
                if intrinsic > 0.0 {
                    fixed_or_intrinsic_width += intrinsic + extra;
                } else {
                    unconstrained_auto_count += 1;
                    fixed_or_intrinsic_width += extra; // just extra
                }
            }
        }

        let total_gaps = if self.children.is_empty() {
            0.0
        } else {
            gap * (self.children.len() - 1) as f32
        };
        let available_for_auto =
            (self.dimensions.content.width - fixed_or_intrinsic_width - total_gaps).max(0.0);
        let fair_share = if unconstrained_auto_count > 0 {
            available_for_auto / unconstrained_auto_count as f32
        } else {
            0.0
        };

        for child in &mut self.children {
            let margin_w = child.styled_node.map_or(0.0, |n| {
                n.styles.margin.left.unwrap_or(0.0) + n.styles.margin.right.unwrap_or(0.0)
            });
            let padding_w = child
                .styled_node
                .map_or(0.0, |n| n.styles.padding.left + n.styles.padding.right);
            let border_w = child.styled_node.map_or(0.0, |n| {
                n.styles.border_width.left + n.styles.border_width.right
            });

            // If no explicit width, use intrinsic content width or
            // divide remaining container space equally among auto-width children
            let child_w = child
                .styled_node
                .and_then(|n| n.styles.width.resolve_against(self.dimensions.content.width))
                .unwrap_or_else(|| {
                    // Compute intrinsic width from text/child content
                    let intrinsic = compute_intrinsic_inline_width(child.styled_node, dom, source);
                    if intrinsic > 0.0 {
                        intrinsic
                    } else {
                        // Fallback: fair share of remaining container width
                        fair_share
                    }
                });

            let outer_item_w = child_w + margin_w + padding_w + border_w;

            // Flex Row Line Wrap Check: if adding child exceeds container max width, wrap to next flex row!
            if cursor_x > start_x && (cursor_x + outer_item_w > container_max_x) {
                cursor_x = start_x;
                cursor_y += max_line_height + gap;
                total_flex_height += max_line_height + gap;
                max_line_height = 0.0;
            }

            let mut item_container = self.dimensions;
            item_container.content.x = cursor_x;
            item_container.content.y = cursor_y;
            item_container.content.width = child_w;

            child.layout(item_container, dom, source);

            let actual_w = child.dimensions.margin_box().width;
            let actual_h = child.dimensions.margin_box().height;

            cursor_x += actual_w + gap;
            max_line_height = max_line_height.max(actual_h);
        }

        self.dimensions.content.height = total_flex_height + max_line_height;
        self.calculate_block_height(containing_block);
    }

    // ─── Grid Layout Algorithm ──────────────────────────────────────

    fn layout_grid(&mut self, containing_block: Dimensions, dom: &Dom, source: &[u8]) {
        self.calculate_block_width(containing_block);
        self.calculate_block_position(containing_block);

        let style = self.styled_node.unwrap().styles.clone();

        if style.grid_template_columns.is_empty() && style.grid_template_rows.is_empty() {
            self.layout_block_children(dom, source);
            self.calculate_block_height(containing_block);
            return;
        }

        let container_w = self.dimensions.content.width;
        let gap_x = style.grid_gap.right;
        let gap_y = style.grid_gap.bottom;

        let mut col_px_sizes = vec![0.0; style.grid_template_columns.len()];
        let mut total_fr = 0.0;
        let mut remaining_w =
            container_w - (style.grid_template_columns.len().saturating_sub(1) as f32 * gap_x);

        for (i, track) in style.grid_template_columns.iter().enumerate() {
            match track {
                crate::values::GridTrack::Px(val) => {
                    col_px_sizes[i] = *val;
                    remaining_w -= *val;
                }
                crate::values::GridTrack::Percent(val) => {
                    let w = container_w * (*val / 100.0);
                    col_px_sizes[i] = w;
                    remaining_w -= w;
                }
                crate::values::GridTrack::Auto => total_fr += 1.0,
                crate::values::GridTrack::Fr(val) => total_fr += *val,
            }
        }

        if total_fr > 0.0 && remaining_w > 0.0 {
            for (i, track) in style.grid_template_columns.iter().enumerate() {
                match track {
                    crate::values::GridTrack::Fr(val) => {
                        col_px_sizes[i] = remaining_w * (*val / total_fr)
                    }
                    crate::values::GridTrack::Auto => {
                        col_px_sizes[i] = remaining_w * (1.0 / total_fr)
                    }
                    _ => {}
                }
            }
        }

        let mut current_row = 0;
        let mut current_col = 0;
        let num_cols = style.grid_template_columns.len().max(1);
        let mut max_row_heights: std::collections::HashMap<usize, f32> =
            std::collections::HashMap::new();

        let start_x = self.dimensions.content.x;
        let start_y = self.dimensions.content.y;

        for child in &mut self.children {
            let mut col_span = 1;

            if let Some(child_style) = child.styled_node {
                match child_style.styles.grid_column {
                    crate::values::GridPlacement::Span(s) => col_span = s.max(1) as usize,
                    crate::values::GridPlacement::Line(l) => current_col = (l.max(1) - 1) as usize,
                    _ => {}
                }
                if let crate::values::GridPlacement::Line(l) = child_style.styles.grid_row {
                    current_row = (l.max(1) - 1) as usize;
                }
            }

            if current_col + col_span > num_cols {
                current_col = 0;
                current_row += 1;
            }

            let mut child_x = start_x;
            for c in 0..current_col {
                child_x += *col_px_sizes.get(c).unwrap_or(&0.0) + gap_x;
            }

            let mut child_w = 0.0;
            for c in current_col..(current_col + col_span).min(num_cols) {
                child_w += *col_px_sizes.get(c).unwrap_or(&0.0);
                if c > current_col {
                    child_w += gap_x;
                }
            }

            let mut child_y = start_y;
            for r in 0..current_row {
                child_y += *max_row_heights.get(&r).unwrap_or(&0.0) + gap_y;
            }

            let mut item_container = self.dimensions;
            item_container.content.x = child_x;
            item_container.content.y = child_y;
            item_container.content.width = child_w;

            child.layout(item_container, dom, source);

            let actual_h = child.dimensions.margin_box().height;
            let current_max = *max_row_heights.get(&current_row).unwrap_or(&0.0);
            max_row_heights.insert(current_row, current_max.max(actual_h));

            current_col += col_span;
        }

        let mut total_h = 0.0;
        let num_rows = if max_row_heights.is_empty() {
            0
        } else {
            *max_row_heights.keys().max().unwrap() + 1
        };
        for r in 0..num_rows {
            total_h += *max_row_heights.get(&r).unwrap_or(&0.0);
            if r > 0 {
                total_h += gap_y;
            }
        }

        self.dimensions.content.height = total_h;
        self.calculate_block_height(containing_block);
    }

    // ─── Inline Layout Handling ────────────────────────────────────

    fn layout_inline(&mut self, containing_block: Dimensions, dom: &Dom, source: &[u8]) {
        let style = self.styled_node.map(|n| &n.styles);

        // Compute edge sizes from style
        let margin_left = style.and_then(|s| s.margin.left).unwrap_or(0.0);
        let margin_right = style.and_then(|s| s.margin.right).unwrap_or(0.0);
        let margin_top = style.and_then(|s| s.margin.top).unwrap_or(0.0);
        let margin_bottom = style.and_then(|s| s.margin.bottom).unwrap_or(0.0);

        let padding_left = style.map_or(0.0, |s| s.padding.left);
        let padding_right = style.map_or(0.0, |s| s.padding.right);
        let padding_top = style.map_or(0.0, |s| s.padding.top);
        let padding_bottom = style.map_or(0.0, |s| s.padding.bottom);

        let border_left = style.map_or(0.0, |s| s.border_width.left);
        let border_right = style.map_or(0.0, |s| s.border_width.right);
        let border_top = style.map_or(0.0, |s| s.border_width.top);
        let border_bottom = style.map_or(0.0, |s| s.border_width.bottom);

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

        // Retain position, width, and height assigned by the inline formatting context
        self.dimensions.content.x = containing_block.content.x;
        self.dimensions.content.y = containing_block.content.y;
        self.dimensions.content.width = containing_block.content.width;
        self.dimensions.content.height = containing_block.content.height;

        // Layout children
        self.layout_block_children(dom, source);

        // Height: explicit or content-driven
        if let Some(h) = style.and_then(|s| s.height.resolve_against(containing_block.content.height)) {
            self.dimensions.content.height = h;
        }
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

    if let Some(w) = styled.styles.width.resolve_against(0.0) {
        return w;
    }

    let node = dom.get(styled.node_id);
    match &node.kind {
        NodeKind::Text { .. } => {
            let font_size = styled.styles.font_size;
            let text = node.text_content(source);
            let trimmed = text.trim_matches(|c: char| c == '\r' || c == '\n');
            let width: f32 = trimmed
                .chars()
                .map(|ch| estimate_char_width_ratio(ch) * font_size)
                .sum();
            width.max(0.0)
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

/// Estimate the proportional width ratio of a character relative to the font size.
///
/// Returns a multiplier such that `ratio * font_size` approximates the glyph advance width.
/// Categories:
///   - Narrow punctuation/thin letters: ~0.28em
///   - Whitespace: 0.25em
///   - Normal lowercase: ~0.52em
///   - Uppercase/digits: ~0.65em
///   - Wide glyphs (W, M, @, %, etc.): ~0.85em
///   - CJK ideographs: ~1.05em (fullwidth)
fn estimate_char_width_ratio(ch: char) -> f32 {
    match ch {
        // Thin / narrow glyphs
        'i' | 'l' | 'j' | '!' | '|' | '\'' | ',' | '.' | ':' | ';' | '`' => 0.28,
        'f' | 'r' | 't' => 0.35,
        // Wide lowercase
        'm' | 'w' => 0.78,
        // Normal lowercase (catch-all after specific overrides)
        'a'..='z' => 0.52,
        // Digits
        '0'..='9' => 0.58,
        // Wide uppercase and symbols
        'W' | 'M' => 0.88,
        'A'..='Z' => 0.65,
        '@' | '#' | '%' | '&' | '$' => 0.85,
        // CJK Unified Ideographs (fullwidth)
        '\u{4e00}'..='\u{9fff}' | '\u{3400}'..='\u{4dbf}' | '\u{f900}'..='\u{faff}' => 1.05,
        // CJK Fullwidth punctuation
        '\u{3000}'..='\u{303f}' | '\u{ff00}'..='\u{ffef}' => 1.0,
        // Whitespace (covers space, tab, newline, CR, NBSP, and all Unicode whitespace)
        _ if ch.is_whitespace() => 0.25,
        // Fallback for other characters
        _ => 0.55,
    }
}

// ─── Margin Collapsing Helper ──────────────────────────────────────

/// Collapse two adjacent vertical margins according to CSS 2.1 §8.3.1:
/// - If both are positive: max(m1, m2)
/// - If both are negative: min(m1, m2) (the most negative value)
/// - If one is positive and one is negative: max(positive) + min(negative)
pub fn collapse_margins(m1: f32, m2: f32) -> f32 {
    let pos = m1.max(0.0).max(m2.max(0.0));
    let neg = m1.min(0.0).min(m2.min(0.0));
    pos + neg
}

// ─── Layout Tree Builder ───────────────────────────────────────────

/// Build a layout tree from a StyledNode root.
/// Filters out `display: none` elements and groups mixed inline/block children.
pub fn build_layout_tree<'a>(
    styled_node: &'a StyledNode,
    dom: &Dom,
    source: &[u8],
) -> Option<LayoutBox<'a>> {
    // Filter display: none
    if styled_node.styles.display == Display::None {
        return None;
    }

    let box_type = match styled_node.styles.display {
        Display::Block => BoxType::BlockNode,
        Display::Flex => BoxType::FlexNode,
        Display::Grid => BoxType::GridNode,
        Display::Inline | Display::InlineBlock => BoxType::InlineNode,
        Display::None => unreachable!(),
    };

    let mut root_box = LayoutBox::new(box_type, Some(styled_node));

    // Recursively build children
    let mut child_boxes = Vec::new();
    for child in &styled_node.children {
        if let Some(child_box) = build_layout_tree(child, dom, source) {
            child_boxes.push(child_box);
        }
    }

    // Process children: Flex/Grid containers do NOT create anonymous blocks for whitespace nodes
    if box_type == BoxType::FlexNode || box_type == BoxType::GridNode {
        root_box.children = child_boxes
            .into_iter()
            .filter(|child| {
                if child.box_type == BoxType::InlineNode {
                    // Check if child contains non-whitespace text
                    let is_empty = child.styled_node.is_none_or(|n| {
                        if let NodeKind::Text { start, end } = dom.get(n.node_id).kind {
                            let text = std::str::from_utf8(&source[start as usize..end as usize])
                                .unwrap_or("");
                            text.trim().is_empty()
                        } else {
                            false
                        }
                    });
                    !is_empty
                } else {
                    true
                }
            })
            .collect();
    } else {
        // Process children to wrap inline nodes in anonymous block boxes if mixed
        let contains_blocks = child_boxes.iter().any(|b| {
            b.box_type == BoxType::BlockNode
                || b.box_type == BoxType::AnonymousBlock
                || b.box_type == BoxType::FlexNode
                || b.box_type == BoxType::GridNode
        });

        if contains_blocks {
            let mut final_children = Vec::new();
            let mut anonymous_buffer: Option<LayoutBox<'a>> = None;

            for child in child_boxes {
                if child.box_type == BoxType::BlockNode
                    || child.box_type == BoxType::AnonymousBlock
                    || child.box_type == BoxType::FlexNode
                    || child.box_type == BoxType::GridNode
                {
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
    }

    Some(root_box)
}

/// Compute top-level layout for a document given viewport dimensions.
pub fn layout_document<'a>(
    styled_root: &'a StyledNode,
    dom: &Dom,
    source: &[u8],
    viewport_width: f32,
    viewport_height: f32,
) -> Option<LayoutBox<'a>> {
    let mut layout_root = build_layout_tree(styled_root, dom, source)?;

    let initial_containing_block = Dimensions {
        content: Rect {
            x: 0.0,
            y: 0.0,
            width: viewport_width,
            height: viewport_height,
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
