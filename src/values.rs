// ─── CSS Value Types & Computation ───────────────────────────────
//
// This module defines the typed value representations that replace
// raw strings throughout the style engine. After cascade + defaulting,
// every CSS property value is converted into a concrete Rust type
// that layout can consume directly — no string parsing needed.
//
// Key types:
//   Color       — rgba(r, g, b, a)
//   Display     — Block | Inline | InlineBlock | None
//   TextAlign   — Left | Right | Center | Justify
//   Position    — Static | Relative | Absolute | Fixed
//   BorderStyle — None | Solid | Dashed | Dotted
//   Edges       — { top, right, bottom, left } in px
//   ComputedStyle — flat struct of all resolved property values
//
// Key functions:
//   parse_color()    — "red", "#ff0000", "#f00", "rgb(255,0,0)"
//   parse_length()   — "16px", "2em", "1.5rem", "50%"
//   parse_display()  — "block", "inline", "none"
//   parse_edges()    — "10px", "10px 20px", "10px 20px 30px 40px"

use crate::properties::PropertyId;

// ─── Color ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    /// Default text color: black
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    /// Default background: transparent
    pub const TRANSPARENT: Color = Color::new(0, 0, 0, 0);
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.a == 255 {
            write!(f, "rgb({},{},{})", self.r, self.g, self.b)
        } else {
            write!(f, "rgba({},{},{},{})", self.r, self.g, self.b, self.a)
        }
    }
}

// ─── Enums ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
    InlineBlock,
    None,
}

impl std::fmt::Display for Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Display::Block => write!(f, "block"),
            Display::Inline => write!(f, "inline"),
            Display::InlineBlock => write!(f, "inline-block"),
            Display::None => write!(f, "none"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Left,
    Right,
    Center,
    Justify,
}

impl std::fmt::Display for TextAlign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextAlign::Left => write!(f, "left"),
            TextAlign::Right => write!(f, "right"),
            TextAlign::Center => write!(f, "center"),
            TextAlign::Justify => write!(f, "justify"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyleValue {
    None,
    Solid,
    Dashed,
    Dotted,
}

// ─── Edges (margin/padding) ──────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edges {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl Edges {
    pub const ZERO: Edges = Edges {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    pub fn uniform(v: f32) -> Self {
        Edges {
            top: v,
            right: v,
            bottom: v,
            left: v,
        }
    }
}

impl std::fmt::Display for Edges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.top == self.right && self.right == self.bottom && self.bottom == self.left {
            write!(f, "{}px", self.top)
        } else {
            write!(
                f,
                "{}px {}px {}px {}px",
                self.top, self.right, self.bottom, self.left
            )
        }
    }
}

// ─── ComputedStyle ───────────────────────────────────────────────
//
// The final resolved style for one DOM node. Every value is a
// concrete Rust type — layout reads directly from these fields.

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle {
    // Box model
    pub display: Display,
    pub position: Position,
    pub width: Option<f32>,  // None = auto
    pub height: Option<f32>, // None = auto

    // Margins (px)
    pub margin: Edges,
    // Padding (px)
    pub padding: Edges,

    // Borders
    pub border_width: Edges,
    pub border_color: Color,
    pub border_style: BorderStyleValue,

    // Text & font
    pub color: Color,
    pub background_color: Color,
    pub font_size: f32,   // always px
    pub font_weight: f32, // 400 = normal, 700 = bold
    pub text_align: TextAlign,
    pub line_height: f32, // px
}

impl Default for ComputedStyle {
    /// CSS initial values per spec
    fn default() -> Self {
        ComputedStyle {
            display: Display::Inline,
            position: Position::Static,
            width: None,
            height: None,
            margin: Edges::ZERO,
            padding: Edges::ZERO,
            border_width: Edges::ZERO,
            border_color: Color::BLACK,
            border_style: BorderStyleValue::None,
            color: Color::BLACK,
            background_color: Color::TRANSPARENT,
            font_size: 16.0,    // browser default
            font_weight: 400.0, // normal
            text_align: TextAlign::Left,
            line_height: 19.2, // 1.2 * 16px default
        }
    }
}

impl ComputedStyle {
    /// Get a displayable string value for a specific property.
    /// Used by the tree printer.
    pub fn get_property_display(&self, prop: PropertyId) -> String {
        match prop {
            PropertyId::Display => format!("{}", self.display),
            PropertyId::Position => format!("{:?}", self.position).to_ascii_lowercase(),
            PropertyId::Width => match self.width {
                Some(v) => format!("{}px", v),
                None => "auto".to_string(),
            },
            PropertyId::Height => match self.height {
                Some(v) => format!("{}px", v),
                None => "auto".to_string(),
            },
            PropertyId::MarginTop => format!("{}px", self.margin.top),
            PropertyId::MarginRight => format!("{}px", self.margin.right),
            PropertyId::MarginBottom => format!("{}px", self.margin.bottom),
            PropertyId::MarginLeft => format!("{}px", self.margin.left),
            PropertyId::PaddingTop => format!("{}px", self.padding.top),
            PropertyId::PaddingRight => format!("{}px", self.padding.right),
            PropertyId::PaddingBottom => format!("{}px", self.padding.bottom),
            PropertyId::PaddingLeft => format!("{}px", self.padding.left),
            PropertyId::BorderTopWidth => format!("{}px", self.border_width.top),
            PropertyId::BorderRightWidth => format!("{}px", self.border_width.right),
            PropertyId::BorderBottomWidth => format!("{}px", self.border_width.bottom),
            PropertyId::BorderLeftWidth => format!("{}px", self.border_width.left),
            PropertyId::BorderColor => format!("{}", self.border_color),
            PropertyId::BorderStyle => format!("{:?}", self.border_style).to_ascii_lowercase(),
            PropertyId::Color => format!("{}", self.color),
            PropertyId::BackgroundColor => format!("{}", self.background_color),
            PropertyId::FontSize => format!("{}px", self.font_size),
            PropertyId::FontWeight => format!("{}", self.font_weight),
            PropertyId::TextAlign => format!("{}", self.text_align),
            PropertyId::LineHeight => format!("{}px", self.line_height),
        }
    }

    /// Set a single property from its PropertyId and a raw string value.
    /// `parent_font_size` is needed to resolve em/% on font-size.
    /// `self.font_size` must already be resolved before calling this
    /// for non-font-size properties (em depends on element's own font-size).
    pub fn set_property(
        &mut self,
        prop: PropertyId,
        value: &str,
        parent_font_size: f32,
        root_font_size: f32,
    ) {
        match prop {
            PropertyId::Display => self.display = parse_display(value),
            PropertyId::Position => self.position = parse_position(value),
            PropertyId::Width => {
                self.width = parse_optional_length(value, self.font_size, root_font_size)
            }
            PropertyId::Height => {
                self.height = parse_optional_length(value, self.font_size, root_font_size)
            }

            PropertyId::MarginTop => {
                self.margin.top = parse_length(value, self.font_size, root_font_size)
            }
            PropertyId::MarginRight => {
                self.margin.right = parse_length(value, self.font_size, root_font_size)
            }
            PropertyId::MarginBottom => {
                self.margin.bottom = parse_length(value, self.font_size, root_font_size)
            }
            PropertyId::MarginLeft => {
                self.margin.left = parse_length(value, self.font_size, root_font_size)
            }

            PropertyId::PaddingTop => {
                self.padding.top = parse_length(value, self.font_size, root_font_size)
            }
            PropertyId::PaddingRight => {
                self.padding.right = parse_length(value, self.font_size, root_font_size)
            }
            PropertyId::PaddingBottom => {
                self.padding.bottom = parse_length(value, self.font_size, root_font_size)
            }
            PropertyId::PaddingLeft => {
                self.padding.left = parse_length(value, self.font_size, root_font_size)
            }

            PropertyId::BorderTopWidth => {
                self.border_width.top = parse_length(value, self.font_size, root_font_size)
            }
            PropertyId::BorderRightWidth => {
                self.border_width.right = parse_length(value, self.font_size, root_font_size)
            }
            PropertyId::BorderBottomWidth => {
                self.border_width.bottom = parse_length(value, self.font_size, root_font_size)
            }
            PropertyId::BorderLeftWidth => {
                self.border_width.left = parse_length(value, self.font_size, root_font_size)
            }
            PropertyId::BorderColor => self.border_color = parse_color(value),
            PropertyId::BorderStyle => self.border_style = parse_border_style(value),

            PropertyId::Color => self.color = parse_color(value),
            PropertyId::BackgroundColor => self.background_color = parse_color(value),

            // font-size is special: em/% are relative to PARENT's font-size
            PropertyId::FontSize => {
                self.font_size = parse_length(value, parent_font_size, root_font_size)
            }

            PropertyId::FontWeight => self.font_weight = parse_font_weight(value),
            PropertyId::TextAlign => self.text_align = parse_text_align(value),
            PropertyId::LineHeight => {
                self.line_height = parse_line_height(value, self.font_size, root_font_size)
            }
        }
    }
}

// ─── Parsing Functions ───────────────────────────────────────────

/// Parse a CSS length value into px.
/// Supports: "16px", "2em", "1.5rem", "50%", plain numbers.
/// `em_base` is the reference for em units (element's own font-size,
///  or parent's font-size when resolving font-size itself).
/// `rem_base` is the root element's font-size for rem units.
pub fn parse_length(value: &str, em_base: f32, rem_base: f32) -> f32 {
    let s = value.trim();

    if s == "0" {
        return 0.0;
    }

    // Check rem BEFORE em (rem ends with "em" too)
    if let Some(num) = s.strip_suffix("rem") {
        return num.trim().parse::<f32>().unwrap_or(1.0) * rem_base;
    }

    if let Some(num) = s.strip_suffix("px") {
        return num.trim().parse::<f32>().unwrap_or(0.0);
    }

    if let Some(num) = s.strip_suffix("em") {
        return num.trim().parse::<f32>().unwrap_or(1.0) * em_base;
    }

    if let Some(num) = s.strip_suffix('%') {
        return num.trim().parse::<f32>().unwrap_or(0.0) / 100.0 * em_base;
    }

    // Try plain number (treated as px)
    s.parse::<f32>().unwrap_or(0.0)
}

/// Parse an optional length — returns None for "auto".
pub fn parse_optional_length(value: &str, em_base: f32, rem_base: f32) -> Option<f32> {
    let s = value.trim();
    if s.eq_ignore_ascii_case("auto") {
        None
    } else {
        Some(parse_length(s, em_base, rem_base))
    }
}

/// Parse a CSS color value.
/// Supports: named colors, #hex (3 or 6 digit), rgb(r,g,b).
pub fn parse_color(value: &str) -> Color {
    let s = value.trim().to_ascii_lowercase();

    // Named colors
    if let Some(c) = named_color(&s) {
        return c;
    }

    // Hex colors: #rgb or #rrggbb
    if let Some(hex) = s.strip_prefix('#') {
        return parse_hex_color(hex);
    }

    // rgb(r, g, b)
    if let Some(inner) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let r = parts[0].trim().parse::<u8>().unwrap_or(0);
            let g = parts[1].trim().parse::<u8>().unwrap_or(0);
            let b = parts[2].trim().parse::<u8>().unwrap_or(0);
            return Color::rgb(r, g, b);
        }
    }

    // rgba(r, g, b, a)
    if let Some(inner) = s.strip_prefix("rgba(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 4 {
            let r = parts[0].trim().parse::<u8>().unwrap_or(0);
            let g = parts[1].trim().parse::<u8>().unwrap_or(0);
            let b = parts[2].trim().parse::<u8>().unwrap_or(0);
            let a_str = parts[3].trim();
            let a = if a_str.contains('.') {
                (a_str.parse::<f32>().unwrap_or(1.0) * 255.0) as u8
            } else {
                a_str.parse::<u8>().unwrap_or(255)
            };
            return Color::new(r, g, b, a);
        }
    }

    // Fallback: black
    Color::BLACK
}

/// Parse a hex color string (without the # prefix).
fn parse_hex_color(hex: &str) -> Color {
    match hex.len() {
        3 => {
            let r = u8_from_hex_char(hex.as_bytes()[0]);
            let g = u8_from_hex_char(hex.as_bytes()[1]);
            let b = u8_from_hex_char(hex.as_bytes()[2]);
            Color::rgb(r * 17, g * 17, b * 17)
        }
        6 => {
            let r = u8_from_hex_pair(hex.as_bytes()[0], hex.as_bytes()[1]);
            let g = u8_from_hex_pair(hex.as_bytes()[2], hex.as_bytes()[3]);
            let b = u8_from_hex_pair(hex.as_bytes()[4], hex.as_bytes()[5]);
            Color::rgb(r, g, b)
        }
        8 => {
            let r = u8_from_hex_pair(hex.as_bytes()[0], hex.as_bytes()[1]);
            let g = u8_from_hex_pair(hex.as_bytes()[2], hex.as_bytes()[3]);
            let b = u8_from_hex_pair(hex.as_bytes()[4], hex.as_bytes()[5]);
            let a = u8_from_hex_pair(hex.as_bytes()[6], hex.as_bytes()[7]);
            Color::new(r, g, b, a)
        }
        _ => Color::BLACK,
    }
}

fn hex_digit(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'f' => 10 + b - b'a',
        b'A'..=b'F' => 10 + b - b'A',
        _ => 0,
    }
}

fn u8_from_hex_char(b: u8) -> u8 {
    hex_digit(b)
}

fn u8_from_hex_pair(hi: u8, lo: u8) -> u8 {
    hex_digit(hi) * 16 + hex_digit(lo)
}

/// Lookup table for CSS named colors (the most common ones for V1).
fn named_color(name: &str) -> Option<Color> {
    Some(match name {
        "black" => Color::rgb(0, 0, 0),
        "white" => Color::rgb(255, 255, 255),
        "red" => Color::rgb(255, 0, 0),
        "green" => Color::rgb(0, 128, 0),
        "blue" => Color::rgb(0, 0, 255),
        "yellow" => Color::rgb(255, 255, 0),
        "cyan" | "aqua" => Color::rgb(0, 255, 255),
        "magenta" | "fuchsia" => Color::rgb(255, 0, 255),
        "orange" => Color::rgb(255, 165, 0),
        "purple" => Color::rgb(128, 0, 128),
        "pink" => Color::rgb(255, 192, 203),
        "brown" => Color::rgb(165, 42, 42),
        "gray" | "grey" => Color::rgb(128, 128, 128),
        "silver" => Color::rgb(192, 192, 192),
        "navy" => Color::rgb(0, 0, 128),
        "teal" => Color::rgb(0, 128, 128),
        "olive" => Color::rgb(128, 128, 0),
        "maroon" => Color::rgb(128, 0, 0),
        "lime" => Color::rgb(0, 255, 0),
        "transparent" => Color::new(0, 0, 0, 0),
        _ => return None,
    })
}

/// Parse a CSS display value.
pub fn parse_display(value: &str) -> Display {
    match value.trim().to_ascii_lowercase().as_str() {
        "block" => Display::Block,
        "inline" => Display::Inline,
        "inline-block" => Display::InlineBlock,
        "none" => Display::None,
        _ => Display::Inline,
    }
}

/// Parse a CSS position value.
pub fn parse_position(value: &str) -> Position {
    match value.trim().to_ascii_lowercase().as_str() {
        "static" => Position::Static,
        "relative" => Position::Relative,
        "absolute" => Position::Absolute,
        "fixed" => Position::Fixed,
        _ => Position::Static,
    }
}

/// Parse a CSS font-weight value.
/// "normal" → 400, "bold" → 700, or a numeric value.
pub fn parse_font_weight(value: &str) -> f32 {
    match value.trim().to_ascii_lowercase().as_str() {
        "normal" => 400.0,
        "bold" => 700.0,
        "lighter" => 100.0,
        "bolder" => 900.0,
        other => other.parse::<f32>().unwrap_or(400.0),
    }
}

/// Parse a CSS text-align value.
pub fn parse_text_align(value: &str) -> TextAlign {
    match value.trim().to_ascii_lowercase().as_str() {
        "left" => TextAlign::Left,
        "right" => TextAlign::Right,
        "center" => TextAlign::Center,
        "justify" => TextAlign::Justify,
        _ => TextAlign::Left,
    }
}

/// Parse a CSS border-style value.
pub fn parse_border_style(value: &str) -> BorderStyleValue {
    match value.trim().to_ascii_lowercase().as_str() {
        "none" => BorderStyleValue::None,
        "solid" => BorderStyleValue::Solid,
        "dashed" => BorderStyleValue::Dashed,
        "dotted" => BorderStyleValue::Dotted,
        _ => BorderStyleValue::None,
    }
}

/// Parse a CSS line-height value.
/// Supports: "normal" (1.2x font-size), px, em, unitless multiplier.
pub fn parse_line_height(value: &str, font_size: f32, rem_base: f32) -> f32 {
    let s = value.trim();
    if s.eq_ignore_ascii_case("normal") {
        return font_size * 1.2;
    }
    if s.ends_with("px") || s.ends_with("em") || s.ends_with("rem") || s.ends_with('%') {
        return parse_length(s, font_size, rem_base);
    }
    // Unitless number: multiply by font-size
    s.parse::<f32>()
        .map(|v| v * font_size)
        .unwrap_or(font_size * 1.2)
}

/// Parse a shorthand margin/padding value into 4 edge values.
/// CSS shorthand rules:
///   1 value:  all four edges
///   2 values: vertical horizontal
///   3 values: top horizontal bottom
///   4 values: top right bottom left
pub fn parse_edges(value: &str, em_base: f32, rem_base: f32) -> Edges {
    let parts: Vec<&str> = value.split_whitespace().collect();
    match parts.len() {
        1 => {
            let v = parse_length(parts[0], em_base, rem_base);
            Edges::uniform(v)
        }
        2 => {
            let v = parse_length(parts[0], em_base, rem_base);
            let h = parse_length(parts[1], em_base, rem_base);
            Edges {
                top: v,
                right: h,
                bottom: v,
                left: h,
            }
        }
        3 => {
            let t = parse_length(parts[0], em_base, rem_base);
            let h = parse_length(parts[1], em_base, rem_base);
            let b = parse_length(parts[2], em_base, rem_base);
            Edges {
                top: t,
                right: h,
                bottom: b,
                left: h,
            }
        }
        4 => Edges {
            top: parse_length(parts[0], em_base, rem_base),
            right: parse_length(parts[1], em_base, rem_base),
            bottom: parse_length(parts[2], em_base, rem_base),
            left: parse_length(parts[3], em_base, rem_base),
        },
        _ => Edges::ZERO,
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_colors() {
        assert_eq!(parse_color("red"), Color::rgb(255, 0, 0));
        assert_eq!(parse_color("blue"), Color::rgb(0, 0, 255));
        assert_eq!(parse_color("black"), Color::rgb(0, 0, 0));
        assert_eq!(parse_color("white"), Color::rgb(255, 255, 255));
        assert_eq!(parse_color("transparent"), Color::new(0, 0, 0, 0));
    }

    #[test]
    fn test_hex_colors_6_digit() {
        assert_eq!(parse_color("#ff0000"), Color::rgb(255, 0, 0));
        assert_eq!(parse_color("#00ff00"), Color::rgb(0, 255, 0));
        assert_eq!(parse_color("#0000ff"), Color::rgb(0, 0, 255));
        assert_eq!(parse_color("#f0f0f0"), Color::rgb(240, 240, 240));
    }

    #[test]
    fn test_hex_colors_3_digit() {
        assert_eq!(parse_color("#f00"), Color::rgb(255, 0, 0));
        assert_eq!(parse_color("#0f0"), Color::rgb(0, 255, 0));
        assert_eq!(parse_color("#00f"), Color::rgb(0, 0, 255));
        assert_eq!(parse_color("#fff"), Color::rgb(255, 255, 255));
    }

    #[test]
    fn test_rgb_function() {
        assert_eq!(parse_color("rgb(255,0,0)"), Color::rgb(255, 0, 0));
        assert_eq!(parse_color("rgb(100, 200, 50)"), Color::rgb(100, 200, 50));
    }

    #[test]
    fn test_case_insensitive_colors() {
        assert_eq!(parse_color("RED"), Color::rgb(255, 0, 0));
        assert_eq!(parse_color("Blue"), Color::rgb(0, 0, 255));
        assert_eq!(parse_color("#FF0000"), Color::rgb(255, 0, 0));
    }

    #[test]
    fn test_px_lengths() {
        assert_eq!(parse_length("16px", 16.0, 16.0), 16.0);
        assert_eq!(parse_length("24px", 16.0, 16.0), 24.0);
        assert_eq!(parse_length("0px", 16.0, 16.0), 0.0);
        assert_eq!(parse_length("0", 16.0, 16.0), 0.0);
    }

    #[test]
    fn test_em_lengths() {
        assert_eq!(parse_length("2em", 16.0, 16.0), 32.0);
        assert_eq!(parse_length("1.5em", 20.0, 16.0), 30.0);
        assert_eq!(parse_length("0.5em", 16.0, 16.0), 8.0);
    }

    #[test]
    fn test_rem_lengths() {
        assert_eq!(parse_length("2rem", 20.0, 16.0), 32.0);
        assert_eq!(parse_length("1rem", 20.0, 16.0), 16.0);
    }

    #[test]
    fn test_percent_lengths() {
        assert_eq!(parse_length("50%", 16.0, 16.0), 8.0);
        assert_eq!(parse_length("100%", 20.0, 16.0), 20.0);
        assert_eq!(parse_length("200%", 16.0, 16.0), 32.0);
    }

    #[test]
    fn test_display_values() {
        assert_eq!(parse_display("block"), Display::Block);
        assert_eq!(parse_display("inline"), Display::Inline);
        assert_eq!(parse_display("none"), Display::None);
        assert_eq!(parse_display("inline-block"), Display::InlineBlock);
        assert_eq!(parse_display("BLOCK"), Display::Block);
    }

    #[test]
    fn test_font_weight() {
        assert_eq!(parse_font_weight("normal"), 400.0);
        assert_eq!(parse_font_weight("bold"), 700.0);
        assert_eq!(parse_font_weight("600"), 600.0);
    }

    #[test]
    fn test_edges_one_value() {
        let e = parse_edges("10px", 16.0, 16.0);
        assert_eq!(e, Edges::uniform(10.0));
    }

    #[test]
    fn test_edges_two_values() {
        let e = parse_edges("10px 20px", 16.0, 16.0);
        assert_eq!(e.top, 10.0);
        assert_eq!(e.right, 20.0);
        assert_eq!(e.bottom, 10.0);
        assert_eq!(e.left, 20.0);
    }

    #[test]
    fn test_edges_four_values() {
        let e = parse_edges("1px 2px 3px 4px", 16.0, 16.0);
        assert_eq!(e.top, 1.0);
        assert_eq!(e.right, 2.0);
        assert_eq!(e.bottom, 3.0);
        assert_eq!(e.left, 4.0);
    }

    #[test]
    fn test_default_computed_style() {
        let s = ComputedStyle::default();
        assert_eq!(s.display, Display::Inline);
        assert_eq!(s.color, Color::BLACK);
        assert_eq!(s.background_color, Color::TRANSPARENT);
        assert_eq!(s.font_size, 16.0);
        assert_eq!(s.font_weight, 400.0);
        assert_eq!(s.margin, Edges::ZERO);
        assert_eq!(s.padding, Edges::ZERO);
        assert_eq!(s.width, None);
        assert_eq!(s.height, None);
    }

    #[test]
    fn test_line_height() {
        assert_eq!(parse_line_height("normal", 16.0, 16.0), 19.2);
        assert_eq!(parse_line_height("24px", 16.0, 16.0), 24.0);
        assert_eq!(parse_line_height("1.5", 16.0, 16.0), 24.0);
        assert_eq!(parse_line_height("2em", 16.0, 16.0), 32.0);
    }

    #[test]
    fn test_optional_length() {
        assert_eq!(parse_optional_length("auto", 16.0, 16.0), None);
        assert_eq!(parse_optional_length("100px", 16.0, 16.0), Some(100.0));
        assert_eq!(parse_optional_length("AUTO", 16.0, 16.0), None);
    }
}
