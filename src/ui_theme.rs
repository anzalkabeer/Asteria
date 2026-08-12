// ─── Laboratory Design System Theme ──────────────────────────────────────────
//
// Design system tokens for Laboratory Dark & Laboratory Light themes,
// extracted from UI/main_dark.html and UI/main_light.html.

use crate::values::Color;

/// Theme mode enum for the browser shell UI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Dark,
    Light,
}

impl Default for ThemeMode {
    fn default() -> Self {
        ThemeMode::Dark
    }
}

/// Laboratory Theme Color Palette.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabTheme {
    pub mode: ThemeMode,
    /// Background color (Dark: #0b1326, Light: #f8fafc)
    pub background: Color,
    /// Surface container color (Dark: #0f172a, Light: #ffffff)
    pub surface: Color,
    /// Border color (Dark: #1e293b, Light: #e2e8f0)
    pub border: Color,
    /// Primary text color (Dark: #f8fafc, Light: #0f172a)
    pub text: Color,
    /// Muted / Secondary text color (Dark: #94a3b8, Light: #64748b)
    pub muted: Color,
    /// Brand Accent color (Dark: #3b82f6, Light: #2563eb)
    pub accent: Color,
    /// Alert / Warning color (#ef4444)
    pub alert: Color,
    /// Success / Online indicator (#10b981)
    pub success: Color,
}

impl LabTheme {
    /// Laboratory Dark Theme Palette (from UI/main_dark.html).
    pub fn dark() -> Self {
        LabTheme {
            mode: ThemeMode::Dark,
            background: Color::from_hex("#0b1326").unwrap_or(Color::new(11, 19, 38, 255)),
            surface: Color::from_hex("#0f172a").unwrap_or(Color::new(15, 23, 42, 255)),
            border: Color::from_hex("#1e293b").unwrap_or(Color::new(30, 41, 59, 255)),
            text: Color::from_hex("#f8fafc").unwrap_or(Color::new(248, 250, 252, 255)),
            muted: Color::from_hex("#94a3b8").unwrap_or(Color::new(148, 163, 184, 255)),
            accent: Color::from_hex("#3b82f6").unwrap_or(Color::new(59, 130, 246, 255)),
            alert: Color::from_hex("#ef4444").unwrap_or(Color::new(239, 68, 68, 255)),
            success: Color::from_hex("#10b981").unwrap_or(Color::new(16, 185, 129, 255)),
        }
    }

    /// Laboratory Light Theme Palette (from UI/main_light.html).
    pub fn light() -> Self {
        LabTheme {
            mode: ThemeMode::Light,
            background: Color::from_hex("#f8fafc").unwrap_or(Color::new(248, 250, 252, 255)),
            surface: Color::from_hex("#ffffff").unwrap_or(Color::new(255, 255, 255, 255)),
            border: Color::from_hex("#e2e8f0").unwrap_or(Color::new(226, 232, 240, 255)),
            text: Color::from_hex("#0f172a").unwrap_or(Color::new(15, 23, 42, 255)),
            muted: Color::from_hex("#64748b").unwrap_or(Color::new(100, 116, 139, 255)),
            accent: Color::from_hex("#2563eb").unwrap_or(Color::new(37, 99, 235, 255)),
            alert: Color::from_hex("#ef4444").unwrap_or(Color::new(239, 68, 68, 255)),
            success: Color::from_hex("#10b981").unwrap_or(Color::new(16, 185, 129, 255)),
        }
    }
}

impl Default for LabTheme {
    fn default() -> Self {
        Self::dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_colors() {
        let dark = LabTheme::dark();
        assert_eq!(dark.mode, ThemeMode::Dark);

        let light = LabTheme::light();
        assert_eq!(light.mode, ThemeMode::Light);
    }
}
