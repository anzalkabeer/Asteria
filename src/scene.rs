// ─── ASTERIA Scene Graph ─────────────────────────────────────────
//
// A flat, data-oriented scene graph that converts the high-level
// DisplayList into a contiguous, cache-friendly representation
// optimized for GPU submission and incremental invalidation.
//
// ASTERIA is a data-oriented, segment-based, GPU-first browser engine.
// The scene graph is the bridge between the Paint Engine's logical
// draw commands and the GPU Renderer's physical vertex buffers.
//
// Design principles (Pillar 1 + Pillar 3):
//   - Flat Vec<SceneNode> storage, NOT a pointer-heavy tree
//   - Parallel arrays for colors and text (struct-of-arrays layout)
//   - Per-node dirty flags for incremental re-rendering
//   - Segment assignment for region-based GPU tile caching

use crate::layout::{EdgeSizes, Rect};

// ─── Scene Node Identification ───────────────────────────────────

/// Index into the SceneGraph's flat node storage
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneNodeId(pub u32);

impl SceneNodeId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

// ─── Scene Node Types ────────────────────────────────────────────

/// What kind of visual primitive this scene node represents
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SceneNodeKind {
    /// Solid color filled rectangle (element backgrounds)
    SolidRect,
    /// Rounded rectangle filled with a color
    RoundedRect { radius: crate::values::BorderRadius },
    /// Box shadow visual primitive
    BoxShadow { shadow: crate::values::BoxShadow },
    /// Box borders (top, right, bottom, left edges)
    Border { widths: EdgeSizes },
    /// Text fragment at a position
    Text { font_size: f32 },
    /// Decoded image mapped to a rectangle
    Image,
    /// Grouping container (no visual output, used for hierarchy)
    Container,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeState {
    Normal,
    Hovered,
    Active,
}
/// A single visual primitive in the scene, stored contiguously in Vec<SceneNode>
///
/// Layout:  [0][1][2][3][4][5]...
/// All nodes live in one contiguous memory block for maximum cache locality.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneNode {
    /// Bounding box in document coordinates (x, y, width, height)
    pub rect: Rect,
    /// What visual primitive to draw
    pub kind: SceneNodeKind,
    /// Parent node index (None for root nodes)
    pub parent: Option<SceneNodeId>,
    /// Paint stacking order (lower = painted first = behind)
    pub z_order: u32,
    /// Which viewport segment this node belongs to (Pillar 4)
    pub segment_id: u16,
    /// Incremental invalidation flag (Pillar 3)
    /// When true, this node needs re-rendering
    pub dirty: bool,
    /// Interactive visual state of this node (hovered, active, normal)
    pub state: NodeState,
    /// Target URL of the anchor tag if the node is an interactive link, otherwise None
    pub link_url: Option<String>,
    /// Active clip rectangle if this node is inside an overflow-hidden container
    pub clip: Option<Rect>,
}

impl Default for SceneNode {
    fn default() -> Self {
        SceneNode {
            rect: Rect::default(),
            kind: SceneNodeKind::SolidRect,
            parent: None,
            z_order: 0,
            segment_id: 0,
            dirty: true,
            state: NodeState::Normal,
            link_url: None,
            clip: None,
        }
    }
}

// ─── Text Run Data ───────────────────────────────────────────────

/// Text content associated with a Text scene node
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    pub text: String,
    pub font_size: f32,
}

// ─── Scene Graph (Flat Contiguous Storage) ───────────────────────

/// The flat, data-oriented scene graph.
///
/// Uses struct-of-arrays layout for cache-friendly iteration:
///   - Iterate only `nodes` when doing spatial queries (no color/text cache pollution)
///   - Iterate only `colors` when uploading color buffers to GPU
///   - Iterate only `texts` when doing text shaping
#[derive(Debug, Clone, PartialEq)]
pub struct SceneGraph {
    /// Contiguous node storage — cache-friendly sequential access
    pub nodes: Vec<SceneNode>,
    /// Parallel array: RGBA color per node (0.0..1.0 normalized for GPU)
    pub colors: Vec<[f32; 4]>,
    /// Parallel array: text content per node (None for non-text nodes)
    pub texts: Vec<Option<TextRun>>,
}

impl SceneGraph {
    pub fn new() -> Self {
        SceneGraph {
            nodes: Vec::new(),
            colors: Vec::new(),
            texts: Vec::new(),
        }
    }

    /// Pre-allocate capacity for expected node count (reduces re-allocations)
    pub fn with_capacity(capacity: usize) -> Self {
        SceneGraph {
            nodes: Vec::with_capacity(capacity),
            colors: Vec::with_capacity(capacity),
            texts: Vec::with_capacity(capacity),
        }
    }

    /// Add a scene node and return its ID
    pub fn push(&mut self, node: SceneNode, color: [f32; 4], text: Option<TextRun>) -> SceneNodeId {
        let id = SceneNodeId(self.nodes.len() as u32);
        self.nodes.push(node);
        self.colors.push(color);
        self.texts.push(text);
        id
    }

    /// Total number of scene nodes
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Clear all nodes for a fresh frame (preserves allocated capacity)
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.colors.clear();
        self.texts.clear();
    }

    // ─── Incremental Invalidation (Pillar 3) ─────────────────────

    /// Mark a node as dirty and propagate dirtiness up to ancestors.
    /// Only walks until it hits an already-dirty ancestor (early exit).
    pub fn invalidate(&mut self, node_id: SceneNodeId) {
        let idx = node_id.index();
        if idx >= self.nodes.len() {
            return;
        }
        self.nodes[idx].dirty = true;

        // Walk up to root marking ancestors dirty
        let mut current = self.nodes[idx].parent;
        while let Some(parent_id) = current {
            let pidx = parent_id.index();
            if pidx >= self.nodes.len() {
                break;
            }
            if self.nodes[pidx].dirty {
                break; // Already dirty — stop propagation (saves work)
            }
            self.nodes[pidx].dirty = true;
            current = self.nodes[pidx].parent;
        }
    }

    /// Count how many nodes are currently dirty (useful for metrics/debugging)
    pub fn dirty_count(&self) -> usize {
        self.nodes.iter().filter(|n| n.dirty).count()
    }

    /// Clear all dirty flags after a successful re-render
    pub fn clear_dirty(&mut self) {
        for node in &mut self.nodes {
            node.dirty = false;
        }
    }

    /// Collect IDs of all dirty nodes (for targeted re-rendering)
    pub fn dirty_nodes(&self) -> Vec<SceneNodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.dirty)
            .map(|(i, _)| SceneNodeId(i as u32))
            .collect()
    }

    // ─── Segment Queries (Pillar 4) ──────────────────────────────

    /// Get all node indices belonging to a specific viewport segment
    pub fn nodes_in_segment(&self, segment_id: u16) -> Vec<SceneNodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.segment_id == segment_id)
            .map(|(i, _)| SceneNodeId(i as u32))
            .collect()
    }

    /// Check if any node in a segment is dirty
    pub fn is_segment_dirty(&self, segment_id: u16) -> bool {
        self.nodes
            .iter()
            .any(|n| n.segment_id == segment_id && n.dirty)
    }

    // ─── Hit Testing (Pillar 5 — Interaction) ────────────────────

    /// Point-in-bounding-box spatial query.
    /// Returns the topmost (highest z_order) SceneNode under (x, y).
    ///
    /// Uses a zero-allocation single-pass scan with max_by_key,
    /// safe to call on every CursorMoved event (~60Hz).
    pub fn hit_test(&self, x: f32, y: f32) -> Option<SceneNodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| {
                let r = &node.rect;
                x >= r.x && x <= r.x + r.width && y >= r.y && y <= r.y + r.height
            })
            .max_by_key(|(_, node)| node.z_order)
            .map(|(i, _)| SceneNodeId(i as u32))
    }

    /// Returns the kind of a node by ID (useful for interaction dispatch).
    pub fn node_kind(&self, id: SceneNodeId) -> Option<&SceneNodeKind> {
        self.nodes.get(id.index()).map(|n| &n.kind)
    }

    /// Returns the bounding rect of a node by ID.
    pub fn node_rect(&self, id: SceneNodeId) -> Option<crate::layout::Rect> {
        self.nodes.get(id.index()).map(|n| n.rect)
    }

    pub fn set_node_state(&mut self, id: SceneNodeId, state: NodeState) -> bool {
        let idx = id.index();
        if idx < self.nodes.len() && self.nodes[idx].state != state {
            self.nodes[idx].state = state;
            self.invalidate(id);
            true
        } else {
            false
        }
    }

    /// Get link URL if the node represents an HTML <a> tag
    pub fn node_url(&self, id: SceneNodeId) -> Option<&str> {
        self.nodes
            .get(id.index())
            .and_then(|n| n.link_url.as_deref())
    }

    /// Collect unique segment IDs of all currently dirty nodes
    pub fn dirty_segments(&self) -> Vec<u16> {
        let mut segs: Vec<u16> = self
            .nodes
            .iter()
            .filter(|n| n.dirty)
            .map(|n| n.segment_id)
            .collect();
        segs.sort_unstable();
        segs.dedup();
        segs
    }
}

impl Default for SceneGraph {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Display List → Scene Graph Conversion ───────────────────────

use crate::paint::{DisplayCommand, DisplayList};
use crate::values::Color;

/// Convert a Color to normalized GPU RGBA [0.0..1.0]
fn color_to_rgba(color: &Color) -> [f32; 4] {
    let (r, g, b, a) = color.to_rgba();
    [
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ]
}

/// Assign a segment ID based on y-position and segment height
fn assign_segment(y: f32, segment_height: f32) -> u16 {
    if segment_height <= 0.0 {
        return 0;
    }
    (y / segment_height) as u16
}

/// Build a flat SceneGraph from a DisplayList.
///
/// This is the core bridge between the Paint Engine output (logical commands)
/// and the GPU Renderer input (flat, contiguous, segment-tagged scene nodes).
///
/// Parent assignment uses a positional heuristic: SolidColor rectangles
/// (element backgrounds) that fully contain a subsequent node's rect are
/// treated as its parent. The deepest (most recently pushed) match wins.
pub fn build_scene_graph(display_list: &DisplayList, segment_height: f32) -> SceneGraph {
    let mut scene = SceneGraph::with_capacity(display_list.commands.len());
    let mut z_order: u32 = 0;

    // Stack of (SceneNodeId, Rect) for positional parent assignment.
    // Only SolidColor nodes (backgrounds) act as potential parents.
    let mut parent_stack: Vec<(SceneNodeId, Rect)> = Vec::new();
    let mut clip_stack: Vec<Rect> = Vec::new();

    for cmd in &display_list.commands {
        let node_rect = cmd_bounding_rect(cmd);

        // Pop containers that no longer contain the current node.
        // This keeps the stack proportional to nesting depth, not total node count.
        while let Some((_, top_rect)) = parent_stack.last() {
            if !rect_contains(top_rect, &node_rect) {
                parent_stack.pop();
            } else {
                break;
            }
        }
        let parent_id = parent_stack.last().map(|(id, _)| *id);
        let active_clip = clip_stack.last().copied();

        match cmd {
            DisplayCommand::SolidColor {
                color,
                rect,
                link_url,
            } => {
                let seg = assign_segment(rect.y, segment_height);
                let id = scene.push(
                    SceneNode {
                        rect: *rect,
                        kind: SceneNodeKind::SolidRect,
                        parent: parent_id,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                        state: NodeState::Normal,
                        link_url: link_url.clone(),
                        clip: active_clip,
                    },
                    color_to_rgba(color),
                    None,
                );
                // SolidColor nodes (backgrounds) can be parents of subsequent nodes
                parent_stack.push((id, *rect));
                z_order += 1;
            }
            DisplayCommand::RoundedRect {
                color,
                rect,
                radius,
                link_url,
            } => {
                let seg = assign_segment(rect.y, segment_height);
                let id = scene.push(
                    SceneNode {
                        rect: *rect,
                        kind: SceneNodeKind::RoundedRect { radius: *radius },
                        parent: parent_id,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                        state: NodeState::Normal,
                        link_url: link_url.clone(),
                        clip: active_clip,
                    },
                    color_to_rgba(color),
                    None,
                );
                parent_stack.push((id, *rect));
                z_order += 1;
            }
            DisplayCommand::BoxShadow {
                rect,
                shadow,
                link_url,
            } => {
                let blur = shadow.blur_radius;
                let expanded_rect = Rect {
                    x: rect.x - blur,
                    y: rect.y - blur,
                    width: rect.width + 2.0 * blur,
                    height: rect.height + 2.0 * blur,
                };
                let seg = assign_segment(expanded_rect.y, segment_height);
                scene.push(
                    SceneNode {
                        rect: expanded_rect,
                        kind: SceneNodeKind::BoxShadow { shadow: *shadow },
                        parent: parent_id,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                        state: NodeState::Normal,
                        link_url: link_url.clone(),
                        clip: active_clip,
                    },
                    color_to_rgba(&shadow.color),
                    None,
                );
                z_order += 1;
            }
            DisplayCommand::PushClip { rect } => {
                let current_clip = match clip_stack.last() {
                    Some(parent_clip) => intersect_rect(parent_clip, rect),
                    None => *rect,
                };
                clip_stack.push(current_clip);
            }
            DisplayCommand::PopClip => {
                clip_stack.pop();
            }
            DisplayCommand::Border {
                color,
                rect,
                border_width,
                link_url,
            } => {
                let seg = assign_segment(rect.y, segment_height);
                scene.push(
                    SceneNode {
                        rect: *rect,
                        kind: SceneNodeKind::Border {
                            widths: *border_width,
                        },
                        parent: parent_id,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                        state: NodeState::Normal,
                        link_url: link_url.clone(),
                        clip: active_clip,
                    },
                    color_to_rgba(color),
                    None,
                );
                z_order += 1;
            }
            DisplayCommand::Text {
                text,
                x,
                y,
                target_width,
                font_size,
                line_height,
                color,
                link_url,
            } => {
                let rect = compute_text_rect(text, *x, *y, *target_width, *line_height);
                let seg = assign_segment(*y, segment_height);
                scene.push(
                    SceneNode {
                        rect,
                        kind: SceneNodeKind::Text {
                            font_size: *font_size,
                        },
                        parent: parent_id,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                        state: NodeState::Normal,
                        link_url: link_url.clone(),
                        clip: active_clip,
                    },
                    color_to_rgba(color),
                    Some(TextRun {
                        text: text.clone(),
                        font_size: *font_size,
                    }),
                );
                z_order += 1;
            }
            DisplayCommand::Image {
                image_id,
                x,
                y,
                width,
                height,
                link_url,
            } => {
                let rect = Rect {
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                };
                let seg = assign_segment(*y, segment_height);
                scene.push(
                    SceneNode {
                        rect,
                        kind: SceneNodeKind::Image,
                        parent: parent_id,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                        state: NodeState::Normal,
                        link_url: link_url.clone(),
                        clip: active_clip,
                    },
                    [1.0, 1.0, 1.0, 1.0], // White placeholder (texture replaces this)
                    Some(TextRun {
                        text: image_id.clone(),
                        font_size: 0.0,
                    }),
                );
                z_order += 1;
            }
        }
    }

    scene
}

/// Compute bounding rect for text with line breaks and minimum 1 line height.
fn compute_text_rect(text: &str, x: f32, y: f32, width: f32, line_height: f32) -> Rect {
    let line_count = text.lines().count().max(1) as f32;
    Rect {
        x,
        y,
        width,
        height: line_height * line_count,
    }
}

/// Intersects two rectangles.
fn intersect_rect(a: &Rect, b: &Rect) -> Rect {
    let x1 = a.x.max(b.x);
    let y1 = a.y.max(b.y);
    let x2 = (a.x + a.width).min(b.x + b.width);
    let y2 = (a.y + a.height).min(b.y + b.height);
    Rect {
        x: x1,
        y: y1,
        width: (x2 - x1).max(0.0),
        height: (y2 - y1).max(0.0),
    }
}

/// Extract the bounding rect from a DisplayCommand.
fn cmd_bounding_rect(cmd: &DisplayCommand) -> Rect {
    match cmd {
        DisplayCommand::SolidColor { rect, .. } => *rect,
        DisplayCommand::RoundedRect { rect, .. } => *rect,
        DisplayCommand::Border { rect, .. } => *rect,
        DisplayCommand::BoxShadow { rect, shadow, .. } => {
            let blur = shadow.blur_radius;
            Rect {
                x: rect.x - blur,
                y: rect.y - blur,
                width: rect.width + 2.0 * blur,
                height: rect.height + 2.0 * blur,
            }
        }
        DisplayCommand::PushClip { rect } => *rect,
        DisplayCommand::PopClip => Rect::default(),
        DisplayCommand::Text {
            text,
            x,
            y,
            target_width,
            line_height,
            ..
        } => compute_text_rect(text, *x, *y, *target_width, *line_height),
        DisplayCommand::Image {
            x,
            y,
            width,
            height,
            ..
        } => Rect {
            x: *x,
            y: *y,
            width: *width,
            height: *height,
        },
    }
}

/// Check if `outer` fully contains `inner` (with a small epsilon tolerance).
fn rect_contains(outer: &Rect, inner: &Rect) -> bool {
    const EPS: f32 = 0.5;
    inner.x >= outer.x - EPS
        && inner.y >= outer.y - EPS
        && inner.x + inner.width <= outer.x + outer.width + EPS
        && inner.y + inner.height <= outer.y + outer.height + EPS
}

// ─── Scene Graph Inspector ───────────────────────────────────────

impl std::fmt::Display for SceneGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "── Scene Graph ({} nodes) ─────────────────",
            self.nodes.len()
        )?;
        for (i, node) in self.nodes.iter().enumerate() {
            let color = self.colors[i];
            let dirty_marker = if node.dirty { " [DIRTY]" } else { "" };
            write!(
                f,
                "  [{:>3}] z={:<3} seg={:<2} {:?} rect=({:.1}, {:.1}, {:.1}, {:.1}) rgba=({:.2},{:.2},{:.2},{:.2}){}",
                i,
                node.z_order,
                node.segment_id,
                node.kind,
                node.rect.x,
                node.rect.y,
                node.rect.width,
                node.rect.height,
                color[0],
                color[1],
                color[2],
                color[3],
                dirty_marker,
            )?;
            if let Some(text_run) = &self.texts[i]
                && !text_run.text.is_empty()
                && text_run.font_size > 0.0
            {
                write!(f, " \"{}\"", text_run.text)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
