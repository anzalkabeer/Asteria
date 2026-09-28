use crate::layout::Rect;
use crate::renderer::commands::batch_builder::BatchBuilder;
use crate::renderer::commands::command_builder::RenderCommand;
use crate::scene::{SceneGraph, SceneNodeId, SceneNodeKind};
use crate::selection::TextSelection;
use crate::values::{IDENTITY_MATRIX, is_identity_matrix};

/// Represents an independent GPU rendering or compositing layer.
/// Elements with CSS transforms, opacity < 1.0 on composite subtrees,
/// fixed positioning, or distinct stacking contexts are promoted into separate layers
/// to enable efficient GPU caching, layer-level culling, and independent transformation.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositorLayer {
    /// Unique layer ID (0 is always the root content layer)
    pub id: u32,
    /// Root scene node that triggered this layer's promotion
    pub root_node_id: SceneNodeId,
    /// Bounding rectangle of this layer in document coordinates
    pub bounds: Rect,
    /// Accumulated 2D affine transformation matrix [a, b, c, d, tx, ty]
    pub transform: [f32; 6],
    /// Layer-level opacity multiplier (0.0 to 1.0)
    pub opacity: f32,
    /// Independent scroll offset (x, y)
    pub scroll_offset: (f32, f32),
    /// Indices of scene nodes assigned to this layer
    pub node_indices: Vec<usize>,
    /// Whether this layer was promoted out of normal flow
    pub is_promoted: bool,
    /// Whether this layer is fixed-positioned (immune to page scrolling)
    pub is_fixed: bool,
    /// Layer stacking context z-order
    pub z_order: u32,
    /// Optional clipping rectangle
    pub clip_rect: Option<Rect>,
}

impl CompositorLayer {
    pub fn new(id: u32, root_node_id: SceneNodeId, bounds: Rect) -> Self {
        Self {
            id,
            root_node_id,
            bounds,
            transform: IDENTITY_MATRIX,
            opacity: 1.0,
            scroll_offset: (0.0, 0.0),
            node_indices: Vec::new(),
            is_promoted: false,
            is_fixed: false,
            z_order: 0,
            clip_rect: None,
        }
    }

    /// Recompute tight bounding box encompassing all nodes in this layer.
    pub fn update_bounds(&mut self, scene: &SceneGraph) {
        if self.node_indices.is_empty() {
            return;
        }
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;

        for &idx in &self.node_indices {
            if let Some(node) = scene.nodes.get(idx) {
                min_x = min_x.min(node.rect.x);
                min_y = min_y.min(node.rect.y);
                max_x = max_x.max(node.rect.x + node.rect.width);
                max_y = max_y.max(node.rect.y + node.rect.height);
            }
        }

        if min_x <= max_x && min_y <= max_y {
            self.bounds = Rect {
                x: min_x,
                y: min_y,
                width: max_x - min_x,
                height: max_y - min_y,
            };
        }
    }

    /// Returns true if this layer is completely outside the visible viewport (layer-level culling).
    pub fn is_culled(&self, scroll_y: f32, top_clip: f32, bottom_clip: f32) -> bool {
        // Transformed layers skip simple AABB culling
        if !is_identity_matrix(&self.transform) {
            return false;
        }
        if self.bounds.width <= 0.0 || self.bounds.height <= 0.0 {
            return false;
        }
        let effective_y = if self.is_fixed {
            self.bounds.y + top_clip
        } else {
            self.bounds.y + top_clip - scroll_y
        };
        let effective_bottom = effective_y + self.bounds.height;
        effective_bottom <= top_clip || effective_y >= bottom_clip
    }

    /// Build render commands for this layer, applying scroll, affine transformation, and layer opacity.
    pub fn build_commands(
        &self,
        scene: &SceneGraph,
        scroll_y: f32,
        top_clip: f32,
        bottom_clip: f32,
    ) -> Vec<RenderCommand> {
        let mut commands = Vec::new();
        let scroll = if self.is_fixed { 0.0 } else { scroll_y };

        for &idx in &self.node_indices {
            let Some(node) = scene.nodes.get(idx) else {
                continue;
            };
            let mut color = scene
                .colors
                .get(idx)
                .copied()
                .unwrap_or([0.0, 0.0, 0.0, 1.0]);

            // Apply layer opacity multiplier
            if self.opacity < 0.999 {
                color[3] *= self.opacity;
            }

            if matches!(node.kind, SceneNodeKind::SolidRect) {
                let mut transform = self.transform;
                if is_identity_matrix(&transform) {
                    let mut r = [
                        node.rect.x,
                        node.rect.y + top_clip - scroll,
                        node.rect.width,
                        node.rect.height,
                    ];
                    let r_top = r[1];
                    let r_bottom = r[1] + r[3];
                    if r_bottom <= top_clip || r_top >= bottom_clip {
                        continue;
                    }
                    let vis_top = r_top.max(top_clip);
                    let vis_bottom = r_bottom.min(bottom_clip);
                    r[1] = vis_top;
                    r[3] = (vis_bottom - vis_top).max(0.0);
                    commands.push(RenderCommand::SolidRect {
                        rect: r,
                        rgba: color,
                        transform,
                    });
                } else {
                    // Offset layer translation by top_clip and scroll
                    transform[5] += top_clip - scroll;
                    commands.push(RenderCommand::SolidRect {
                        rect: [node.rect.x, node.rect.y, node.rect.width, node.rect.height],
                        rgba: color,
                        transform,
                    });
                }
            }
        }
        commands
    }
}

/// The Compositor manages layer assignment, promotion logic, stacking context ordering,
/// and high-performance GPU compositing.
#[derive(Debug, Clone, Default)]
pub struct Compositor {
    /// Ordered list of layers (bottom-to-top rendering order)
    pub layers: Vec<CompositorLayer>,
}

impl Compositor {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Build a layer tree from a SceneGraph by identifying promotion candidates.
    pub fn build_layers(&mut self, scene: &SceneGraph) {
        self.layers.clear();

        let mut root_layer = CompositorLayer::new(0, SceneNodeId(0), Rect::default());
        root_layer.is_promoted = false;
        root_layer.z_order = 0;

        let mut next_layer_id = 1;
        let mut promoted_layers = Vec::new();

        for (i, node) in scene.nodes.iter().enumerate() {
            let has_transform = !is_identity_matrix(&node.transform);
            let has_stacking_order = node.z_order != 0;
            let has_opacity = scene.colors.get(i).map(|c| c[3] < 0.999).unwrap_or(false)
                && (node.rect.width > 200.0 || node.rect.height > 200.0);

            // Promote transformed elements, distinct stacking contexts, or large translucent overlays
            if has_transform || has_stacking_order || has_opacity {
                let mut layer =
                    CompositorLayer::new(next_layer_id, SceneNodeId(i as u32), node.rect);
                layer.transform = node.transform;
                layer.z_order = node.z_order;
                layer.is_promoted = true;
                layer.node_indices.push(i);
                promoted_layers.push(layer);
                next_layer_id += 1;
            } else {
                root_layer.node_indices.push(i);
            }
        }

        root_layer.update_bounds(scene);
        self.layers.push(root_layer);

        for mut layer in promoted_layers {
            layer.update_bounds(scene);
            self.layers.push(layer);
        }

        // Sort layers by stacking context z_order so compositing paints in correct CSS order
        self.layers.sort_by_key(|l| l.z_order);
    }

    /// Render all layers into a consolidated BatchBuilder, performing layer-level culling,
    /// applying layer transforms and opacities, and adding selection highlights.
    #[allow(clippy::too_many_arguments)]
    pub fn render_to_batch(
        &self,
        scene: &SceneGraph,
        scroll_y: f32,
        vp_w: f32,
        _vp_h: f32,
        top_clip: f32,
        bottom_clip: f32,
        selection: &TextSelection,
    ) -> BatchBuilder {
        let mut batch = BatchBuilder::new();

        // 1. Canvas background fill
        let canvas_bg = scene
            .nodes
            .iter()
            .enumerate()
            .find_map(|(i, n)| {
                if matches!(n.kind, SceneNodeKind::SolidRect)
                    && n.rect.x <= 0.0
                    && n.rect.y <= 0.0
                    && n.rect.width > 50.0
                {
                    Some(scene.colors[i])
                } else {
                    None
                }
            })
            .unwrap_or([0.118, 0.118, 0.180, 1.0]);

        batch.add_quad_direct(
            0.0,
            top_clip,
            vp_w,
            (bottom_clip - top_clip).max(0.0),
            canvas_bg,
        );

        // 2. Render each layer in sorted stacking order (culling offscreen layers)
        for layer in &self.layers {
            if layer.is_culled(scroll_y, top_clip, bottom_clip) {
                continue;
            }
            let cmds = layer.build_commands(scene, scroll_y, top_clip, bottom_clip);
            batch.append_batches(&cmds, vp_w);
        }

        // 3. Render text selection highlight quads behind text
        for range in &selection.ranges {
            let r = range.rect;
            let y = r.y + top_clip - scroll_y;
            if y + r.height > top_clip && y < bottom_clip {
                let vis_top = y.max(top_clip);
                let vis_bottom = (y + r.height).min(bottom_clip);
                batch.add_quad_direct(
                    r.x,
                    vis_top,
                    r.width,
                    (vis_bottom - vis_top).max(0.0),
                    crate::selection::SELECTION_HIGHLIGHT_COLOR,
                );
            }
        }

        batch
    }

    pub fn layer_count(&self) -> usize {
        self.layers.len()
    }

    pub fn promoted_layer_count(&self) -> usize {
        self.layers.iter().filter(|l| l.is_promoted).count()
    }

    pub fn promoted_layers(&self) -> impl Iterator<Item = &CompositorLayer> {
        self.layers.iter().filter(|l| l.is_promoted)
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{NodeState, SceneNode};

    #[test]
    fn test_compositor_root_layer_creation() {
        let mut scene = SceneGraph::new();
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 100.0,
                    height: 100.0,
                },
                kind: SceneNodeKind::SolidRect,
                parent: None,
                z_order: 0,
                segment_id: 0,
                dirty: false,
                state: NodeState::Normal,
                link_url: None,
                clip: None,
                transform: IDENTITY_MATRIX,
            },
            [1.0, 1.0, 1.0, 1.0],
            None,
        );

        let mut comp = Compositor::new();
        comp.build_layers(&scene);

        assert_eq!(comp.layer_count(), 1);
        assert_eq!(comp.promoted_layer_count(), 0);
        assert_eq!(comp.layers[0].node_indices, vec![0]);
        assert_eq!(comp.layers[0].bounds.width, 100.0);
    }

    #[test]
    fn test_compositor_promotes_transformed_node() {
        let mut scene = SceneGraph::new();
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 50.0,
                    y: 50.0,
                    width: 100.0,
                    height: 100.0,
                },
                kind: SceneNodeKind::SolidRect,
                parent: None,
                z_order: 1,
                segment_id: 0,
                dirty: false,
                state: NodeState::Normal,
                link_url: None,
                clip: None,
                transform: [2.0, 0.0, 0.0, 2.0, 10.0, 20.0],
            },
            [1.0, 0.0, 0.0, 1.0],
            None,
        );

        let mut comp = Compositor::new();
        comp.build_layers(&scene);

        assert_eq!(comp.layer_count(), 2);
        assert_eq!(comp.promoted_layer_count(), 1);
        let promoted = comp.promoted_layers().next().unwrap();
        assert_eq!(promoted.transform[0], 2.0);
    }

    #[test]
    fn test_compositor_layer_culling() {
        let layer = CompositorLayer {
            id: 1,
            root_node_id: SceneNodeId(0),
            bounds: Rect {
                x: 0.0,
                y: 1000.0,
                width: 200.0,
                height: 100.0,
            },
            transform: IDENTITY_MATRIX,
            opacity: 1.0,
            scroll_offset: (0.0, 0.0),
            node_indices: vec![0],
            is_promoted: true,
            is_fixed: false,
            z_order: 0,
            clip_rect: None,
        };

        // Viewport is at y=0..600, layer is at y=1000 (culled)
        assert!(layer.is_culled(0.0, 50.0, 600.0));
        // Viewport scrolled down to 800, layer is at effective y = 1000 + 50 - 800 = 250 (visible)
        assert!(!layer.is_culled(800.0, 50.0, 600.0));
    }

    #[test]
    fn test_compositor_render_to_batch() {
        let mut scene = SceneGraph::new();
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 10.0,
                    y: 10.0,
                    width: 50.0,
                    height: 50.0,
                },
                kind: SceneNodeKind::SolidRect,
                parent: None,
                z_order: 0,
                segment_id: 0,
                dirty: false,
                state: NodeState::Normal,
                link_url: None,
                clip: None,
                transform: IDENTITY_MATRIX,
            },
            [0.8, 0.2, 0.2, 1.0],
            None,
        );

        let mut comp = Compositor::new();
        comp.build_layers(&scene);

        let selection = TextSelection::new();
        let batch = comp.render_to_batch(&scene, 0.0, 800.0, 600.0, 60.0, 560.0, &selection);
        assert!(!batch.is_empty());
    }
}
