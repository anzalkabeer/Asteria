use crate::css_parser::Stylesheet;
use crate::layout::layout_document;
use crate::paint::{build_display_list, DisplayList};
use crate::scene::{build_scene_graph, SceneGraph};
use crate::shell::TabManager;
use crate::style::resolve_styles_with_viewport;

/// Sub-Container Web Engine Viewport
///
/// Encapsulates the 7-stage Asteria Web Engine pipeline, rendering web page content
/// strictly within the allocated viewport canvas bounds (y = 84px to y = height - 32px).
pub struct EngineViewport {
    pub scroll_y: f32,
    pub target_scroll_y: f32,
}

impl Default for EngineViewport {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineViewport {
    pub fn new() -> Self {
        Self {
            scroll_y: 0.0,
            target_scroll_y: 0.0,
        }
    }

    /// Smooth scrolling lerp update.
    /// Returns `true` if viewport position changed and requires a redraw.
    pub fn update_scroll(&mut self) -> bool {
        let diff = self.target_scroll_y - self.scroll_y;
        if diff.abs() > 0.5 {
            self.scroll_y += diff * 0.15;
            true
        } else if diff.abs() > 0.0 {
            self.scroll_y = self.target_scroll_y;
            true
        } else {
            false
        }
    }

    /// Build web page scene graph for central viewport (y = 84px to y = height - 32px)
    pub fn build_page_scene(
        &self,
        viewport_w: f32,
        viewport_h: f32,
        tab_manager: &TabManager,
    ) -> SceneGraph {
        let sample_html_bytes = b"<!DOCTYPE html><html><head><style>body { background-color: #0f172a; color: #f8fafc; font-family: sans-serif; } h1 { color: #3b82f6; font-size: 24px; } p { color: #94a3b8; font-size: 16px; } div { background-color: #1e293b; padding: 16px; margin-top: 16px; border: 1px solid #334155; }</style></head><body><h1>Asteria Browser Engine</h1><p>Hardware-accelerated GPU renderer running with Laboratory HUD design system.</p><div><p>Interactive Viewport: Scroll, Hover, Click & Tab Switching Supported!</p></div></body></html>";

        let active_tab = tab_manager.active_tab();
        let html_bytes = active_tab
            .page_resources
            .as_ref()
            .map(|r| r.html.bytes.as_slice())
            .unwrap_or(sample_html_bytes);

        let mut processor = crate::streaming_parser::StreamingHtmlProcessor::new();
        let _ = processor.receive_network_chunk(html_bytes, true);
        let dom = processor.finish();

        let css_bytes = active_tab
            .page_resources
            .as_ref()
            .and_then(|r| r.stylesheets.first())
            .map(|c| c.bytes.as_slice())
            .unwrap_or(b"");

        let stylesheet = Stylesheet::parse(css_bytes);
        let content_h = (viewport_h - 84.0 - 32.0).max(100.0);

        let styled = resolve_styles_with_viewport(&dom, &stylesheet, html_bytes, viewport_w);

        if let Some(layout) = layout_document(&styled, &dom, html_bytes, viewport_w, content_h) {
            let display_list = build_display_list(&layout, &dom, html_bytes);
            let mut scene = build_scene_graph(&display_list, 256.0);

            // Shift page content below top header bar (y += 84.0)
            for node in &mut scene.nodes {
                node.rect.y += 84.0;
            }
            return scene;
        }

        let empty_dl = DisplayList::new();
        build_scene_graph(&empty_dl, 256.0)
    }
}
