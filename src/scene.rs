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

/// A single visual primitive in the scene, stored contiguously in Vec<SceneNode>
///
/// Layout:  [0][1][2][3][4][5]...
/// All nodes live in one contiguous memory block for maximum cache locality.
#[derive(Debug, Clone, Copy)]
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
}

// ─── Text Run Data ───────────────────────────────────────────────

/// Text content associated with a Text scene node
#[derive(Debug, Clone)]
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
    pub fn push(
        &mut self,
        node: SceneNode,
        color: [f32; 4],
        text: Option<TextRun>,
    ) -> SceneNodeId {
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

    /// Mark a node as dirty and propagate dirtiness up to available ancestors.
    ///
    /// DisplayList-derived scene graphs currently do not populate parent links,
    /// so ancestor propagation only occurs when `SceneNode::parent` is set.
    pub fn invalidate(&mut self, node_id: SceneNodeId) {
        let idx = node_id.index();
        if idx >= self.nodes.len() {
            return;
        }
        self.nodes[idx].dirty = true;

        // Walk up to root marking ancestors dirty when parent references are available.
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
    [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0]
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
pub fn build_scene_graph(
    display_list: &DisplayList,
    segment_height: f32,
) -> SceneGraph {
    let mut scene = SceneGraph::with_capacity(display_list.commands.len());
    let mut z_order: u32 = 0;

    for cmd in &display_list.commands {
        match cmd {
            DisplayCommand::SolidColor { color, rect } => {
                let seg = assign_segment(rect.y, segment_height);
                scene.push(
                    SceneNode {
                        rect: *rect,
                        kind: SceneNodeKind::SolidRect,
                        parent: None,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                    },
                    color_to_rgba(color),
                    None,
                );
                z_order += 1;
            }
            DisplayCommand::Border { color, rect, border_width } => {
                let seg = assign_segment(rect.y, segment_height);
                scene.push(
                    SceneNode {
                        rect: *rect,
                        kind: SceneNodeKind::Border { widths: *border_width },
                        parent: None,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                    },
                    color_to_rgba(color),
                    None,
                );
                z_order += 1;
            }
            DisplayCommand::Text { text, x, y, font_size, color } => {
                let text_width = text.chars().count() as f32 * font_size * 0.55;
                let rect = Rect {
                    x: *x,
                    y: *y,
                    width: text_width,
                    height: *font_size * 1.2,
                };
                let seg = assign_segment(*y, segment_height);
                scene.push(
                    SceneNode {
                        rect,
                        kind: SceneNodeKind::Text { font_size: *font_size },
                        parent: None,
                        z_order,
                        segment_id: seg,
                        dirty: true,
                    },
                    color_to_rgba(color),
                    Some(TextRun {
                        text: text.clone(),
                        font_size: *font_size,
                    }),
                );
                z_order += 1;
            }
            DisplayCommand::Image { image_id, x, y, width, height } => {
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

use std::fmt;

impl fmt::Display for SceneGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "── Scene Graph ({} nodes) ─────────────────", self.nodes.len())?;
        for (i, node) in self.nodes.iter().enumerate() {
            let color = self.colors[i];
            let dirty_marker = if node.dirty { " [DIRTY]" } else { "" };
            write!(
                f,
                "  [{:>3}] z={:<3} seg={:<2} {:?} rect=({:.1}, {:.1}, {:.1}, {:.1}) rgba=({:.2},{:.2},{:.2},{:.2}){}",
                i, node.z_order, node.segment_id, node.kind,
                node.rect.x, node.rect.y, node.rect.width, node.rect.height,
                color[0], color[1], color[2], color[3],
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
