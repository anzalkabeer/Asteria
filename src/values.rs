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

    /// Return (r, g, b, a) tuple for GPU color conversion
    pub const fn to_rgba(self) -> (u8, u8, u8, u8) {
        (self.r, self.g, self.b, self.a)
    }
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

/// A CSS color value which can be either a concrete RGBA color or the `currentColor` keyword.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CssColor {
    Rgba(Color),
    CurrentColor,
}

impl CssColor {
    /// Resolve this CSS color against the element's computed text color.
    pub fn resolve(self, current_color: Color) -> Color {
        match self {
            CssColor::Rgba(c) => c,
            CssColor::CurrentColor => current_color,
        }
    }
}

// ─── Enums ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Display {
    Block,
    Inline,
    InlineBlock,
    Flex,
    Grid,
    None,
}

impl std::fmt::Display for Display {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Display::Block => write!(f, "block"),
            Display::Inline => write!(f, "inline"),
            Display::InlineBlock => write!(f, "inline-block"),
            Display::Flex => write!(f, "flex"),
            Display::Grid => write!(f, "grid"),
            Display::None => write!(f, "none"),
        }
    }
}

// ─── Grid Types ──────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum GridTrack {
    Auto,
    Fr(f32),
    Px(f32),
    Percent(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GridPlacement {
    Auto,
    Line(i32),
    Span(i32),
}

// ─── Animation Types ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AnimationTimingFunction {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
    EaseInOut,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnimationSpec {
    pub name: String,
    pub duration: f32, // seconds
    pub timing_function: AnimationTimingFunction,
    pub iteration_count: f32, // f32::INFINITY for infinite
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

/// CSS box-sizing property: determines how width/height are measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxSizing {
    /// Width/height specify content box (default).
    ContentBox,
    /// Width/height include padding and border.
    BorderBox,
}

// ─── Edges (padding/border/gap) ──────────────────────────────────
 
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

/// Margin edges for box model (None represents "auto").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margin {
    pub top: Option<f32>,
    pub right: Option<f32>,
    pub bottom: Option<f32>,
    pub left: Option<f32>,
}

impl Margin {
    pub const ZERO: Margin = Margin {
        top: Some(0.0),
        right: Some(0.0),
        bottom: Some(0.0),
        left: Some(0.0),
    };

    pub const AUTO: Margin = Margin {
        top: None,
        right: None,
        bottom: None,
        left: None,
    };

    pub fn uniform(v: f32) -> Self {
        Margin {
            top: Some(v),
            right: Some(v),
            bottom: Some(v),
            left: Some(v),
        }
    }
}

impl std::fmt::Display for Margin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fn fmt_opt(o: Option<f32>) -> String {
            match o {
                Some(v) => format!("{}px", v),
                None => "auto".to_string(),
            }
        }
        if self.top == self.right && self.right == self.bottom && self.bottom == self.left {
            write!(f, "{}", fmt_opt(self.top))
        } else {
            write!(
                f,
                "{} {} {} {}",
                fmt_opt(self.top),
                fmt_opt(self.right),
                fmt_opt(self.bottom),
                fmt_opt(self.left)
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
    pub width: Option<f32>,     // None = auto
    pub height: Option<f32>,    // None = auto
    pub box_sizing: BoxSizing,  // content-box | border-box

    // Margins (px or auto)
    pub margin: Margin,
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

    // Grid
    pub grid_template_columns: Vec<GridTrack>,
    pub grid_template_rows: Vec<GridTrack>,
    pub grid_column: GridPlacement,
    pub grid_row: GridPlacement,
    pub grid_gap: Edges,

    // Animation
    pub animation_name: String,
    pub animation_duration: f32,
    pub animation_timing_function: AnimationTimingFunction,
    pub animation_iteration_count: f32,

    // CSS Variables
    pub variables: std::collections::HashMap<String, String>,
}

impl Default for ComputedStyle {
    /// CSS initial values per spec
    fn default() -> Self {
        ComputedStyle {
            display: Display::Inline,
            position: Position::Static,
            width: None,
            height: None,
            box_sizing: BoxSizing::ContentBox,
            margin: Margin::ZERO,
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
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            grid_column: GridPlacement::Auto,
            grid_row: GridPlacement::Auto,
            grid_gap: Edges::ZERO,
            animation_name: "none".to_string(),
            animation_duration: 0.0,
            animation_timing_function: AnimationTimingFunction::Ease,
            animation_iteration_count: 1.0,
            variables: std::collections::HashMap::new(),
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
            PropertyId::BoxSizing => match self.box_sizing {
                BoxSizing::ContentBox => "content-box".to_string(),
                BoxSizing::BorderBox => "border-box".to_string(),
            },
            PropertyId::MarginTop => match self.margin.top {
                Some(v) => format!("{}px", v),
                None => "auto".to_string(),
            },
            PropertyId::MarginRight => match self.margin.right {
                Some(v) => format!("{}px", v),
                None => "auto".to_string(),
            },
            PropertyId::MarginBottom => match self.margin.bottom {
                Some(v) => format!("{}px", v),
                None => "auto".to_string(),
            },
            PropertyId::MarginLeft => match self.margin.left {
                Some(v) => format!("{}px", v),
                None => "auto".to_string(),
            },
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
            PropertyId::GridTemplateColumns => "<grid-tracks>".to_string(),
            PropertyId::GridTemplateRows => "<grid-tracks>".to_string(),
            PropertyId::GridColumn => "<grid-placement>".to_string(),
            PropertyId::GridRow => "<grid-placement>".to_string(),
            PropertyId::GridGap => {
                let g = self.grid_gap;
                if (g.top - g.right).abs() < 1e-4
                    && (g.top - g.bottom).abs() < 1e-4
                    && (g.top - g.left).abs() < 1e-4
                {
                    format!("{}px", g.top)
                } else if (g.top - g.bottom).abs() < 1e-4 && (g.left - g.right).abs() < 1e-4 {
                    format!("{}px {}px", g.top, g.right)
                } else {
                    format!("{}px {}px {}px {}px", g.top, g.right, g.bottom, g.left)
                }
            }
            PropertyId::AnimationName => self.animation_name.clone(),
            PropertyId::AnimationDuration => format!("{}s", self.animation_duration),
            PropertyId::AnimationTimingFunction => "<timing-function>".to_string(),
            PropertyId::AnimationIterationCount => format!("{}", self.animation_iteration_count),
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
            PropertyId::BoxSizing => self.box_sizing = parse_box_sizing(value),
            PropertyId::MarginTop => {
                self.margin.top = parse_optional_length(value, self.font_size, root_font_size)
            }
            PropertyId::MarginRight => {
                self.margin.right = parse_optional_length(value, self.font_size, root_font_size)
            }
            PropertyId::MarginBottom => {
                self.margin.bottom = parse_optional_length(value, self.font_size, root_font_size)
            }
            PropertyId::MarginLeft => {
                self.margin.left = parse_optional_length(value, self.font_size, root_font_size)
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

            PropertyId::FontWeight => {
                self.font_weight = parse_font_weight_relative(value, self.font_weight)
            }
            PropertyId::TextAlign => self.text_align = parse_text_align(value),
            PropertyId::LineHeight => {
                self.line_height = parse_line_height(value, self.font_size, root_font_size)
            }
            PropertyId::GridTemplateColumns => {
                self.grid_template_columns = parse_grid_tracks(value)
            }
            PropertyId::GridTemplateRows => self.grid_template_rows = parse_grid_tracks(value),
            PropertyId::GridColumn => self.grid_column = parse_grid_placement(value),
            PropertyId::GridRow => self.grid_row = parse_grid_placement(value),
            PropertyId::GridGap => {
                self.grid_gap = parse_gap(value, self.font_size, root_font_size)
            }
            PropertyId::AnimationName => self.animation_name = value.trim().to_string(),
            PropertyId::AnimationDuration => self.animation_duration = parse_time(value),
            PropertyId::AnimationTimingFunction => {
                self.animation_timing_function = parse_timing_function(value)
            }
            PropertyId::AnimationIterationCount => {
                self.animation_iteration_count = parse_iteration_count(value)
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

/// Parse a CSS box-sizing keyword.
pub fn parse_box_sizing(value: &str) -> BoxSizing {
    match value.trim().to_ascii_lowercase().as_str() {
        "border-box" => BoxSizing::BorderBox,
        _ => BoxSizing::ContentBox,
    }
}

/// Try to parse a single CSS color token (without fallback).
/// Returns Some(Color) on recognized syntax, None otherwise.
pub fn try_parse_color(value: &str) -> Option<Color> {
    let s = value.trim().to_ascii_lowercase();

    if s == "transparent" || s == "none" {
        return Some(Color::TRANSPARENT);
    }

    if let Some(c) = named_color(&s) {
        return Some(c);
    }

    if let Some(hex) = s.strip_prefix('#') {
        return try_parse_hex_color(hex);
    }

    if let Some(inner) = s.strip_prefix("rgb(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            return Some(Color::rgb(r, g, b));
        }
    }

    if let Some(inner) = s.strip_prefix("rgba(").and_then(|s| s.strip_suffix(')')) {
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 4 {
            let r = parts[0].trim().parse::<u8>().ok()?;
            let g = parts[1].trim().parse::<u8>().ok()?;
            let b = parts[2].trim().parse::<u8>().ok()?;
            let a_str = parts[3].trim();
            let a = {
                let a_float = a_str.parse::<f32>().ok()?;
                if a_float > 1.0 {
                    (a_float.min(255.0)) as u8
                } else {
                    (a_float.clamp(0.0, 1.0) * 255.0) as u8
                }
            };
            return Some(Color::new(r, g, b, a));
        }
    }

    None
}

/// Try to parse a CSS color or `currentColor` keyword from a value string
/// (including scanning whitespace-separated background shorthand tokens).
pub fn try_parse_css_color(value: &str) -> Option<CssColor> {
    let s = value.trim();
    if s.eq_ignore_ascii_case("currentcolor") {
        return Some(CssColor::CurrentColor);
    }
    if let Some(c) = try_parse_color(s) {
        return Some(CssColor::Rgba(c));
    }
    if s.contains(' ') {
        for token in s.split_whitespace() {
            if token.starts_with("url(")
                || matches!(
                    token,
                    "no-repeat"
                        | "repeat"
                        | "repeat-x"
                        | "repeat-y"
                        | "center"
                        | "top"
                        | "bottom"
                        | "left"
                        | "right"
                        | "cover"
                        | "contain"
                        | "fixed"
                        | "scroll"
                        | "local"
                        | "auto"
                        | "/"
                )
            {
                continue;
            }
            if token.eq_ignore_ascii_case("currentcolor") {
                return Some(CssColor::CurrentColor);
            }
            if let Some(c) = try_parse_color(token) {
                return Some(CssColor::Rgba(c));
            }
        }
    }
    None
}

/// Parse a CSS color value.
/// Supports: named colors, #hex (3, 6, 8 digit), rgb(r,g,b), rgba(r,g,b,a),
/// and multi-token background shorthand values (extracts the recognized color token).
pub fn parse_color(value: &str) -> Color {
    if let Some(css_color) = try_parse_css_color(value) {
        match css_color {
            CssColor::Rgba(c) => c,
            CssColor::CurrentColor => Color::BLACK,
        }
    } else {
        Color::TRANSPARENT
    }
}

/// Parse a CSS color or `currentColor` keyword with fallback.
pub fn parse_css_color(value: &str) -> CssColor {
    try_parse_css_color(value).unwrap_or(CssColor::Rgba(Color::TRANSPARENT))
}

/// Parse a hex color string (without the # prefix).
fn try_parse_hex_color(hex: &str) -> Option<Color> {
    let valid_hex = hex.chars().all(|c| c.is_ascii_hexdigit());
    if !valid_hex {
        return None;
    }
    match hex.len() {
        3 => {
            let r = u8_from_hex_char(hex.as_bytes()[0]);
            let g = u8_from_hex_char(hex.as_bytes()[1]);
            let b = u8_from_hex_char(hex.as_bytes()[2]);
            Some(Color::rgb(r * 17, g * 17, b * 17))
        }
        6 => {
            let r = u8_from_hex_pair(hex.as_bytes()[0], hex.as_bytes()[1]);
            let g = u8_from_hex_pair(hex.as_bytes()[2], hex.as_bytes()[3]);
            let b = u8_from_hex_pair(hex.as_bytes()[4], hex.as_bytes()[5]);
            Some(Color::rgb(r, g, b))
        }
        8 => {
            let r = u8_from_hex_pair(hex.as_bytes()[0], hex.as_bytes()[1]);
            let g = u8_from_hex_pair(hex.as_bytes()[2], hex.as_bytes()[3]);
            let b = u8_from_hex_pair(hex.as_bytes()[4], hex.as_bytes()[5]);
            let a = u8_from_hex_pair(hex.as_bytes()[6], hex.as_bytes()[7]);
            Some(Color::new(r, g, b, a))
        }
        _ => None,
    }
}

pub fn parse_hex_color(hex: &str) -> Color {
    try_parse_hex_color(hex).unwrap_or(Color::BLACK)
}

/// Parse a shorthand margin value into 4 optional edge values (supporting "auto").
pub fn parse_margin(value: &str, em_base: f32, rem_base: f32) -> Margin {
    let parts: Vec<&str> = value.split_whitespace().collect();
    match parts.len() {
        1 => {
            let v = parse_optional_length(parts[0], em_base, rem_base);
            Margin {
                top: v,
                right: v,
                bottom: v,
                left: v,
            }
        }
        2 => {
            let v = parse_optional_length(parts[0], em_base, rem_base);
            let h = parse_optional_length(parts[1], em_base, rem_base);
            Margin {
                top: v,
                right: h,
                bottom: v,
                left: h,
            }
        }
        3 => {
            let top = parse_optional_length(parts[0], em_base, rem_base);
            let h = parse_optional_length(parts[1], em_base, rem_base);
            let bottom = parse_optional_length(parts[2], em_base, rem_base);
            Margin {
                top,
                right: h,
                bottom,
                left: h,
            }
        }
        4 => {
            let top = parse_optional_length(parts[0], em_base, rem_base);
            let right = parse_optional_length(parts[1], em_base, rem_base);
            let bottom = parse_optional_length(parts[2], em_base, rem_base);
            let left = parse_optional_length(parts[3], em_base, rem_base);
            Margin {
                top,
                right,
                bottom,
                left,
            }
        }
        _ => Margin::ZERO,
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
        "flex" => Display::Flex,
        "grid" => Display::Grid,
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

/// Parse a CSS font-weight value relative to parent weight.
pub fn parse_font_weight_relative(value: &str, parent_weight: f32) -> f32 {
    match value.trim().to_ascii_lowercase().as_str() {
        "normal" => 400.0,
        "bold" => 700.0,
        "lighter" => (parent_weight - 100.0).max(100.0),
        "bolder" => (parent_weight + 100.0).min(900.0),
        other => other.parse::<f32>().unwrap_or(400.0),
    }
}

/// Parse a CSS font-weight value.
pub fn parse_font_weight(value: &str) -> f32 {
    parse_font_weight_relative(value, 400.0)
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

/// Parse a CSS border shorthand value (e.g. "1px solid #bae6fd") into (width, style, color) parts.
pub fn parse_border_shorthand(value: &str) -> (Option<String>, Option<String>, Option<String>) {
    let mut width = None;
    let mut style = None;
    let mut color = None;

    for part in value.split_whitespace() {
        let p_lower = part.to_ascii_lowercase();
        if p_lower.ends_with("px")
            || p_lower.ends_with("em")
            || p_lower.ends_with("rem")
            || p_lower.chars().all(|c| c.is_ascii_digit() || c == '.')
        {
            width = Some(part.to_string());
        } else if matches!(p_lower.as_str(), "none" | "solid" | "dashed" | "dotted") {
            style = Some(part.to_string());
        } else {
            color = Some(part.to_string());
        }
    }
    (width, style, color)
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

/// Parse a CSS gap / grid-gap value.
/// Per CSS Box Alignment Module Level 3 §8:
/// Accepts 1 or 2 values: `row-gap column-gap?`
/// 1 value sets both row and column gap.
/// 2 values sets row-gap (top/bottom) and column-gap (left/right).
pub fn parse_gap(value: &str, em_base: f32, rem_base: f32) -> Edges {
    let parts: Vec<&str> = value.split_whitespace().collect();
    match parts.len() {
        1 => {
            let v = parse_length(parts[0], em_base, rem_base);
            Edges::uniform(v)
        }
        2 => {
            let row = parse_length(parts[0], em_base, rem_base);
            let col = parse_length(parts[1], em_base, rem_base);
            Edges {
                top: row,
                right: col,
                bottom: row,
                left: col,
            }
        }
        _ => Edges::ZERO,
    }
}

pub fn parse_grid_tracks(value: &str) -> Vec<GridTrack> {
    let mut tracks = Vec::new();
    for part in value.split_whitespace() {
        if part == "auto" {
            tracks.push(GridTrack::Auto);
        } else if let Some(num) = part.strip_suffix("fr") {
            if let Ok(val) = num.parse::<f32>() {
                tracks.push(GridTrack::Fr(val));
            }
        } else if let Some(num) = part.strip_suffix("px") {
            if let Ok(val) = num.parse::<f32>() {
                tracks.push(GridTrack::Px(val));
            }
        } else if let Some(num) = part.strip_suffix("%") {
            if let Ok(val) = num.parse::<f32>() {
                tracks.push(GridTrack::Percent(val));
            }
        } else if part.parse::<f32>().is_ok_and(|val| val == 0.0) {
            tracks.push(GridTrack::Px(0.0));
        }
    }
    tracks
}

pub fn parse_grid_placement(value: &str) -> GridPlacement {
    let s = value.trim();
    if s == "auto" {
        return GridPlacement::Auto;
    }
    if let Some(val) = s
        .strip_prefix("span ")
        .and_then(|span| span.trim().parse::<i32>().ok())
    {
        return GridPlacement::Span(val);
    }
    if let Ok(val) = s.parse::<i32>() {
        return GridPlacement::Line(val);
    }
    GridPlacement::Auto
}

pub fn parse_time(value: &str) -> f32 {
    let s = value.trim();
    if let Some(num) = s.strip_suffix("ms") {
        num.parse::<f32>().unwrap_or(0.0) / 1000.0
    } else if let Some(num) = s.strip_suffix("s") {
        num.parse::<f32>().unwrap_or(0.0)
    } else {
        0.0
    }
}

pub fn parse_timing_function(value: &str) -> AnimationTimingFunction {
    match value.trim() {
        "linear" => AnimationTimingFunction::Linear,
        "ease" => AnimationTimingFunction::Ease,
        "ease-in" => AnimationTimingFunction::EaseIn,
        "ease-out" => AnimationTimingFunction::EaseOut,
        "ease-in-out" => AnimationTimingFunction::EaseInOut,
        _ => AnimationTimingFunction::Ease,
    }
}

pub fn parse_iteration_count(value: &str) -> f32 {
    let s = value.trim();
    if s == "infinite" {
        f32::INFINITY
    } else {
        s.parse::<f32>().unwrap_or(1.0)
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
        assert_eq!(s.margin, Margin::ZERO);
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

    #[test]
    fn test_box_sizing_parsing() {
        assert_eq!(parse_box_sizing("border-box"), BoxSizing::BorderBox);
        assert_eq!(parse_box_sizing("content-box"), BoxSizing::ContentBox);
        assert_eq!(parse_box_sizing("BORDER-BOX"), BoxSizing::BorderBox);
        assert_eq!(parse_box_sizing("unknown"), BoxSizing::ContentBox);
    }

    #[test]
    fn test_box_sizing_default_is_content_box() {
        let s = ComputedStyle::default();
        assert_eq!(s.box_sizing, BoxSizing::ContentBox);
    }

    #[test]
    fn test_css_color_and_currentcolor() {
        assert_eq!(
            parse_css_color("currentColor"),
            CssColor::CurrentColor
        );
        assert_eq!(
            parse_css_color("currentcolor"),
            CssColor::CurrentColor
        );
        let resolved = parse_css_color("currentColor").resolve(Color::rgb(10, 20, 30));
        assert_eq!(resolved, Color::rgb(10, 20, 30));

        // Explicit rgba(1, 1, 1, 0) is parsed accurately and not confused with anything
        let explicit = parse_color("rgba(1, 1, 1, 0)");
        assert_eq!(explicit, Color::new(1, 1, 1, 0));
    }

    #[test]
    fn test_transparent_keyword() {
        assert_eq!(parse_color("transparent"), Color::TRANSPARENT);
        assert_eq!(parse_color("TRANSPARENT"), Color::TRANSPARENT);
    }

    #[test]
    fn test_background_shorthand_color_extraction() {
        // Should find the color in a multi-token background value
        let c = parse_color("no-repeat center blue");
        assert_eq!(c, Color::rgb(0, 0, 255));

        // Should find hex color in shorthand
        let c2 = parse_color("center #ff0000");
        assert_eq!(c2, Color::rgb(255, 0, 0));

        // Malformed or non-color fragments should not be treated as color matches
        assert_eq!(try_parse_color("rgb(255,"), None);
        assert_eq!(try_parse_color("foo"), None);

        // Multi-token with malformed parts should still extract the valid color
        let c3 = parse_color("url(foo) rgb(255, invalid center green");
        assert_eq!(c3, Color::rgb(0, 128, 0));
    }

    #[test]
    fn test_margin_shorthand_and_auto() {
        let m1 = parse_margin("10px auto", 16.0, 16.0);
        assert_eq!(m1.top, Some(10.0));
        assert_eq!(m1.right, None);
        assert_eq!(m1.bottom, Some(10.0));
        assert_eq!(m1.left, None);

        let m2 = parse_margin("0", 16.0, 16.0);
        assert_eq!(m2.top, Some(0.0));
        assert_eq!(m2.right, Some(0.0));
        assert_eq!(m2.bottom, Some(0.0));
        assert_eq!(m2.left, Some(0.0));
    }
}
