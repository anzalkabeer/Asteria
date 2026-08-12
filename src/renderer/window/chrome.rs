use crate::layout::{EdgeSizes, Rect};
use crate::scene::{NodeState, SceneGraph, SceneNode, SceneNodeKind, TextRun};
use crate::shell::TabManager;
use crate::ui_theme::{LabTheme, ThemeMode};

/// Low-Level Laboratory Shell Chrome UI Manager
pub struct ChromeShell {
    pub theme: LabTheme,
    pub is_address_bar_focused: bool,
    pub address_bar_buffer: String,
    pub last_fps: u32,
    pub last_latency_ms: u32,
}

impl Default for ChromeShell {
    fn default() -> Self {
        Self::new()
    }
}

impl ChromeShell {
    pub fn new() -> Self {
        Self {
            theme: LabTheme::dark(),
            is_address_bar_focused: false,
            address_bar_buffer: String::new(),
            last_fps: 144,
            last_latency_ms: 12,
        }
    }

    pub fn toggle_theme(&mut self) {
        self.theme = match self.theme.mode {
            ThemeMode::Dark => LabTheme::light(),
            ThemeMode::Light => LabTheme::dark(),
        };
    }

    /// Build Chrome HUD SceneGraph (Top Navigation Bar + Command Line + Status Footer)
    pub fn build_chrome_scene(
        &self,
        viewport_w: f32,
        viewport_h: f32,
        tab_manager: &TabManager,
    ) -> SceneGraph {
        let mut scene = SceneGraph::new();
        let mut z = 1000u32;

        let bg_color = self.theme.background.to_rgba_f32();
        let surface_color = self.theme.surface.to_rgba_f32();
        let border_color = self.theme.border.to_rgba_f32();
        let text_color = self.theme.text.to_rgba_f32();
        let muted_color = self.theme.muted.to_rgba_f32();
        let primary_color = self.theme.accent.to_rgba_f32();

        // 1. Top Navigation Bar Background (y = 0..44)
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: viewport_w,
                    height: 44.0,
                },
                kind: SceneNodeKind::SolidRect,
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: None,
            },
            bg_color,
            None,
        );
        z += 1;

        // Top Navigation Bar Bottom Border
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 0.0,
                    y: 43.0,
                    width: viewport_w,
                    height: 1.0,
                },
                kind: SceneNodeKind::SolidRect,
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: None,
            },
            border_color,
            None,
        );
        z += 1;

        // Brand Label ("ASTERIA // DIAGNOSTICS")
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 16.0,
                    y: 12.0,
                    width: 180.0,
                    height: 20.0,
                },
                kind: SceneNodeKind::Text { font_size: 13.0 },
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: None,
            },
            primary_color,
            Some(TextRun {
                text: "ASTERIA // DIAGNOSTICS".to_string(),
                font_size: 13.0,
            }),
        );
        z += 1;

        // Tab Strip Pills
        let mut tab_x = 210.0;
        let tab_h = 32.0;
        let active_idx = tab_manager.active_tab_index;

        for (idx, tab) in tab_manager.tabs.iter().enumerate() {
            let is_active = idx == active_idx;
            let tab_w = 120.0;

            let pill_bg = if is_active {
                surface_color
            } else {
                [bg_color[0] * 1.1, bg_color[1] * 1.1, bg_color[2] * 1.1, 1.0]
            };

            // Tab Pill Background
            scene.push(
                SceneNode {
                    rect: Rect {
                        x: tab_x,
                        y: 12.0,
                        width: tab_w,
                        height: tab_h,
                    },
                    kind: SceneNodeKind::SolidRect,
                    parent: None,
                    z_order: z,
                    segment_id: 0,
                    dirty: true,
                    state: NodeState::Normal,
                    link_url: Some(format!("asteria://tab/switch/{}", idx)),
                },
                pill_bg,
                None,
            );
            z += 1;

            // Active Tab Accent Line
            if is_active {
                scene.push(
                    SceneNode {
                        rect: Rect {
                            x: tab_x,
                            y: 12.0,
                            width: tab_w,
                            height: 2.0,
                        },
                        kind: SceneNodeKind::SolidRect,
                        parent: None,
                        z_order: z,
                        segment_id: 0,
                        dirty: true,
                        state: NodeState::Normal,
                        link_url: None,
                    },
                    primary_color,
                    None,
                );
                z += 1;
            }

            // Tab Title Text
            scene.push(
                SceneNode {
                    rect: Rect {
                        x: tab_x + 10.0,
                        y: 20.0,
                        width: tab_w - 20.0,
                        height: 16.0,
                    },
                    kind: SceneNodeKind::Text { font_size: 11.0 },
                    parent: None,
                    z_order: z,
                    segment_id: 0,
                    dirty: true,
                    state: NodeState::Normal,
                    link_url: Some(format!("asteria://tab/switch/{}", idx)),
                },
                if is_active { text_color } else { muted_color },
                Some(TextRun {
                    text: tab.title.clone(),
                    font_size: 11.0,
                }),
            );
            z += 1;

            tab_x += tab_w + 6.0;
        }

        // '+' New Tab Button
        scene.push(
            SceneNode {
                rect: Rect {
                    x: tab_x,
                    y: 14.0,
                    width: 28.0,
                    height: 28.0,
                },
                kind: SceneNodeKind::SolidRect,
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: Some("asteria://tab/new".to_string()),
            },
            surface_color,
            None,
        );
        z += 1;

        scene.push(
            SceneNode {
                rect: Rect {
                    x: tab_x + 9.0,
                    y: 20.0,
                    width: 12.0,
                    height: 14.0,
                },
                kind: SceneNodeKind::Text { font_size: 14.0 },
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: Some("asteria://tab/new".to_string()),
            },
            text_color,
            Some(TextRun {
                text: "+".to_string(),
                font_size: 14.0,
            }),
        );
        z += 1;

        // Theme Toggle Switch ("DARK" / "LIGHT")
        let theme_x = viewport_w - 180.0;
        scene.push(
            SceneNode {
                rect: Rect {
                    x: theme_x,
                    y: 12.0,
                    width: 56.0,
                    height: 24.0,
                },
                kind: SceneNodeKind::SolidRect,
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: Some("asteria://theme/toggle".to_string()),
            },
            surface_color,
            None,
        );
        z += 1;

        let theme_label = match self.theme.mode {
            ThemeMode::Dark => "DARK",
            ThemeMode::Light => "LIGHT",
        };
        scene.push(
            SceneNode {
                rect: Rect {
                    x: theme_x + 8.0,
                    y: 17.0,
                    width: 40.0,
                    height: 14.0,
                },
                kind: SceneNodeKind::Text { font_size: 10.0 },
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: Some("asteria://theme/toggle".to_string()),
            },
            primary_color,
            Some(TextRun {
                text: theme_label.to_string(),
                font_size: 10.0,
            }),
        );
        z += 1;

        // Window Control HUD (Minimize "-", Maximize "[]", Close "x")
        let win_ctrl_x = viewport_w - 110.0;
        let ctrl_labels = [
            ("-", "asteria://window/minimize"),
            ("[]", "asteria://window/maximize"),
            ("x", "asteria://window/close"),
        ];

        for (i, (lbl, action)) in ctrl_labels.iter().enumerate() {
            let cx = win_ctrl_x + (i as f32 * 32.0);
            scene.push(
                SceneNode {
                    rect: Rect {
                        x: cx,
                        y: 12.0,
                        width: 26.0,
                        height: 24.0,
                    },
                    kind: SceneNodeKind::SolidRect,
                    parent: None,
                    z_order: z,
                    segment_id: 0,
                    dirty: true,
                    state: NodeState::Normal,
                    link_url: Some(action.to_string()),
                },
                surface_color,
                None,
            );
            z += 1;

            scene.push(
                SceneNode {
                    rect: Rect {
                        x: cx + 8.0,
                        y: 17.0,
                        width: 12.0,
                        height: 14.0,
                    },
                    kind: SceneNodeKind::Text { font_size: 11.0 },
                    parent: None,
                    z_order: z,
                    segment_id: 0,
                    dirty: true,
                    state: NodeState::Normal,
                    link_url: Some(action.to_string()),
                },
                text_color,
                Some(TextRun {
                    text: lbl.to_string(),
                    font_size: 11.0,
                }),
            );
            z += 1;
        }

        // 2. Command Line / Address Bar Area (y = 44..84)
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 0.0,
                    y: 44.0,
                    width: viewport_w,
                    height: 40.0,
                },
                kind: SceneNodeKind::SolidRect,
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: None,
            },
            bg_color,
            None,
        );
        z += 1;

        // Address Input Container
        let addr_w = viewport_w - 32.0;
        let addr_bg = if self.is_address_bar_focused {
            surface_color
        } else {
            surface_color
        };

        scene.push(
            SceneNode {
                rect: Rect {
                    x: 16.0,
                    y: 48.0,
                    width: addr_w,
                    height: 32.0,
                },
                kind: SceneNodeKind::SolidRect,
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: Some("asteria://addressbar/focus".to_string()),
            },
            addr_bg,
            None,
        );
        z += 1;

        // Address Container Border
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 16.0,
                    y: 48.0,
                    width: addr_w,
                    height: 32.0,
                },
                kind: SceneNodeKind::Border {
                    widths: EdgeSizes {
                        top: 1.0,
                        right: 1.0,
                        bottom: 1.0,
                        left: 1.0,
                    },
                },
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: None,
            },
            if self.is_address_bar_focused {
                primary_color
            } else {
                border_color
            },
            None,
        );
        z += 1;

        // Terminal Icon Badge "[>]"
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 24.0,
                    y: 56.0,
                    width: 24.0,
                    height: 16.0,
                },
                kind: SceneNodeKind::Text { font_size: 11.0 },
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: Some("asteria://addressbar/focus".to_string()),
            },
            primary_color,
            Some(TextRun {
                text: "[>]".to_string(),
                font_size: 11.0,
            }),
        );
        z += 1;

        // Address Text
        let active_url = if self.is_address_bar_focused {
            &self.address_bar_buffer
        } else {
            &tab_manager.active_tab().url
        };

        scene.push(
            SceneNode {
                rect: Rect {
                    x: 52.0,
                    y: 56.0,
                    width: addr_w - 100.0,
                    height: 16.0,
                },
                kind: SceneNodeKind::Text { font_size: 12.0 },
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: Some("asteria://addressbar/focus".to_string()),
            },
            text_color,
            Some(TextRun {
                text: active_url.clone(),
                font_size: 12.0,
            }),
        );
        z += 1;

        // Shortcut Badge "[K]"
        scene.push(
            SceneNode {
                rect: Rect {
                    x: viewport_w - 52.0,
                    y: 56.0,
                    width: 24.0,
                    height: 16.0,
                },
                kind: SceneNodeKind::Text { font_size: 11.0 },
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: None,
            },
            muted_color,
            Some(TextRun {
                text: "[K]".to_string(),
                font_size: 11.0,
            }),
        );
        z += 1;

        // 3. Telemetry Status Footer (y = viewport_h - 32..viewport_h)
        let footer_y = viewport_h - 32.0;

        scene.push(
            SceneNode {
                rect: Rect {
                    x: 0.0,
                    y: footer_y,
                    width: viewport_w,
                    height: 32.0,
                },
                kind: SceneNodeKind::SolidRect,
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: None,
            },
            bg_color,
            None,
        );
        z += 1;

        // Footer Top Border Line
        scene.push(
            SceneNode {
                rect: Rect {
                    x: 0.0,
                    y: footer_y,
                    width: viewport_w,
                    height: 1.0,
                },
                kind: SceneNodeKind::SolidRect,
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: None,
            },
            border_color,
            None,
        );
        z += 1;

        // Status Ticker Text
        let ticker_text = format!(
            "FPS: {}  |  LATENCY: {}ms  |  ENGINE: v0.1.0  |  KERNEL: AST-902  |  THEME: {:?}",
            self.last_fps, self.last_latency_ms, self.theme.mode
        );

        scene.push(
            SceneNode {
                rect: Rect {
                    x: 16.0,
                    y: footer_y + 9.0,
                    width: viewport_w - 32.0,
                    height: 16.0,
                },
                kind: SceneNodeKind::Text { font_size: 11.0 },
                parent: None,
                z_order: z,
                segment_id: 0,
                dirty: true,
                state: NodeState::Normal,
                link_url: None,
            },
            muted_color,
            Some(TextRun {
                text: ticker_text,
                font_size: 11.0,
            }),
        );

        scene
    }

    /// Dispatch click actions targeting asteria:// URLs
    pub fn handle_action_click(
        &mut self,
        url: &str,
        tab_manager: &mut TabManager,
    ) -> Option<ChromeAction> {
        if url == "asteria://theme/toggle" {
            self.toggle_theme();
            Some(ChromeAction::RedrawRequired)
        } else if let Some(idx_str) = url.strip_prefix("asteria://tab/switch/") {
            if let Ok(idx) = idx_str.parse::<usize>() {
                let _ = tab_manager.handle_event(crate::shell::ShellEvent::SwitchTab(idx));
                Some(ChromeAction::RedrawRequired)
            } else {
                None
            }
        } else if url == "asteria://tab/new" {
            let _ = tab_manager.handle_event(crate::shell::ShellEvent::NewTab(
                "https://en.wikipedia.org".to_string(),
            ));
            Some(ChromeAction::RedrawRequired)
        } else if url == "asteria://window/minimize" {
            Some(ChromeAction::MinimizeWindow)
        } else if url == "asteria://window/maximize" {
            Some(ChromeAction::MaximizeWindow)
        } else if url == "asteria://window/close" {
            Some(ChromeAction::CloseWindow)
        } else if url == "asteria://addressbar/focus" {
            self.is_address_bar_focused = true;
            self.address_bar_buffer = tab_manager.active_tab().url.clone();
            Some(ChromeAction::RedrawRequired)
        } else {
            None
        }
    }
}

pub enum ChromeAction {
    RedrawRequired,
    MinimizeWindow,
    MaximizeWindow,
    CloseWindow,
}
