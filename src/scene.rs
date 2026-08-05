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
    //this state is from the NOdestate
    pub state: NodeState,
    //interactive visuall state of a oarticular node on ehihc the mous e is hovering
    pub link_url: Option<String>, //target url of the anchor tag if the node is a link, otherwise None
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
pub fn build_scene_graph(display_list: &DisplayList, segment_height: f32) -> SceneGraph {
    let mut scene = SceneGraph::with_capacity(display_list.commands.len());
    let mut z_order: u32 = 0;

    for cmd in &display_list.commands {
        match cmd {
            DisplayCommand::SolidColor {
                color,
                rect,
                link_url,
            } => {
                let seg = assign_segment(rect.y, segment_height);
                scene.push(
                    SceneNode {
                        rect: *rect,
                        kind: SceneNodeKind::SolidRect,
                        parent: None,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                        state: NodeState::Normal,
                        link_url: link_url.clone(),
                    },
                    color_to_rgba(color),
                    None,
                );
                z_order += 1;
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
                        parent: None,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                        state: NodeState::Normal,
                        link_url: link_url.clone(),
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
                color,
                link_url,
            } => {
                let rect = Rect {
                    x: *x,
                    y: *y,
                    width: *target_width,
                    height: *font_size * 1.2,
                };
                let seg = assign_segment(*y, segment_height);
                scene.push(
                    SceneNode {
                        rect,
                        kind: SceneNodeKind::Text {
                            font_size: *font_size,
                        },
                        parent: None,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                        state: NodeState::Normal,
                        link_url: link_url.clone(),
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
                        parent: None,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                        state: NodeState::Normal,
                        link_url: link_url.clone(),
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

// ─── Scene Graph Inspector ───────────────────────────────────────

impl std::fmt::Display for SceneGraph {
    #[allow(clippy::collapsible_if)]
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
            if let Some(text_run) = &self.texts[i] {
                if !text_run.text.is_empty() && text_run.font_size > 0.0 {
                    write!(f, " \"{}\"", text_run.text)?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}
