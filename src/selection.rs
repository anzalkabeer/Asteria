use crate::layout::Rect;
use crate::scene::{SceneGraph, SceneNodeId, SceneNodeKind};

/// Highlight color used across the browser for selected text: semi-transparent browser blue
pub const SELECTION_HIGHLIGHT_COLOR: [f32; 4] = [0.26, 0.52, 0.96, 0.35];

/// An active or completed text selection in document space.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextSelection {
    /// Document (x, y) coordinates where mouse button was pressed down
    pub anchor: Option<(f32, f32)>,
    /// Current mouse position in document (x, y) coordinates during drag
    pub extent: Option<(f32, f32)>,
    /// Whether user is currently in the middle of a click-and-drag selection
    pub is_active: bool,
    /// Resolved selection ranges per text node
    pub ranges: Vec<SelectionRange>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectionRange {
    pub node_id: SceneNodeId,
    pub char_start: usize,
    pub char_end: usize,
    /// Visual highlight rectangle in document coordinates
    pub rect: Rect,
}

impl TextSelection {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new selection at document coordinates (x, y).
    pub fn start(&mut self, x: f32, y: f32) {
        self.anchor = Some((x, y));
        self.extent = Some((x, y));
        self.is_active = true;
        self.ranges.clear();
    }

    /// Update selection extent during mouse drag and resolve selection ranges across the scene.
    pub fn update(&mut self, x: f32, y: f32, scene: &SceneGraph) {
        self.extent = Some((x, y));
        if self.anchor.is_some() {
            self.resolve_ranges(scene);
        }
    }

    /// Complete the selection on mouse release.
    pub fn finish(&mut self, scene: &SceneGraph) {
        self.is_active = false;
        if self.anchor.is_some() && self.extent.is_some() {
            self.resolve_ranges(scene);
        }
    }

    /// Clear the selection.
    pub fn clear(&mut self) {
        self.anchor = None;
        self.extent = None;
        self.is_active = false;
        self.ranges.clear();
    }

    /// Check if there is an active non-empty text selection.
    pub fn has_selection(&self) -> bool {
        !self.ranges.is_empty()
    }

    /// Select all text within the entire scene graph.
    pub fn select_all(&mut self, scene: &SceneGraph) {
        self.clear();
        for (i, node) in scene.nodes.iter().enumerate() {
            if let (SceneNodeKind::Text { .. }, Some(Some(text_run))) =
                (&node.kind, scene.texts.get(i))
            {
                let char_count = text_run.text.chars().count();
                if char_count > 0 {
                    self.ranges.push(SelectionRange {
                        node_id: SceneNodeId(i as u32),
                        char_start: 0,
                        char_end: char_count,
                        rect: node.rect,
                    });
                }
            }
        }
    }

    /// Select the word under cursor at document coordinates (x, y).
    pub fn select_word_at(&mut self, x: f32, y: f32, scene: &SceneGraph) {
        self.clear();
        for (i, node) in scene.nodes.iter().enumerate() {
            let nr = node.rect;
            if x < nr.x || x > nr.x + nr.width || y < nr.y || y > nr.y + nr.height {
                continue;
            }
            let SceneNodeKind::Text { font_size } = node.kind else {
                continue;
            };
            let Some(Some(text_run)) = scene.texts.get(i) else {
                continue;
            };

            let chars: Vec<char> = text_run.text.chars().collect();
            if chars.is_empty() {
                continue;
            }
            let avg_char_w = (nr.width / chars.len() as f32).max(font_size * 0.45);
            let mut char_idx =
                (((x - nr.x) / avg_char_w).floor() as usize).min(chars.len().saturating_sub(1));

            if chars[char_idx].is_whitespace() {
                if char_idx + 1 < chars.len() && !chars[char_idx + 1].is_whitespace() {
                    char_idx += 1;
                } else if char_idx > 0 && !chars[char_idx - 1].is_whitespace() {
                    char_idx -= 1;
                }
            }

            // Expand outwards to find word boundaries
            let mut start = char_idx;
            while start > 0
                && !chars[start - 1].is_whitespace()
                && !chars[start - 1].is_ascii_punctuation()
            {
                start -= 1;
            }
            let mut end = char_idx;
            while end < chars.len()
                && !chars[end].is_whitespace()
                && !chars[end].is_ascii_punctuation()
            {
                end += 1;
            }

            if start < end {
                let hl_x = nr.x + (start as f32) * avg_char_w;
                let hl_w = ((end - start) as f32) * avg_char_w;
                self.ranges.push(SelectionRange {
                    node_id: SceneNodeId(i as u32),
                    char_start: start,
                    char_end: end,
                    rect: Rect {
                        x: hl_x,
                        y: nr.y,
                        width: hl_w.min((nr.width - (hl_x - nr.x)).max(avg_char_w)),
                        height: nr.height.max(font_size * 1.2),
                    },
                });
                return;
            }
        }
    }

    /// Resolve selection ranges by intersecting document selection coordinates with text scene nodes.
    pub fn resolve_ranges(&mut self, scene: &SceneGraph) {
        self.ranges.clear();
        let (Some((ax, ay)), Some((ex, ey))) = (self.anchor, self.extent) else {
            return;
        };

        // If click without drag (distance squared < 9.0px), don't select anything
        let dist_sq = (ax - ex) * (ax - ex) + (ay - ey) * (ay - ey);
        if dist_sq < 9.0 {
            return;
        }

        // Forward or backward selection
        let is_forward = if (ay - ey).abs() > 4.0 {
            ay < ey
        } else {
            ax <= ex
        };

        let (start_pt, end_pt) = if is_forward {
            ((ax, ay), (ex, ey))
        } else {
            ((ex, ey), (ax, ay))
        };

        for (i, node) in scene.nodes.iter().enumerate() {
            if let SceneNodeKind::Text { font_size } = node.kind {
                let Some(text_run) = &scene.texts[i] else {
                    continue;
                };
                let text = &text_run.text;
                if text.is_empty() {
                    continue;
                }

                let nr = node.rect;
                let node_bottom = nr.y + nr.height;

                // Check vertical overlap with text line
                if node_bottom < start_pt.1 || nr.y > end_pt.1 {
                    continue;
                }

                let char_count = text.chars().count();
                if char_count == 0 {
                    continue;
                }
                let avg_char_w = (nr.width / char_count as f32).max(font_size * 0.45);

                let is_first_line = start_pt.1 >= nr.y && start_pt.1 <= node_bottom;
                let is_last_line = end_pt.1 >= nr.y && end_pt.1 <= node_bottom;

                let char_start = if is_first_line {
                    let rel_x = (start_pt.0 - nr.x).max(0.0);
                    ((rel_x / avg_char_w).floor() as usize).min(char_count)
                } else {
                    0
                };

                let char_end = if is_last_line {
                    let rel_x = (end_pt.0 - nr.x).max(0.0);
                    ((rel_x / avg_char_w).ceil() as usize).min(char_count)
                } else {
                    char_count
                };

                if char_start < char_end {
                    let hl_x = nr.x + (char_start as f32) * avg_char_w;
                    let hl_w = ((char_end - char_start) as f32) * avg_char_w;
                    let highlight_rect = Rect {
                        x: hl_x,
                        y: nr.y,
                        width: hl_w.min((nr.width - (hl_x - nr.x)).max(avg_char_w)),
                        height: nr.height.max(font_size * 1.2),
                    };

                    self.ranges.push(SelectionRange {
                        node_id: SceneNodeId(i as u32),
                        char_start,
                        char_end,
                        rect: highlight_rect,
                    });
                }
            }
        }
    }

    /// Extract the currently selected text string across all ranges.
    pub fn get_selected_text(&self, scene: &SceneGraph) -> String {
        let mut result = Vec::new();
        for range in &self.ranges {
            let idx = range.node_id.index();
            if let Some(Some(text_run)) = scene.texts.get(idx) {
                let chars: Vec<char> = text_run.text.chars().collect();
                if range.char_start < chars.len() {
                    let end = range.char_end.min(chars.len());
                    let sub: String = chars[range.char_start..end].iter().collect();
                    if !sub.is_empty() {
                        result.push(sub);
                    }
                }
            }
        }
        result.join("\n")
    }

    /// Copy selected text to the system clipboard.
    pub fn copy_to_clipboard(&self, scene: &SceneGraph) -> Result<String, String> {
        let text = self.get_selected_text(scene);
        if text.is_empty() {
            return Err("No text selected".to_string());
        }

        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(&text);
            Ok(text)
        } else {
            Ok(text)
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{NodeState, SceneNode, TextRun};

    #[test]
    fn test_empty_selection() {
        let sel = TextSelection::new();
        assert!(!sel.has_selection());
        assert_eq!(sel.get_selected_text(&SceneGraph::new()), "");
    }

    #[test]
    fn test_drag_selection_and_text_extraction() {
        let mut scene = SceneGraph::new();
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 10.0,
                    y: 20.0,
                    width: 100.0,
                    height: 20.0,
                },
                kind: SceneNodeKind::Text { font_size: 16.0 },
                parent: None,
                z_order: 0,
                segment_id: 0,
                dirty: false,
                state: NodeState::Normal,
                link_url: None,
                clip: None,
                transform: crate::values::IDENTITY_MATRIX,
            },
            [0.0, 0.0, 0.0, 1.0],
            Some(TextRun {
                text: "Hello World".to_string(),
                font_size: 16.0,
                font_family: vec!["sans-serif".to_string()],
                font_weight: 400.0,
            }),
        );

        let mut sel = TextSelection::new();
        // Drag across the text node
        sel.start(10.0, 25.0);
        sel.update(60.0, 25.0, &scene);
        sel.finish(&scene);

        assert!(sel.has_selection());
        let selected = sel.get_selected_text(&scene);
        assert!(!selected.is_empty());
        assert!(selected.starts_with("Hell"));

        // Copy test
        let copied = sel.copy_to_clipboard(&scene);
        assert!(copied.is_ok());
    }

    #[test]
    fn test_clear_selection() {
        let mut sel = TextSelection::new();
        sel.start(0.0, 0.0);
        sel.update(100.0, 100.0, &SceneGraph::new());
        assert!(sel.anchor.is_some());
        sel.clear();
        assert!(sel.anchor.is_none());
        assert!(!sel.has_selection());
    }

    #[test]
    fn test_select_all() {
        let mut scene = SceneGraph::new();
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 80.0,
                    height: 20.0,
                },
                kind: SceneNodeKind::Text { font_size: 16.0 },
                parent: None,
                z_order: 0,
                segment_id: 0,
                dirty: false,
                state: NodeState::Normal,
                link_url: None,
                clip: None,
                transform: crate::values::IDENTITY_MATRIX,
            },
            [0.0, 0.0, 0.0, 1.0],
            Some(TextRun {
                text: "First".to_string(),
                font_size: 16.0,
                font_family: vec!["sans-serif".to_string()],
                font_weight: 400.0,
            }),
        );
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 0.0,
                    y: 25.0,
                    width: 80.0,
                    height: 20.0,
                },
                kind: SceneNodeKind::Text { font_size: 16.0 },
                parent: None,
                z_order: 0,
                segment_id: 0,
                dirty: false,
                state: NodeState::Normal,
                link_url: None,
                clip: None,
                transform: crate::values::IDENTITY_MATRIX,
            },
            [0.0, 0.0, 0.0, 1.0],
            Some(TextRun {
                text: "Second".to_string(),
                font_size: 16.0,
                font_family: vec!["sans-serif".to_string()],
                font_weight: 400.0,
            }),
        );

        let mut sel = TextSelection::new();
        sel.select_all(&scene);
        assert_eq!(sel.ranges.len(), 2);
        assert_eq!(sel.get_selected_text(&scene), "First\nSecond");
    }

    #[test]
    fn test_select_word_at() {
        let mut scene = SceneGraph::new();
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 10.0,
                    y: 10.0,
                    width: 200.0,
                    height: 20.0,
                },
                kind: SceneNodeKind::Text { font_size: 16.0 },
                parent: None,
                z_order: 0,
                segment_id: 0,
                dirty: false,
                state: NodeState::Normal,
                link_url: None,
                clip: None,
                transform: crate::values::IDENTITY_MATRIX,
            },
            [0.0, 0.0, 0.0, 1.0],
            Some(TextRun {
                text: "Asteria Engine".to_string(),
                font_size: 16.0,
                font_family: vec!["sans-serif".to_string()],
                font_weight: 400.0,
            }),
        );

        let mut sel = TextSelection::new();
        // Click directly on "Engine" (x ~ 150)
        sel.select_word_at(150.0, 15.0, &scene);
        assert!(sel.has_selection());
        assert_eq!(sel.get_selected_text(&scene), "Engine");
    }

    #[test]
    fn test_backward_drag_selection() {
        let mut scene = SceneGraph::new();
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 10.0,
                    y: 10.0,
                    width: 100.0,
                    height: 20.0,
                },
                kind: SceneNodeKind::Text { font_size: 16.0 },
                parent: None,
                z_order: 0,
                segment_id: 0,
                dirty: false,
                state: NodeState::Normal,
                link_url: None,
                clip: None,
                transform: crate::values::IDENTITY_MATRIX,
            },
            [0.0, 0.0, 0.0, 1.0],
            Some(TextRun {
                text: "Hello World".to_string(),
                font_size: 16.0,
                font_family: vec!["sans-serif".to_string()],
                font_weight: 400.0,
            }),
        );

        let mut sel = TextSelection::new();
        // Drag backwards from right to left
        sel.start(90.0, 15.0);
        sel.update(20.0, 15.0, &scene);
        sel.finish(&scene);

        assert!(sel.has_selection());
        let text = sel.get_selected_text(&scene);
        assert!(!text.is_empty());
    }
}
