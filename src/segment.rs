// ─── ASTERIA Viewport Segment Builder ────────────────────────────
//
// Divides the viewport into horizontal segments (tiles) for
// region-based rendering (Pillar 4). Only dirty segments are
// re-rendered each frame — clean segments reuse their cached output.
//
// ASTERIA is a data-oriented, segment-based, GPU-first browser engine.
// Segment rendering is core to minimizing unnecessary GPU work.
//
// Example with 256px segment height on a 1024px viewport:
//
//   Viewport (800 x 1024)
//   ────────────────────
//     Segment 0 (y: 0..256)      → Clean → Reuse
//     Segment 1 (y: 256..512)    → DIRTY → Re-render
//     Segment 2 (y: 512..768)    → Clean → Reuse
//     Segment 3 (y: 768..1024)   → Clean → Reuse
//   ────────────────────
//   Only Segment 1 costs GPU cycles this frame.

use crate::layout::Rect;

// ─── Viewport Segment ────────────────────────────────────────────

/// A single horizontal tile of the viewport.
///
/// When a segment is clean, its previously rendered output can be
/// composited directly without re-running the paint pipeline for
/// the nodes it contains.
#[derive(Debug, Clone)]
pub struct ViewportSegment {
    /// Unique segment identifier (sequential from top to bottom)
    pub id: u16,
    /// Screen-space bounding rectangle of this segment
    pub rect: Rect,
    /// Whether this segment needs re-rendering this frame
    pub dirty: bool,
    /// Generation counter — incremented each time segment is re-rendered.
    /// Used by GPU texture cache to detect stale cached tiles.
    pub generation: u64,
}

// ─── Segment Builder ─────────────────────────────────────────────

/// Builds and manages viewport segments for region-based rendering.
///
/// The builder divides the viewport into fixed-height horizontal strips.
/// Each segment tracks its own dirty state. When scene nodes change,
/// only the segments containing those nodes are marked dirty.
pub struct SegmentBuilder {
    /// All viewport segments (ordered top to bottom)
    pub segments: Vec<ViewportSegment>,
    /// Height of each segment in pixels (e.g., 256.0)
    pub segment_height: f32,
    /// Current viewport dimensions
    pub viewport_width: f32,
    pub viewport_height: f32,
}

impl SegmentBuilder {
    /// Create a new segment builder with the given tile height
    pub fn new(segment_height: f32) -> Self {
        SegmentBuilder {
            segments: Vec::new(),
            segment_height: segment_height.max(1.0), // Prevent zero-height segments
            viewport_width: 0.0,
            viewport_height: 0.0,
        }
    }

    /// Divide the viewport into horizontal segments.
    /// Called on window creation and on every resize event.
    pub fn build_segments(&mut self, viewport_width: f32, viewport_height: f32) -> Result<(), String> {
        if !viewport_width.is_finite() || !viewport_height.is_finite() {
            return Err("Invalid viewport dimensions".into());
        }

        let previous_generations: Vec<u64> = self.segments.iter().map(|s| s.generation).collect();
        self.segments.clear();
        self.viewport_width = viewport_width;
        self.viewport_height = viewport_height;

        let mut y = 0.0;
        let mut id = 0u16;

        while y < viewport_height {
            let h = self.segment_height.min(viewport_height - y);
            if id == u16::MAX && y + h < viewport_height {
                return Err("Viewport height exceeds maximum segment count".to_string());
            }

            let generation = previous_generations
                .get(id as usize)
                .copied()
                .unwrap_or(0)
                .saturating_add(1);

            self.segments.push(ViewportSegment {
                id,
                rect: Rect {
                    x: 0.0,
                    y,
                    width: viewport_width,
                    height: h,
                },
                dirty: true, // First frame: everything is dirty
                generation,
            });

            y += h;
            if y < viewport_height {
                id = id
                    .checked_add(1)
                    .ok_or_else(|| String::from("Viewport height exceeds maximum segment count"))?;
            }
        }

        Ok(())
    }

    /// Mark all segments as dirty (used after full re-layout, e.g., window resize)
    pub fn invalidate_all(&mut self) {
        for seg in &mut self.segments {
            seg.dirty = true;
        }
    }

    /// Mark a specific segment as dirty by ID
    pub fn invalidate_segment(&mut self, segment_id: u16) {
        if let Some(seg) = self.segments.iter_mut().find(|s| s.id == segment_id) {
            seg.dirty = true;
        }
    }

    /// Mark all segments that intersect a given rectangle as dirty.
    /// Used when a scene node changes — only its overlapping segments
    /// need re-rendering.
    pub fn invalidate_rect(&mut self, rect: &Rect) {
        for seg in &mut self.segments {
            if rects_intersect(&seg.rect, rect) {
                seg.dirty = true;
            }
        }
    }

    /// Mark a segment as clean after successful re-render and bump generation
    pub fn mark_clean(&mut self, segment_id: u16) {
        if let Some(seg) = self.segments.iter_mut().find(|s| s.id == segment_id) {
            seg.dirty = false;
            seg.generation += 1;
        }
    }

    /// Get all dirty segment IDs (these need GPU re-rendering this frame)
    pub fn dirty_segments(&self) -> Vec<u16> {
        self.segments
            .iter()
            .filter(|s| s.dirty)
            .map(|s| s.id)
            .collect()
    }

    /// Get all clean segment IDs (these can reuse cached GPU textures)
    pub fn clean_segments(&self) -> Vec<u16> {
        self.segments
            .iter()
            .filter(|s| !s.dirty)
            .map(|s| s.id)
            .collect()
    }

    /// Total number of segments
    pub fn len(&self) -> usize {
        self.segments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Determine which segment ID a y-coordinate falls into
    pub fn segment_for_y(&self, y: f32) -> Option<u16> {
        if self.segment_height <= 0.0 || !y.is_finite() || y < 0.0 || y >= self.viewport_height {
            return None;
        }
        let id = (y / self.segment_height) as u16;
        if (id as usize) < self.segments.len() {
            Some(id)
        } else {
            None
        }
    }
}

impl Default for SegmentBuilder {
    fn default() -> Self {
        Self::new(256.0) // Default 256px segment height
    }
}

// ─── Geometry Helpers ────────────────────────────────────────────

/// Check if two rectangles intersect (shared by segment and image modules)
pub fn rects_intersect(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.width
        && a.x + a.width > b.x
        && a.y < b.y + b.height
        && a.y + a.height > b.y
}

// ─── Display ─────────────────────────────────────────────────────

use std::fmt;

impl fmt::Display for SegmentBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "── Viewport Segments ({} tiles, {}px each) ─────────────────",
            self.segments.len(),
            self.segment_height,
        )?;
        for seg in &self.segments {
            let status = if seg.dirty { "DIRTY" } else { "CLEAN" };
            writeln!(
                f,
                "  [Seg {:>2}] y={:.0}..{:.0}  gen={}  {}",
                seg.id,
                seg.rect.y,
                seg.rect.y + seg.rect.height,
                seg.generation,
                status,
            )?;
        }
        Ok(())
    }
}
