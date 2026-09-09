// ─── ASTERIA Browser Shell UI ─────────────────────────────────────
//
// GPU-rendered browser chrome: Tab Bar, Navigation Toolbar, Omnibox,
// and Status Bar. All visuals are emitted as rects (BatchBuilder) and
// text (TextPass) — no external UI toolkit dependencies.
//
// Architecture:
//   ShellUiState holds all interactive state (omnibox text, focus,
//   hover targets). Each frame, render_shell_rects() and
//   render_shell_text() emit draw primitives layered on top of the
//   webpage viewport. hit_test() maps pixel coordinates to
//   ShellHitTarget variants for event dispatch.

use crate::renderer::commands::batch_builder::BatchBuilder;
use crate::renderer::passes::text_pass::TextPass;
use crate::shell::TabManager;

// ─── Layout Metrics ──────────────────────────────────────────────

pub const TAB_BAR_HEIGHT: f32 = 38.0;
pub const TOOLBAR_HEIGHT: f32 = 42.0;
pub const HEADER_HEIGHT: f32 = TAB_BAR_HEIGHT + TOOLBAR_HEIGHT; // 80px
pub const STATUS_BAR_HEIGHT: f32 = 24.0;

const TAB_MIN_WIDTH: f32 = 80.0;
const TAB_MAX_WIDTH: f32 = 220.0;
const TAB_PADDING: f32 = 12.0;
const TAB_CLOSE_SIZE: f32 = 16.0;
const TAB_CLOSE_MARGIN: f32 = 6.0;
const TAB_GAP: f32 = 2.0;
const TAB_START_X: f32 = 4.0;
const NEW_TAB_BTN_WIDTH: f32 = 32.0;

const NAV_BTN_SIZE: f32 = 30.0;
const NAV_BTN_GAP: f32 = 4.0;
const NAV_BTN_Y: f32 = TAB_BAR_HEIGHT + (TOOLBAR_HEIGHT - NAV_BTN_SIZE) / 2.0;
const NAV_BTN_START_X: f32 = 8.0;

const OMNIBOX_LEFT_MARGIN: f32 = NAV_BTN_START_X + (NAV_BTN_SIZE + NAV_BTN_GAP) * 4.0 + 8.0;
const OMNIBOX_RIGHT_MARGIN: f32 = 48.0; // space for settings button
const OMNIBOX_HEIGHT: f32 = 30.0;
const OMNIBOX_Y: f32 = TAB_BAR_HEIGHT + (TOOLBAR_HEIGHT - OMNIBOX_HEIGHT) / 2.0;
const OMNIBOX_GO_WIDTH: f32 = 32.0;

const ACCENT_STRIP_HEIGHT: f32 = 2.0;

// ─── Theme Colors (Catppuccin Mocha) ─────────────────────────────

pub struct ShellTheme;

impl ShellTheme {
    // Backgrounds
    pub const TAB_BAR_BG: [f32; 4] = [0.067, 0.067, 0.106, 1.0]; // #11111b
    pub const TOOLBAR_BG: [f32; 4] = [0.118, 0.118, 0.180, 1.0]; // #1e1e2e
    pub const STATUS_BAR_BG: [f32; 4] = [0.067, 0.067, 0.106, 1.0]; // #11111b

    // Borders
    pub const BORDER: [f32; 4] = [0.192, 0.200, 0.267, 1.0]; // #313244

    // Tab states
    pub const TAB_ACTIVE_BG: [f32; 4] = [0.118, 0.118, 0.180, 1.0]; // #1e1e2e
    pub const TAB_INACTIVE_BG: [f32; 4] = [0.094, 0.094, 0.145, 1.0]; // #181825
    pub const TAB_HOVER_BG: [f32; 4] = [0.149, 0.149, 0.224, 1.0]; // #262639
    pub const TAB_ACCENT: [f32; 4] = [0.537, 0.706, 0.980, 1.0]; // #89b4fa

    // Close button
    pub const CLOSE_HOVER_BG: [f32; 4] = [0.953, 0.545, 0.659, 1.0]; // #f38ba8
    pub const CLOSE_NORMAL: [f32; 4] = [0.651, 0.678, 0.784, 1.0]; // #a6adc8

    // Text
    pub const TEXT_PRIMARY: [f32; 4] = [0.804, 0.839, 0.957, 1.0]; // #cdd6f4
    pub const TEXT_SECONDARY: [f32; 4] = [0.651, 0.678, 0.784, 1.0]; // #a6adc8
    pub const TEXT_DISABLED: [f32; 4] = [0.271, 0.278, 0.353, 1.0]; // #45475a

    // Omnibox
    pub const OMNIBOX_BG: [f32; 4] = [0.067, 0.067, 0.106, 1.0]; // #11111b
    pub const OMNIBOX_FOCUSED_BG: [f32; 4] = [0.094, 0.094, 0.145, 1.0]; // #181825
    pub const OMNIBOX_FOCUS_BORDER: [f32; 4] = [0.537, 0.706, 0.980, 1.0]; // #89b4fa

    // Nav buttons
    pub const NAV_HOVER_BG: [f32; 4] = [0.192, 0.200, 0.267, 1.0]; // #313244

    // Status bar
    pub const STATUS_LINK: [f32; 4] = [0.537, 0.706, 0.980, 1.0]; // #89b4fa

    // New tab button
    pub const NEW_TAB_BG: [f32; 4] = [0.094, 0.094, 0.145, 1.0]; // #181825
    pub const NEW_TAB_HOVER_BG: [f32; 4] = [0.149, 0.149, 0.224, 1.0]; // #262639
}

// ─── Hit Target ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellHitTarget {
    /// Clicked on tab at given index
    Tab(usize),
    /// Clicked the close button on tab at given index
    TabClose(usize),
    /// Clicked the new tab (+) button
    NewTab,
    /// Navigation: Back
    Back,
    /// Navigation: Forward
    Forward,
    /// Navigation: Reload
    Reload,
    /// Navigation: Home
    Home,
    /// Clicked the omnibox (address bar)
    Omnibox,
    /// Clicked the Go button in omnibox
    OmniboxGo,
    /// Settings gear icon
    Settings,
    /// Clicked within the webpage viewport
    Webpage,
    /// Status bar area
    StatusBar,
}

// ─── Shell UI State ──────────────────────────────────────────────

pub struct ShellUiState {
    // Omnibox
    pub omnibox_text: String,
    pub omnibox_focused: bool,
    pub omnibox_cursor: usize,

    // Hover
    pub hovered_target: Option<ShellHitTarget>,
    pub hovered_link_url: Option<String>,

    // Status bar
    pub status_message: Option<String>,

    // Cursor blink timer (frame counter)
    pub cursor_blink_counter: u32,
}

impl Default for ShellUiState {
    fn default() -> Self {
        Self::new()
    }
}

impl ShellUiState {
    pub fn new() -> Self {
        Self {
            omnibox_text: String::new(),
            omnibox_focused: false,
            omnibox_cursor: 0,
            hovered_target: None,
            hovered_link_url: None,
            status_message: None,
            cursor_blink_counter: 0,
        }
    }

    /// Synchronize omnibox text with the active tab's URL.
    pub fn sync_with_tab_url(&mut self, url: &str) {
        if !self.omnibox_focused {
            self.omnibox_text = url.to_string();
            self.omnibox_cursor = self.omnibox_text.len();
        }
    }

    /// Focus the omnibox and select all text.
    pub fn focus_omnibox(&mut self) {
        self.omnibox_focused = true;
        self.omnibox_cursor = self.omnibox_text.len();
        self.cursor_blink_counter = 0;
    }

    /// Unfocus the omnibox.
    pub fn unfocus_omnibox(&mut self) {
        self.omnibox_focused = false;
    }

    /// Insert a character at the current cursor position.
    pub fn insert_char(&mut self, ch: char) {
        let byte_pos = self.cursor_byte_offset();
        self.omnibox_text.insert(byte_pos, ch);
        self.omnibox_cursor += 1;
        self.cursor_blink_counter = 0;
    }

    /// Insert a string at the current cursor position.
    pub fn insert_str(&mut self, s: &str) {
        let byte_pos = self.cursor_byte_offset();
        self.omnibox_text.insert_str(byte_pos, s);
        self.omnibox_cursor += s.chars().count();
        self.cursor_blink_counter = 0;
    }

    /// Delete the character before the cursor (Backspace).
    pub fn backspace(&mut self) {
        if self.omnibox_cursor > 0 {
            self.omnibox_cursor -= 1;
            let byte_pos = self.cursor_byte_offset();
            // Find the end of the char at byte_pos
            let ch = self.omnibox_text[byte_pos..].chars().next().unwrap();
            self.omnibox_text.drain(byte_pos..byte_pos + ch.len_utf8());
            self.cursor_blink_counter = 0;
        }
    }

    /// Delete the character after the cursor (Delete key).
    pub fn delete_forward(&mut self) {
        let char_count = self.omnibox_text.chars().count();
        if self.omnibox_cursor < char_count {
            let byte_pos = self.cursor_byte_offset();
            let ch = self.omnibox_text[byte_pos..].chars().next().unwrap();
            self.omnibox_text.drain(byte_pos..byte_pos + ch.len_utf8());
            self.cursor_blink_counter = 0;
        }
    }

    /// Move cursor left by one character.
    pub fn cursor_left(&mut self) {
        if self.omnibox_cursor > 0 {
            self.omnibox_cursor -= 1;
            self.cursor_blink_counter = 0;
        }
    }

    /// Move cursor right by one character.
    pub fn cursor_right(&mut self) {
        let char_count = self.omnibox_text.chars().count();
        if self.omnibox_cursor < char_count {
            self.omnibox_cursor += 1;
            self.cursor_blink_counter = 0;
        }
    }

    /// Move cursor to the beginning.
    pub fn cursor_home(&mut self) {
        self.omnibox_cursor = 0;
        self.cursor_blink_counter = 0;
    }

    /// Move cursor to the end.
    pub fn cursor_end(&mut self) {
        self.omnibox_cursor = self.omnibox_text.chars().count();
        self.cursor_blink_counter = 0;
    }

    /// Get the byte offset corresponding to the current character cursor position.
    fn cursor_byte_offset(&self) -> usize {
        self.omnibox_text
            .char_indices()
            .nth(self.omnibox_cursor)
            .map(|(idx, _)| idx)
            .unwrap_or(self.omnibox_text.len())
    }

    /// Advance blink counter; returns whether cursor should be visible.
    pub fn tick_cursor_blink(&mut self) -> bool {
        self.cursor_blink_counter += 1;
        // Blink every 30 frames (~0.5s at 60fps)
        (self.cursor_blink_counter / 30).is_multiple_of(2)
    }

    // ─── Hit Testing ─────────────────────────────────────────────

    /// Calculate the width of a single tab given the number of tabs and window width.
    fn tab_width(tab_count: usize, window_w: f32) -> f32 {
        if tab_count == 0 {
            return TAB_MIN_WIDTH;
        }
        let available = window_w - TAB_START_X - NEW_TAB_BTN_WIDTH - 8.0;
        let per_tab = (available / tab_count as f32) - TAB_GAP;
        per_tab.clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH)
    }

    /// Determine what element is under the given pixel coordinate.
    pub fn hit_test(
        &self,
        x: f32,
        y: f32,
        window_w: f32,
        window_h: f32,
        tab_count: usize,
    ) -> ShellHitTarget {
        // ── Tab Bar Region (y < TAB_BAR_HEIGHT) ──────────────────
        if y < TAB_BAR_HEIGHT {
            let tw = Self::tab_width(tab_count, window_w);
            for i in 0..tab_count {
                let tx = TAB_START_X + i as f32 * (tw + TAB_GAP);
                if x >= tx && x < tx + tw {
                    // Check close button sub-region
                    let close_x = tx + tw - TAB_CLOSE_MARGIN - TAB_CLOSE_SIZE;
                    let close_y = (TAB_BAR_HEIGHT - TAB_CLOSE_SIZE) / 2.0;
                    if x >= close_x
                        && x <= close_x + TAB_CLOSE_SIZE
                        && y >= close_y
                        && y <= close_y + TAB_CLOSE_SIZE
                    {
                        return ShellHitTarget::TabClose(i);
                    }
                    return ShellHitTarget::Tab(i);
                }
            }
            // New tab button
            let new_tab_x = TAB_START_X + tab_count as f32 * (tw + TAB_GAP) + 4.0;
            if x >= new_tab_x && x < new_tab_x + NEW_TAB_BTN_WIDTH {
                return ShellHitTarget::NewTab;
            }
            return ShellHitTarget::StatusBar; // empty tab bar area
        }

        // ── Toolbar Region (TAB_BAR_HEIGHT .. HEADER_HEIGHT) ─────
        if y < HEADER_HEIGHT {
            // Nav buttons: Back, Forward, Reload, Home
            let buttons = [
                ShellHitTarget::Back,
                ShellHitTarget::Forward,
                ShellHitTarget::Reload,
                ShellHitTarget::Home,
            ];
            for (i, target) in buttons.iter().enumerate() {
                let bx = NAV_BTN_START_X + i as f32 * (NAV_BTN_SIZE + NAV_BTN_GAP);
                if x >= bx
                    && x < bx + NAV_BTN_SIZE
                    && (NAV_BTN_Y..NAV_BTN_Y + NAV_BTN_SIZE).contains(&y)
                {
                    return *target;
                }
            }

            // Omnibox
            let omni_x = OMNIBOX_LEFT_MARGIN;
            let omni_w = window_w - OMNIBOX_LEFT_MARGIN - OMNIBOX_RIGHT_MARGIN;
            if x >= omni_x
                && x < omni_x + omni_w
                && (OMNIBOX_Y..OMNIBOX_Y + OMNIBOX_HEIGHT).contains(&y)
            {
                // Go button at the right end
                let go_x = omni_x + omni_w - OMNIBOX_GO_WIDTH;
                if x >= go_x {
                    return ShellHitTarget::OmniboxGo;
                }
                return ShellHitTarget::Omnibox;
            }

            // Settings button
            let settings_x = window_w - 40.0;
            if x >= settings_x
                && x < settings_x + NAV_BTN_SIZE
                && (NAV_BTN_Y..NAV_BTN_Y + NAV_BTN_SIZE).contains(&y)
            {
                return ShellHitTarget::Settings;
            }

            return ShellHitTarget::StatusBar; // empty toolbar area
        }

        // ── Status Bar Region (bottom STATUS_BAR_HEIGHT pixels) ──
        if y >= window_h - STATUS_BAR_HEIGHT {
            return ShellHitTarget::StatusBar;
        }

        // ── Webpage Viewport ─────────────────────────────────────
        ShellHitTarget::Webpage
    }

    // ─── Rendering ───────────────────────────────────────────────

    /// Emit all shell chrome rectangles into the BatchBuilder.
    /// Called each frame to overlay UI on top of the webpage.
    pub fn render_shell_rects(
        &self,
        batch: &mut BatchBuilder,
        window_w: f32,
        window_h: f32,
        tab_manager: &TabManager,
    ) {
        let tab_count = tab_manager.tabs.len();
        let active_idx = tab_manager.active_tab_index;

        // ── Tab Bar Background ───────────────────────────────────
        batch.add_quad_direct(0.0, 0.0, window_w, TAB_BAR_HEIGHT, ShellTheme::TAB_BAR_BG);

        // ── Individual Tabs ──────────────────────────────────────
        let tw = Self::tab_width(tab_count, window_w);
        for i in 0..tab_count {
            let tx = TAB_START_X + i as f32 * (tw + TAB_GAP);
            let is_active = i == active_idx;
            let is_hovered = self.hovered_target == Some(ShellHitTarget::Tab(i));

            // Tab background
            let bg = if is_active {
                ShellTheme::TAB_ACTIVE_BG
            } else if is_hovered {
                ShellTheme::TAB_HOVER_BG
            } else {
                ShellTheme::TAB_INACTIVE_BG
            };
            batch.add_quad_direct(tx, 0.0, tw, TAB_BAR_HEIGHT, bg);

            // Active tab accent strip
            if is_active {
                batch.add_quad_direct(tx, 0.0, tw, ACCENT_STRIP_HEIGHT, ShellTheme::TAB_ACCENT);
            }

            // Close button hover background
            let close_hovered = self.hovered_target == Some(ShellHitTarget::TabClose(i));
            if close_hovered {
                let close_x = tx + tw - TAB_CLOSE_MARGIN - TAB_CLOSE_SIZE;
                let close_y = (TAB_BAR_HEIGHT - TAB_CLOSE_SIZE) / 2.0;
                batch.add_quad_direct(
                    close_x,
                    close_y,
                    TAB_CLOSE_SIZE,
                    TAB_CLOSE_SIZE,
                    ShellTheme::CLOSE_HOVER_BG,
                );
            }
        }

        // ── New Tab Button ───────────────────────────────────────
        let new_tab_x = TAB_START_X + tab_count as f32 * (tw + TAB_GAP) + 4.0;
        let new_tab_hovered = self.hovered_target == Some(ShellHitTarget::NewTab);
        let new_tab_bg = if new_tab_hovered {
            ShellTheme::NEW_TAB_HOVER_BG
        } else {
            ShellTheme::NEW_TAB_BG
        };
        let new_tab_y = (TAB_BAR_HEIGHT - NAV_BTN_SIZE) / 2.0;
        batch.add_quad_direct(
            new_tab_x,
            new_tab_y,
            NEW_TAB_BTN_WIDTH,
            NAV_BTN_SIZE,
            new_tab_bg,
        );

        // ── Toolbar Background ───────────────────────────────────
        batch.add_quad_direct(
            0.0,
            TAB_BAR_HEIGHT,
            window_w,
            TOOLBAR_HEIGHT,
            ShellTheme::TOOLBAR_BG,
        );

        // Toolbar top border
        batch.add_quad_direct(0.0, TAB_BAR_HEIGHT, window_w, 1.0, ShellTheme::BORDER);

        // ── Navigation Buttons ───────────────────────────────────
        let nav_targets = [
            ShellHitTarget::Back,
            ShellHitTarget::Forward,
            ShellHitTarget::Reload,
            ShellHitTarget::Home,
        ];
        for (i, target) in nav_targets.iter().enumerate() {
            let bx = NAV_BTN_START_X + i as f32 * (NAV_BTN_SIZE + NAV_BTN_GAP);
            let is_hovered = self.hovered_target == Some(*target);
            if is_hovered {
                batch.add_quad_direct(
                    bx,
                    NAV_BTN_Y,
                    NAV_BTN_SIZE,
                    NAV_BTN_SIZE,
                    ShellTheme::NAV_HOVER_BG,
                );
            }
        }

        // ── Omnibox ──────────────────────────────────────────────
        let omni_x = OMNIBOX_LEFT_MARGIN;
        let omni_w = window_w - OMNIBOX_LEFT_MARGIN - OMNIBOX_RIGHT_MARGIN;

        // Omnibox background
        let omni_bg = if self.omnibox_focused {
            ShellTheme::OMNIBOX_FOCUSED_BG
        } else {
            ShellTheme::OMNIBOX_BG
        };
        batch.add_quad_direct(omni_x, OMNIBOX_Y, omni_w, OMNIBOX_HEIGHT, omni_bg);

        // Omnibox border
        let border_color = if self.omnibox_focused {
            ShellTheme::OMNIBOX_FOCUS_BORDER
        } else {
            ShellTheme::BORDER
        };
        // Top border
        batch.add_quad_direct(omni_x, OMNIBOX_Y, omni_w, 1.0, border_color);
        // Bottom border
        batch.add_quad_direct(
            omni_x,
            OMNIBOX_Y + OMNIBOX_HEIGHT - 1.0,
            omni_w,
            1.0,
            border_color,
        );
        // Left border
        batch.add_quad_direct(omni_x, OMNIBOX_Y, 1.0, OMNIBOX_HEIGHT, border_color);
        // Right border
        batch.add_quad_direct(
            omni_x + omni_w - 1.0,
            OMNIBOX_Y,
            1.0,
            OMNIBOX_HEIGHT,
            border_color,
        );

        // Go button separator
        let go_x = omni_x + omni_w - OMNIBOX_GO_WIDTH;
        batch.add_quad_direct(
            go_x,
            OMNIBOX_Y + 4.0,
            1.0,
            OMNIBOX_HEIGHT - 8.0,
            ShellTheme::BORDER,
        );

        // Go button hover
        if self.hovered_target == Some(ShellHitTarget::OmniboxGo) {
            batch.add_quad_direct(
                go_x + 1.0,
                OMNIBOX_Y + 1.0,
                OMNIBOX_GO_WIDTH - 2.0,
                OMNIBOX_HEIGHT - 2.0,
                ShellTheme::NAV_HOVER_BG,
            );
        }

        // ── Status Bar ───────────────────────────────────────────
        let status_y = window_h - STATUS_BAR_HEIGHT;
        batch.add_quad_direct(
            0.0,
            status_y,
            window_w,
            STATUS_BAR_HEIGHT,
            ShellTheme::STATUS_BAR_BG,
        );
        // Top border
        batch.add_quad_direct(0.0, status_y, window_w, 1.0, ShellTheme::BORDER);
    }

    /// Emit all shell chrome text into the TextPass.
    pub fn render_shell_text(
        &mut self,
        text_pass: &mut TextPass,
        window_w: f32,
        window_h: f32,
        tab_manager: &TabManager,
        can_go_back: bool,
        can_go_forward: bool,
    ) {
        let tab_count = tab_manager.tabs.len();
        let active_idx = tab_manager.active_tab_index;
        let tw = Self::tab_width(tab_count, window_w);

        // ── Tab Titles & Close Buttons ───────────────────────────
        for i in 0..tab_count {
            let tx = TAB_START_X + i as f32 * (tw + TAB_GAP);
            let is_active = i == active_idx;
            let text_color = if is_active {
                ShellTheme::TEXT_PRIMARY
            } else {
                ShellTheme::TEXT_SECONDARY
            };

            // Tab title (truncated)
            let title = &tab_manager.tabs[i].title;
            let max_chars = ((tw - TAB_PADDING * 2.0 - TAB_CLOSE_SIZE - TAB_CLOSE_MARGIN) / 7.5)
                .max(3.0) as usize;
            let display_title = truncate_with_ellipsis(title, max_chars);
            let text_y = (TAB_BAR_HEIGHT - 13.0) / 2.0;
            text_pass.add_text(&display_title, [tx + TAB_PADDING, text_y], 13.0, text_color);

            // Close button "x"
            let close_x = tx + tw - TAB_CLOSE_MARGIN - TAB_CLOSE_SIZE;
            let close_y = (TAB_BAR_HEIGHT - TAB_CLOSE_SIZE) / 2.0;
            let close_hovered = self.hovered_target == Some(ShellHitTarget::TabClose(i));
            let close_color = if close_hovered {
                ShellTheme::TEXT_PRIMARY
            } else {
                ShellTheme::CLOSE_NORMAL
            };
            text_pass.add_text("x", [close_x + 3.0, close_y + 1.0], 12.0, close_color);
        }

        // ── New Tab "+" Button ───────────────────────────────────
        let new_tab_x = TAB_START_X + tab_count as f32 * (tw + TAB_GAP) + 4.0;
        let new_tab_y = (TAB_BAR_HEIGHT - 13.0) / 2.0;
        text_pass.add_text(
            "+",
            [new_tab_x + 10.0, new_tab_y],
            14.0,
            ShellTheme::TEXT_SECONDARY,
        );

        // ── Navigation Button Icons ──────────────────────────────
        let nav_icons = ["<", ">", "R", "H"];
        let nav_enabled = [can_go_back, can_go_forward, true, true];
        for (i, (icon, enabled)) in nav_icons.iter().zip(nav_enabled.iter()).enumerate() {
            let bx = NAV_BTN_START_X + i as f32 * (NAV_BTN_SIZE + NAV_BTN_GAP);
            let color = if *enabled {
                ShellTheme::TEXT_PRIMARY
            } else {
                ShellTheme::TEXT_DISABLED
            };
            let icon_x = bx + (NAV_BTN_SIZE - 10.0) / 2.0;
            let icon_y = NAV_BTN_Y + (NAV_BTN_SIZE - 13.0) / 2.0;
            text_pass.add_text(icon, [icon_x, icon_y], 13.0, color);
        }

        // ── Omnibox Text ─────────────────────────────────────────
        let omni_x = OMNIBOX_LEFT_MARGIN;
        let omni_w = window_w - OMNIBOX_LEFT_MARGIN - OMNIBOX_RIGHT_MARGIN;
        let text_x = omni_x + 8.0;
        let text_y = OMNIBOX_Y + (OMNIBOX_HEIGHT - 13.0) / 2.0;
        let available_chars = ((omni_w - OMNIBOX_GO_WIDTH - 16.0) / 7.5).max(1.0) as usize;

        // Protocol indicator
        let (protocol_label, url_display) = if self.omnibox_text.starts_with("https://") {
            ("HTTPS ", &self.omnibox_text[8..])
        } else if self.omnibox_text.starts_with("http://") {
            ("HTTP ", &self.omnibox_text[7..])
        } else {
            ("", self.omnibox_text.as_str())
        };

        if !protocol_label.is_empty() {
            text_pass.add_text(
                protocol_label,
                [text_x, text_y],
                11.0,
                ShellTheme::TAB_ACCENT,
            );
        }

        let protocol_offset = if protocol_label.is_empty() {
            0.0
        } else {
            protocol_label.len() as f32 * 7.0
        };

        let display_url = truncate_with_ellipsis(
            url_display,
            available_chars.saturating_sub(protocol_label.len()),
        );
        text_pass.add_text(
            &display_url,
            [text_x + protocol_offset, text_y],
            13.0,
            ShellTheme::TEXT_PRIMARY,
        );

        // Blinking cursor when focused
        if self.omnibox_focused {
            let cursor_visible = self.tick_cursor_blink();
            if cursor_visible {
                // Calculate cursor x position based on char offset
                let chars_before_cursor =
                    self.omnibox_cursor.min(self.omnibox_text.chars().count());
                let cursor_x_offset = chars_before_cursor as f32 * 7.5;
                text_pass.add_text(
                    "|",
                    [text_x + cursor_x_offset, text_y - 1.0],
                    14.0,
                    ShellTheme::TAB_ACCENT,
                );
            }
        }

        // Go button arrow
        let go_x = omni_x + omni_w - OMNIBOX_GO_WIDTH;
        text_pass.add_text(">", [go_x + 10.0, text_y], 13.0, ShellTheme::TEXT_SECONDARY);

        // ── Status Bar Text ──────────────────────────────────────
        let status_y = window_h - STATUS_BAR_HEIGHT;
        let status_text_y = status_y + (STATUS_BAR_HEIGHT - 11.0) / 2.0;

        // Left side: link hover preview or "Ready"
        let left_text = if let Some(ref url) = self.hovered_link_url {
            url.as_str()
        } else if let Some(ref msg) = self.status_message {
            msg.as_str()
        } else {
            "Ready"
        };
        let left_color = if self.hovered_link_url.is_some() {
            ShellTheme::STATUS_LINK
        } else {
            ShellTheme::TEXT_SECONDARY
        };
        text_pass.add_text(left_text, [8.0, status_text_y], 11.0, left_color);

        // Right side: engine badge
        let badge = "Asteria GPU";
        let badge_x = window_w - (badge.len() as f32 * 6.5) - 12.0;
        text_pass.add_text(
            badge,
            [badge_x, status_text_y],
            11.0,
            ShellTheme::TEXT_SECONDARY,
        );
    }
}

// ─── Helpers ─────────────────────────────────────────────────────

/// Truncate a string to `max_chars` and append "..." if truncated.
fn truncate_with_ellipsis(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        s.to_string()
    } else if max_chars <= 3 {
        s.chars().take(max_chars).collect()
    } else {
        let mut result: String = s.chars().take(max_chars - 3).collect();
        result.push_str("...");
        result
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_ui_hit_test_tabs() {
        let state = ShellUiState::new();

        // With 3 tabs at 800px width
        let window_w = 800.0;
        let window_h = 600.0;
        let tab_count = 3;
        let tw = ShellUiState::tab_width(tab_count, window_w);

        // Click on first tab center
        let result = state.hit_test(
            TAB_START_X + tw / 2.0,
            TAB_BAR_HEIGHT / 2.0,
            window_w,
            window_h,
            tab_count,
        );
        assert_eq!(result, ShellHitTarget::Tab(0));

        // Click on second tab center
        let result = state.hit_test(
            TAB_START_X + (tw + TAB_GAP) + tw / 2.0,
            TAB_BAR_HEIGHT / 2.0,
            window_w,
            window_h,
            tab_count,
        );
        assert_eq!(result, ShellHitTarget::Tab(1));

        // Click on first tab close button
        let close_x = TAB_START_X + tw - TAB_CLOSE_MARGIN - TAB_CLOSE_SIZE / 2.0;
        let close_y = TAB_BAR_HEIGHT / 2.0;
        let result = state.hit_test(close_x, close_y, window_w, window_h, tab_count);
        assert_eq!(result, ShellHitTarget::TabClose(0));
    }

    #[test]
    fn test_shell_ui_hit_test_buttons() {
        let state = ShellUiState::new();
        let window_w = 800.0;
        let window_h = 600.0;

        // Back button
        let bx = NAV_BTN_START_X + NAV_BTN_SIZE / 2.0;
        let by = NAV_BTN_Y + NAV_BTN_SIZE / 2.0;
        let result = state.hit_test(bx, by, window_w, window_h, 1);
        assert_eq!(result, ShellHitTarget::Back);

        // Forward button
        let fx = NAV_BTN_START_X + (NAV_BTN_SIZE + NAV_BTN_GAP) + NAV_BTN_SIZE / 2.0;
        let result = state.hit_test(fx, by, window_w, window_h, 1);
        assert_eq!(result, ShellHitTarget::Forward);

        // Reload button
        let rx = NAV_BTN_START_X + 2.0 * (NAV_BTN_SIZE + NAV_BTN_GAP) + NAV_BTN_SIZE / 2.0;
        let result = state.hit_test(rx, by, window_w, window_h, 1);
        assert_eq!(result, ShellHitTarget::Reload);

        // Home button
        let hx = NAV_BTN_START_X + 3.0 * (NAV_BTN_SIZE + NAV_BTN_GAP) + NAV_BTN_SIZE / 2.0;
        let result = state.hit_test(hx, by, window_w, window_h, 1);
        assert_eq!(result, ShellHitTarget::Home);
    }

    #[test]
    fn test_shell_ui_hit_test_omnibox() {
        let state = ShellUiState::new();
        let window_w = 800.0;
        let window_h = 600.0;

        // Click center of omnibox
        let omni_x = OMNIBOX_LEFT_MARGIN;
        let omni_w = window_w - OMNIBOX_LEFT_MARGIN - OMNIBOX_RIGHT_MARGIN;
        let center_x = omni_x + omni_w / 2.0;
        let center_y = OMNIBOX_Y + OMNIBOX_HEIGHT / 2.0;
        let result = state.hit_test(center_x, center_y, window_w, window_h, 1);
        assert_eq!(result, ShellHitTarget::Omnibox);

        // Click Go button
        let go_x = omni_x + omni_w - OMNIBOX_GO_WIDTH / 2.0;
        let result = state.hit_test(go_x, center_y, window_w, window_h, 1);
        assert_eq!(result, ShellHitTarget::OmniboxGo);

        // Click in webpage area
        let result = state.hit_test(400.0, 300.0, window_w, window_h, 1);
        assert_eq!(result, ShellHitTarget::Webpage);

        // Click in status bar
        let result = state.hit_test(400.0, window_h - 10.0, window_w, window_h, 1);
        assert_eq!(result, ShellHitTarget::StatusBar);
    }

    #[test]
    fn test_shell_ui_text_editing() {
        let mut state = ShellUiState::new();
        state.focus_omnibox();

        // Type "hello"
        for ch in "hello".chars() {
            state.insert_char(ch);
        }
        assert_eq!(state.omnibox_text, "hello");
        assert_eq!(state.omnibox_cursor, 5);

        // Backspace
        state.backspace();
        assert_eq!(state.omnibox_text, "hell");
        assert_eq!(state.omnibox_cursor, 4);

        // Move cursor left twice
        state.cursor_left();
        state.cursor_left();
        assert_eq!(state.omnibox_cursor, 2);

        // Insert at cursor
        state.insert_char('X');
        assert_eq!(state.omnibox_text, "heXll");
        assert_eq!(state.omnibox_cursor, 3);

        // Delete forward
        state.delete_forward();
        assert_eq!(state.omnibox_text, "heXl");
        assert_eq!(state.omnibox_cursor, 3);

        // Home
        state.cursor_home();
        assert_eq!(state.omnibox_cursor, 0);

        // End
        state.cursor_end();
        assert_eq!(state.omnibox_cursor, 4);

        // Insert string
        state.cursor_home();
        state.insert_str("https://");
        assert_eq!(state.omnibox_text, "https://heXl");
        assert_eq!(state.omnibox_cursor, 8);
    }

    #[test]
    fn test_shell_ui_tab_title_truncation() {
        assert_eq!(truncate_with_ellipsis("Hello", 10), "Hello");
        assert_eq!(truncate_with_ellipsis("Hello, World!", 10), "Hello, ...");
        assert_eq!(truncate_with_ellipsis("AB", 3), "AB");
        assert_eq!(truncate_with_ellipsis("ABCD", 3), "ABC");
        assert_eq!(truncate_with_ellipsis("A", 1), "A");
        assert_eq!(
            truncate_with_ellipsis("https://example.com/very/long/path/to/page.html", 20),
            "https://example.c..."
        );
    }

    #[test]
    fn test_shell_ui_new_tab_hit_test() {
        let state = ShellUiState::new();
        let window_w = 800.0;
        let window_h = 600.0;

        // With 1 tab, new tab button should be after first tab
        let tw = ShellUiState::tab_width(1, window_w);
        let ntx = TAB_START_X + (tw + TAB_GAP) + 4.0 + NEW_TAB_BTN_WIDTH / 2.0;
        let nty = TAB_BAR_HEIGHT / 2.0;
        let result = state.hit_test(ntx, nty, window_w, window_h, 1);
        assert_eq!(result, ShellHitTarget::NewTab);
    }

    #[test]
    fn test_shell_ui_sync_unfocused() {
        let mut state = ShellUiState::new();
        state.sync_with_tab_url("https://example.com");
        assert_eq!(state.omnibox_text, "https://example.com");
        assert_eq!(state.omnibox_cursor, 19);

        // When focused, sync should not overwrite
        state.focus_omnibox();
        state.omnibox_text = "typed-text".to_string();
        state.sync_with_tab_url("https://other.com");
        assert_eq!(state.omnibox_text, "typed-text");
    }
}
