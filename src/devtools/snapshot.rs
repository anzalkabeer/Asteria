// ─── ASTERIA Observability Framework: Snapshots ─────────────────
//
// Immutable captures of engine state at a specific point in time.
// Kept zero-overhead unless explicitly enabled.

use crate::dom::Dom;
use crate::layout::LayoutBox;
use crate::scene::SceneGraph;
use crate::segment::SegmentBuilder;
use crate::style::StyledNode;

/// Immutable capture of a subsystem state
pub struct EngineSnapshot<'a> {
    pub dom: Option<&'a Dom>,
    pub style: Option<&'a StyledNode>,
    pub layout: Option<&'a LayoutBox<'a>>,
    pub scene: Option<&'a SceneGraph>,
    pub segments: Option<&'a SegmentBuilder>,
}

impl<'a> EngineSnapshot<'a> {
    pub fn new() -> Self {
        EngineSnapshot {
            dom: None,
            style: None,
            layout: None,
            scene: None,
            segments: None,
        }
    }

    pub fn with_dom(mut self, dom: &'a Dom) -> Self {
        self.dom = Some(dom);
        self
    }

    pub fn with_style(mut self, style: &'a StyledNode) -> Self {
        self.style = Some(style);
        self
    }

    pub fn with_layout(mut self, layout: &'a LayoutBox<'a>) -> Self {
        self.layout = Some(layout);
        self
    }

    pub fn with_scene(mut self, scene: &'a SceneGraph) -> Self {
        self.scene = Some(scene);
        self
    }

    pub fn with_segments(mut self, segments: &'a SegmentBuilder) -> Self {
        self.segments = Some(segments);
        self
    }
}
