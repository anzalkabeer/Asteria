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

    /// Multiply the alpha channel by a factor in [0.0, 1.0].
    pub fn with_alpha_multiplier(self, multiplier: f32) -> Self {
        let new_a = ((self.a as f32) * multiplier.clamp(0.0, 1.0)).round() as u8;
        Color {
            r: self.r,
            g: self.g,
            b: self.b,
            a: new_a,
        }
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
    // Table formatting context
    Table,
    TableRow,
    TableCell,
    TableRowGroup,
    TableHeaderGroup,
    TableFooterGroup,
    TableCaption,
    InlineTable,
    TableColumn,
    TableColumnGroup,
}

impl Display {
    /// Returns true if this display type establishes a table formatting context.
    pub fn is_table_display(self) -> bool {
        matches!(
            self,
            Display::Table
                | Display::InlineTable
                | Display::TableRow
                | Display::TableCell
                | Display::TableRowGroup
                | Display::TableHeaderGroup
                | Display::TableFooterGroup
                | Display::TableCaption
                | Display::TableColumn
                | Display::TableColumnGroup
        )
    }
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
            Display::Table => write!(f, "table"),
            Display::TableRow => write!(f, "table-row"),
            Display::TableCell => write!(f, "table-cell"),
            Display::TableRowGroup => write!(f, "table-row-group"),
            Display::TableHeaderGroup => write!(f, "table-header-group"),
            Display::TableFooterGroup => write!(f, "table-footer-group"),
            Display::TableCaption => write!(f, "table-caption"),
            Display::InlineTable => write!(f, "inline-table"),
            Display::TableColumn => write!(f, "table-column"),
            Display::TableColumnGroup => write!(f, "table-column-group"),
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
    MinMax(Box<GridTrack>, Box<GridTrack>),
}

/// A single grid line reference: auto, a numbered line, or a span count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridLine {
    Auto,
    Line(i32),
    Span(i32),
}

impl std::fmt::Display for GridLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GridLine::Auto => write!(f, "auto"),
            GridLine::Line(l) => write!(f, "{}", l),
            GridLine::Span(s) => write!(f, "span {}", s),
        }
    }
}

/// A grid placement consisting of start and end lines.
/// For shorthand values like `grid-column: 2`, end defaults to Auto.
/// For `grid-column: 1 / -1`, both start and end are populated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPlacement {
    pub start: GridLine,
    pub end: GridLine,
}

impl std::fmt::Display for GridPlacement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.end == GridLine::Auto {
            write!(f, "{}", self.start)
        } else {
            write!(f, "{} / {}", self.start, self.end)
        }
    }
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

// ─── Table Types ─────────────────────────────────────────────────

/// CSS border-collapse property for tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderCollapse {
    Separate,
    Collapse,
}

impl std::fmt::Display for BorderCollapse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BorderCollapse::Separate => write!(f, "separate"),
            BorderCollapse::Collapse => write!(f, "collapse"),
        }
    }
}

/// CSS vertical-align property for table cells and inline elements.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    Baseline,
    Top,
    Middle,
    Bottom,
}

impl std::fmt::Display for VerticalAlign {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VerticalAlign::Baseline => write!(f, "baseline"),
            VerticalAlign::Top => write!(f, "top"),
            VerticalAlign::Middle => write!(f, "middle"),
            VerticalAlign::Bottom => write!(f, "bottom"),
        }
    }
}

// ─── Visual Rendering Types ─────────────────────────────────────

/// CSS overflow property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

impl std::fmt::Display for Overflow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Overflow::Visible => write!(f, "visible"),
            Overflow::Hidden => write!(f, "hidden"),
            Overflow::Scroll => write!(f, "scroll"),
            Overflow::Auto => write!(f, "auto"),
        }
    }
}

/// CSS border-radius for one corner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BorderRadius {
    pub top_left: f32,
    pub top_right: f32,
    pub bottom_right: f32,
    pub bottom_left: f32,
}

impl BorderRadius {
    pub const ZERO: BorderRadius = BorderRadius {
        top_left: 0.0,
        top_right: 0.0,
        bottom_right: 0.0,
        bottom_left: 0.0,
    };

    pub fn uniform(r: f32) -> Self {
        BorderRadius {
            top_left: r,
            top_right: r,
            bottom_right: r,
            bottom_left: r,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.top_left == 0.0
            && self.top_right == 0.0
            && self.bottom_right == 0.0
            && self.bottom_left == 0.0
    }
}

impl std::fmt::Display for BorderRadius {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.top_left == self.top_right
            && self.top_right == self.bottom_right
            && self.bottom_right == self.bottom_left
        {
            write!(f, "{}px", self.top_left)
        } else {
            write!(
                f,
                "{}px {}px {}px {}px",
                self.top_left, self.top_right, self.bottom_right, self.bottom_left
            )
        }
    }
}

/// A single CSS box-shadow declaration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoxShadow {
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur_radius: f32,
    pub spread_radius: f32,
    pub color: Color,
    pub inset: bool,
}

impl BoxShadow {
    pub fn none() -> Vec<BoxShadow> {
        Vec::new()
    }
}

// ─── Flexbox Types ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

impl std::fmt::Display for FlexDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlexDirection::Row => write!(f, "row"),
            FlexDirection::RowReverse => write!(f, "row-reverse"),
            FlexDirection::Column => write!(f, "column"),
            FlexDirection::ColumnReverse => write!(f, "column-reverse"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

impl std::fmt::Display for FlexWrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlexWrap::NoWrap => write!(f, "nowrap"),
            FlexWrap::Wrap => write!(f, "wrap"),
            FlexWrap::WrapReverse => write!(f, "wrap-reverse"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JustifyContent {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    Start,
    End,
}

impl std::fmt::Display for JustifyContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JustifyContent::FlexStart => write!(f, "flex-start"),
            JustifyContent::FlexEnd => write!(f, "flex-end"),
            JustifyContent::Center => write!(f, "center"),
            JustifyContent::SpaceBetween => write!(f, "space-between"),
            JustifyContent::SpaceAround => write!(f, "space-around"),
            JustifyContent::SpaceEvenly => write!(f, "space-evenly"),
            JustifyContent::Start => write!(f, "start"),
            JustifyContent::End => write!(f, "end"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignItems {
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

impl std::fmt::Display for AlignItems {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlignItems::Stretch => write!(f, "stretch"),
            AlignItems::FlexStart => write!(f, "flex-start"),
            AlignItems::FlexEnd => write!(f, "flex-end"),
            AlignItems::Center => write!(f, "center"),
            AlignItems::Baseline => write!(f, "baseline"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignSelf {
    Auto,
    Stretch,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
}

impl std::fmt::Display for AlignSelf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlignSelf::Auto => write!(f, "auto"),
            AlignSelf::Stretch => write!(f, "stretch"),
            AlignSelf::FlexStart => write!(f, "flex-start"),
            AlignSelf::FlexEnd => write!(f, "flex-end"),
            AlignSelf::Center => write!(f, "center"),
            AlignSelf::Baseline => write!(f, "baseline"),
        }
    }
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
    pub z_index: Option<i32>, // None = auto
    pub width: LengthOrPercentage,
    pub height: LengthOrPercentage,
    pub box_sizing: BoxSizing, // content-box | border-box

    // Insets (Positioning)
    pub top: LengthOrPercentage,
    pub right: LengthOrPercentage,
    pub bottom: LengthOrPercentage,
    pub left: LengthOrPercentage,

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

    // Flexbox
    pub flex_direction: FlexDirection,
    pub flex_wrap: FlexWrap,
    pub justify_content: JustifyContent,
    pub align_items: AlignItems,
    pub align_self: AlignSelf,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub flex_basis: LengthOrPercentage,

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

    // Table
    pub border_collapse: BorderCollapse,
    pub border_spacing: f32,
    pub vertical_align: VerticalAlign,

    // Visual rendering
    pub opacity: f32,
    pub border_radius: BorderRadius,
    pub box_shadow: Vec<BoxShadow>,
    pub overflow: Overflow,

    // CSS Variables
    pub variables: std::collections::HashMap<String, String>,
}

impl Default for ComputedStyle {
    /// CSS initial values per spec
    fn default() -> Self {
        ComputedStyle {
            display: Display::Inline,
            position: Position::Static,
            z_index: None,
            width: LengthOrPercentage::Auto,
            height: LengthOrPercentage::Auto,
            box_sizing: BoxSizing::ContentBox,
            top: LengthOrPercentage::Auto,
            right: LengthOrPercentage::Auto,
            bottom: LengthOrPercentage::Auto,
            left: LengthOrPercentage::Auto,
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
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::NoWrap,
            justify_content: JustifyContent::FlexStart,
            align_items: AlignItems::Stretch,
            align_self: AlignSelf::Auto,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: LengthOrPercentage::Auto,
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            grid_column: GridPlacement {
                start: GridLine::Auto,
                end: GridLine::Auto,
            },
            grid_row: GridPlacement {
                start: GridLine::Auto,
                end: GridLine::Auto,
            },
            grid_gap: Edges::ZERO,
            animation_name: "none".to_string(),
            animation_duration: 0.0,
            animation_timing_function: AnimationTimingFunction::Ease,
            animation_iteration_count: 1.0,
            // Table
            border_collapse: BorderCollapse::Separate,
            border_spacing: 0.0,
            vertical_align: VerticalAlign::Baseline,
            // Visual rendering
            opacity: 1.0,
            border_radius: BorderRadius::ZERO,
            box_shadow: Vec::new(),
            overflow: Overflow::Visible,
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
            PropertyId::ZIndex => match self.z_index {
                Some(z) => format!("{}", z),
                None => "auto".to_string(),
            },
            PropertyId::Width => format_lop(&self.width),
            PropertyId::Height => format_lop(&self.height),
            PropertyId::BoxSizing => match self.box_sizing {
                BoxSizing::ContentBox => "content-box".to_string(),
                BoxSizing::BorderBox => "border-box".to_string(),
            },
            PropertyId::Top => format_lop(&self.top),
            PropertyId::Right => format_lop(&self.right),
            PropertyId::Bottom => format_lop(&self.bottom),
            PropertyId::Left => format_lop(&self.left),
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
            PropertyId::FlexDirection => format!("{}", self.flex_direction),
            PropertyId::FlexWrap => format!("{}", self.flex_wrap),
            PropertyId::JustifyContent => format!("{}", self.justify_content),
            PropertyId::AlignItems => format!("{}", self.align_items),
            PropertyId::AlignSelf => format!("{}", self.align_self),
            PropertyId::FlexGrow => format!("{}", self.flex_grow),
            PropertyId::FlexShrink => format!("{}", self.flex_shrink),
            PropertyId::FlexBasis => format_lop(&self.flex_basis),
            PropertyId::GridTemplateColumns => "<grid-tracks>".to_string(),
            PropertyId::GridTemplateRows => "<grid-tracks>".to_string(),
            PropertyId::GridColumn => "<grid-placement>".to_string(),
            PropertyId::GridRow => "<grid-placement>".to_string(),
            PropertyId::GridColumnStart => format!("{}", self.grid_column.start),
            PropertyId::GridColumnEnd => format!("{}", self.grid_column.end),
            PropertyId::GridRowStart => format!("{}", self.grid_row.start),
            PropertyId::GridRowEnd => format!("{}", self.grid_row.end),
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
            PropertyId::RowGap => format!("{}px", self.grid_gap.top),
            PropertyId::ColumnGap => format!("{}px", self.grid_gap.left),
            PropertyId::AnimationName => self.animation_name.clone(),
            PropertyId::AnimationDuration => format!("{}s", self.animation_duration),
            PropertyId::AnimationTimingFunction => "<timing-function>".to_string(),
            PropertyId::AnimationIterationCount => format!("{}", self.animation_iteration_count),
            // Table
            PropertyId::BorderCollapse => format!("{}", self.border_collapse),
            PropertyId::BorderSpacing => format!("{}px", self.border_spacing),
            PropertyId::VerticalAlign => format!("{}", self.vertical_align),
            // Visual rendering
            PropertyId::Opacity => format!("{}", self.opacity),
            PropertyId::BorderRadius => format!("{}", self.border_radius),
            PropertyId::BoxShadow => {
                if self.box_shadow.is_empty() {
                    "none".to_string()
                } else {
                    "<box-shadow>".to_string()
                }
            }
            PropertyId::Overflow => format!("{}", self.overflow),
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
            PropertyId::ZIndex => {
                if let Some(zi) = try_parse_z_index(value) {
                    self.z_index = match zi {
                        ZIndex::Auto => None,
                        ZIndex::Integer(n) => Some(n),
                    };
                }
            }
            PropertyId::Width => {
                self.width = parse_length_or_percentage(value, self.font_size, root_font_size)
            }
            PropertyId::Height => {
                self.height = parse_length_or_percentage(value, self.font_size, root_font_size)
            }
            PropertyId::BoxSizing => self.box_sizing = parse_box_sizing(value),
            PropertyId::Top => {
                self.top = parse_length_or_percentage(value, self.font_size, root_font_size)
            }
            PropertyId::Right => {
                self.right = parse_length_or_percentage(value, self.font_size, root_font_size)
            }
            PropertyId::Bottom => {
                self.bottom = parse_length_or_percentage(value, self.font_size, root_font_size)
            }
            PropertyId::Left => {
                self.left = parse_length_or_percentage(value, self.font_size, root_font_size)
            }
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
            PropertyId::FlexDirection => self.flex_direction = parse_flex_direction(value),
            PropertyId::FlexWrap => self.flex_wrap = parse_flex_wrap(value),
            PropertyId::JustifyContent => self.justify_content = parse_justify_content(value),
            PropertyId::AlignItems => self.align_items = parse_align_items(value),
            PropertyId::AlignSelf => self.align_self = parse_align_self(value),
            PropertyId::FlexGrow => self.flex_grow = parse_flex_grow(value),
            PropertyId::FlexShrink => self.flex_shrink = parse_flex_shrink(value),
            PropertyId::FlexBasis => {
                self.flex_basis = parse_length_or_percentage(value, self.font_size, root_font_size)
            }
            PropertyId::GridTemplateColumns => {
                self.grid_template_columns = parse_grid_tracks(value)
            }
            PropertyId::GridTemplateRows => self.grid_template_rows = parse_grid_tracks(value),
            PropertyId::GridColumn => self.grid_column = parse_grid_placement(value),
            PropertyId::GridRow => self.grid_row = parse_grid_placement(value),
            PropertyId::GridColumnStart => self.grid_column.start = parse_grid_line(value),
            PropertyId::GridColumnEnd => self.grid_column.end = parse_grid_line(value),
            PropertyId::GridRowStart => self.grid_row.start = parse_grid_line(value),
            PropertyId::GridRowEnd => self.grid_row.end = parse_grid_line(value),
            PropertyId::GridGap => self.grid_gap = parse_gap(value, self.font_size, root_font_size),
            PropertyId::RowGap => {
                let v = parse_length(value, self.font_size, root_font_size);
                self.grid_gap.top = v;
                self.grid_gap.bottom = v;
            }
            PropertyId::ColumnGap => {
                let v = parse_length(value, self.font_size, root_font_size);
                self.grid_gap.left = v;
                self.grid_gap.right = v;
            }
            PropertyId::AnimationName => self.animation_name = value.trim().to_string(),
            PropertyId::AnimationDuration => self.animation_duration = parse_time(value),
            PropertyId::AnimationTimingFunction => {
                self.animation_timing_function = parse_timing_function(value)
            }
            PropertyId::AnimationIterationCount => {
                self.animation_iteration_count = parse_iteration_count(value)
            }
            // Table
            PropertyId::BorderCollapse => self.border_collapse = parse_border_collapse(value),
            PropertyId::BorderSpacing => {
                let first = value.split_whitespace().next().unwrap_or(value);
                self.border_spacing = parse_length(first, self.font_size, root_font_size);
            }
            PropertyId::VerticalAlign => self.vertical_align = parse_vertical_align(value),
            // Visual rendering
            PropertyId::Opacity => self.opacity = parse_opacity(value),
            PropertyId::BorderRadius => {
                self.border_radius = parse_border_radius(value, self.font_size, root_font_size)
            }
            PropertyId::BoxShadow => {
                self.box_shadow = parse_box_shadow(value, self.font_size, root_font_size)
            }
            PropertyId::Overflow => self.overflow = parse_overflow(value),
        }
    }
}

// ─── Parsing Functions ───────────────────────────────────────────

/// Strip a function call like `calc(...)` or `minmax(...)` from a value string.
/// Returns the inner content between the parentheses if the value starts with `name(`.
fn strip_function_call<'a>(value: &'a str, name: &str) -> Option<&'a str> {
    let lower = value.to_ascii_lowercase();
    if !lower.starts_with(name) {
        return None;
    }
    let rest = &value[name.len()..];
    let rest = rest.trim_start();
    if !rest.starts_with('(') {
        return None;
    }
    let inner = &rest[1..];
    // Find matching close paren
    let mut depth = 1;
    let mut end = 0;
    for (i, ch) in inner.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = i;
                    break;
                }
            }
            _ => {}
        }
    }
    if depth == 0 {
        Some(inner[..end].trim())
    } else {
        None
    }
}

/// Tokenize and evaluate a `calc()` expression returning `(px_value, percentage_value)`.
/// Supports `+`, `-`, `*`, `/` with correct operator precedence, nested parentheses,
/// and mixed units (`px`, `em`, `rem`, `%`, unitless numbers).
///
/// Returns `None` if the expression is malformed.
pub fn evaluate_calc(expr: &str, em_base: f32, rem_base: f32) -> Option<(f32, f32)> {
    // Tokenize the expression
    let tokens = calc_tokenize(expr, em_base, rem_base)?;
    let mut pos = 0;
    let result = calc_parse_expr(&tokens, &mut pos)?;
    if pos != tokens.len() {
        return None; // Unexpected trailing tokens
    }
    Some(result)
}

/// Calc token types
#[derive(Debug, Clone, Copy)]
enum CalcToken {
    /// A value with px and percentage components
    Value(f32, f32), // (px, percentage)
    Plus,
    Minus,
    Star,
    Slash,
    OpenParen,
    CloseParen,
}

/// Tokenize a calc expression into CalcTokens.
fn calc_tokenize(expr: &str, em_base: f32, rem_base: f32) -> Option<Vec<CalcToken>> {
    let mut tokens = Vec::new();
    let bytes = expr.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b' ' | b'\t' | b'\n' | b'\r' => {
                i += 1;
            }
            b'+' => {
                tokens.push(CalcToken::Plus);
                i += 1;
            }
            b'-' => {
                // Check if this is a negative number or a minus operator
                let is_unary = tokens.is_empty()
                    || matches!(
                        tokens.last(),
                        Some(CalcToken::Plus)
                            | Some(CalcToken::Minus)
                            | Some(CalcToken::Star)
                            | Some(CalcToken::Slash)
                            | Some(CalcToken::OpenParen)
                    );
                if is_unary {
                    // Parse as negative number
                    let start = i;
                    i += 1; // skip '-'
                    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                        i += 1;
                    }
                    // Check for unit suffix
                    let num_end = i;
                    let unit_start = i;
                    while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    // Check for % suffix
                    if i < bytes.len() && bytes[i] == b'%' {
                        i += 1;
                    }
                    let token_str = &expr[start..i];
                    let (px, pct) = calc_parse_value_token(token_str, em_base, rem_base)?;
                    tokens.push(CalcToken::Value(px, pct));
                    let _ = (num_end, unit_start); // suppress unused warnings
                } else {
                    tokens.push(CalcToken::Minus);
                    i += 1;
                }
            }
            b'*' => {
                tokens.push(CalcToken::Star);
                i += 1;
            }
            b'/' => {
                tokens.push(CalcToken::Slash);
                i += 1;
            }
            b'(' => {
                tokens.push(CalcToken::OpenParen);
                i += 1;
            }
            b')' => {
                tokens.push(CalcToken::CloseParen);
                i += 1;
            }
            _ if b.is_ascii_digit() || b == b'.' => {
                let start = i;
                while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                // Check for unit suffix or %
                while i < bytes.len() && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                if i < bytes.len() && bytes[i] == b'%' {
                    i += 1;
                }
                let token_str = &expr[start..i];
                let (px, pct) = calc_parse_value_token(token_str, em_base, rem_base)?;
                tokens.push(CalcToken::Value(px, pct));
            }
            _ => {
                // Skip unknown characters
                i += 1;
            }
        }
    }
    Some(tokens)
}

/// Parse a single value token from calc (e.g. "10px", "50%", "2em", "3").
/// Returns (px_component, percentage_component).
fn calc_parse_value_token(token: &str, em_base: f32, rem_base: f32) -> Option<(f32, f32)> {
    let s = token.trim();
    if s.is_empty() {
        return None;
    }
    if let Some(num) = s.strip_suffix('%') {
        let val = num.trim().parse::<f32>().ok()?;
        return Some((0.0, val));
    }
    if let Some(num) = s.strip_suffix("rem") {
        let val = num.trim().parse::<f32>().ok()?;
        return Some((val * rem_base, 0.0));
    }
    if let Some(num) = s.strip_suffix("px") {
        let val = num.trim().parse::<f32>().ok()?;
        return Some((val, 0.0));
    }
    if let Some(num) = s.strip_suffix("em") {
        let val = num.trim().parse::<f32>().ok()?;
        return Some((val * em_base, 0.0));
    }
    if let Some(num) = s.strip_suffix("pt") {
        let val = num.trim().parse::<f32>().ok()?;
        return Some((val * 4.0 / 3.0, 0.0)); // 1pt = 4/3 px
    }
    // Plain number (unitless scalar)
    let val = s.parse::<f32>().ok()?;
    Some((val, 0.0))
}

/// Parse an additive expression: term (('+' | '-') term)*
fn calc_parse_expr(tokens: &[CalcToken], pos: &mut usize) -> Option<(f32, f32)> {
    let mut result = calc_parse_term(tokens, pos)?;
    while *pos < tokens.len() {
        match tokens[*pos] {
            CalcToken::Plus => {
                *pos += 1;
                let rhs = calc_parse_term(tokens, pos)?;
                result.0 += rhs.0;
                result.1 += rhs.1;
            }
            CalcToken::Minus => {
                *pos += 1;
                let rhs = calc_parse_term(tokens, pos)?;
                result.0 -= rhs.0;
                result.1 -= rhs.1;
            }
            _ => break,
        }
    }
    Some(result)
}

/// Parse a multiplicative term: factor (('*' | '/') factor)*
fn calc_parse_term(tokens: &[CalcToken], pos: &mut usize) -> Option<(f32, f32)> {
    let mut result = calc_parse_factor(tokens, pos)?;
    while *pos < tokens.len() {
        match tokens[*pos] {
            CalcToken::Star => {
                *pos += 1;
                let rhs = calc_parse_factor(tokens, pos)?;
                // Multiplication: at least one side must be a pure scalar
                if rhs.1 == 0.0 && result.1 == 0.0 {
                    result.0 *= rhs.0;
                } else if rhs.1 == 0.0 {
                    result.0 *= rhs.0;
                    result.1 *= rhs.0;
                } else if result.1 == 0.0 {
                    let scalar = result.0;
                    result.0 = rhs.0 * scalar;
                    result.1 = rhs.1 * scalar;
                } else {
                    return None; // Cannot multiply two percentage values
                }
            }
            CalcToken::Slash => {
                *pos += 1;
                let rhs = calc_parse_factor(tokens, pos)?;
                // Division: divisor must be a pure scalar
                if rhs.1 != 0.0 {
                    return None;
                }
                if rhs.0 == 0.0 {
                    return None; // Division by zero
                }
                result.0 /= rhs.0;
                result.1 /= rhs.0;
            }
            _ => break,
        }
    }
    Some(result)
}

/// Parse a primary factor: a value literal or parenthesized expression.
fn calc_parse_factor(tokens: &[CalcToken], pos: &mut usize) -> Option<(f32, f32)> {
    if *pos >= tokens.len() {
        return None;
    }
    match tokens[*pos] {
        CalcToken::Value(px, pct) => {
            *pos += 1;
            Some((px, pct))
        }
        CalcToken::OpenParen => {
            *pos += 1; // skip '('
            let result = calc_parse_expr(tokens, pos)?;
            if *pos < tokens.len() && matches!(tokens[*pos], CalcToken::CloseParen) {
                *pos += 1; // skip ')'
            } else {
                return None; // Missing close paren
            }
            Some(result)
        }
        _ => None,
    }
}

/// Parse a CSS length value into px.
/// Supports: "16px", "2em", "1.5rem", "50%", plain numbers, and `calc()` expressions.
/// `em_base` is the reference for em units (element's own font-size,
///  or parent's font-size when resolving font-size itself).
/// `rem_base` is the root element's font-size for rem units.
pub fn parse_length(value: &str, em_base: f32, rem_base: f32) -> f32 {
    let s = value.trim();

    if s == "0" {
        return 0.0;
    }

    // Handle calc() expressions (collapses to px with percentage resolved against em_base)
    if let Some(inner) = strip_function_call(s, "calc")
        && let Some((px, pct)) = evaluate_calc(inner, em_base, rem_base)
    {
        return px + pct / 100.0 * em_base;
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

/// Represents either a definite pixel length, a percentage value (0.0..100.0),
/// a linear combination of px and percentage (`calc()`), or auto.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthOrPercentage {
    Px(f32),
    Percentage(f32),
    /// Result of a `calc()` expression: `px + percentage/100 * base`.
    Calc {
        px: f32,
        percentage: f32,
    },
    Auto,
}

impl LengthOrPercentage {
    #[inline]
    pub fn is_auto(&self) -> bool {
        matches!(self, LengthOrPercentage::Auto)
    }

    #[inline]
    pub fn resolve_against(&self, base: f32) -> Option<f32> {
        match *self {
            LengthOrPercentage::Px(px) => Some(px),
            LengthOrPercentage::Percentage(pct) => Some(pct / 100.0 * base),
            LengthOrPercentage::Calc { px, percentage } => Some(px + percentage / 100.0 * base),
            LengthOrPercentage::Auto => None,
        }
    }
}

/// Format a `LengthOrPercentage` for display.
fn format_lop(lop: &LengthOrPercentage) -> String {
    match lop {
        LengthOrPercentage::Px(v) => format!("{}px", v),
        LengthOrPercentage::Percentage(p) => format!("{}%", p),
        LengthOrPercentage::Calc { px, percentage } => {
            format!("calc({}% + {}px)", percentage, px)
        }
        LengthOrPercentage::Auto => "auto".to_string(),
    }
}

/// Parse a length or percentage value without collapsing percentages to px.
/// Supports `calc()` expressions that mix px and percentage values.
pub fn parse_length_or_percentage(value: &str, em_base: f32, rem_base: f32) -> LengthOrPercentage {
    let s = value.trim();
    if s.eq_ignore_ascii_case("auto") {
        return LengthOrPercentage::Auto;
    }
    // Check for calc() expression
    if let Some(inner) = strip_function_call(s, "calc")
        && let Some((px, pct)) = evaluate_calc(inner, em_base, rem_base)
    {
        if pct == 0.0 {
            return LengthOrPercentage::Px(px);
        }
        if px == 0.0 {
            return LengthOrPercentage::Percentage(pct);
        }
        return LengthOrPercentage::Calc {
            px,
            percentage: pct,
        };
    }
    if let Some(p) = s
        .strip_suffix('%')
        .and_then(|num| num.trim().parse::<f32>().ok())
    {
        return LengthOrPercentage::Percentage(p);
    }
    LengthOrPercentage::Px(parse_length(s, em_base, rem_base))
}

/// Represents a valid CSS z-index value: integer or auto.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZIndex {
    Auto,
    Integer(i32),
}

/// Try to parse a CSS z-index value (integer or 'auto').
/// Returns Some(ZIndex::Auto) for 'auto', Some(ZIndex::Integer(n)) for valid integers,
/// and None if the syntax is invalid.
pub fn try_parse_z_index(value: &str) -> Option<ZIndex> {
    let s = value.trim();
    if s.eq_ignore_ascii_case("auto") {
        Some(ZIndex::Auto)
    } else if let Ok(n) = s.parse::<i32>() {
        Some(ZIndex::Integer(n))
    } else {
        None
    }
}

/// Parse a CSS z-index value returning Some(i32) for numeric values and None for auto/invalid.
pub fn parse_z_index(value: &str) -> Option<i32> {
    match try_parse_z_index(value) {
        Some(ZIndex::Integer(n)) => Some(n),
        _ => None,
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
        // Table display types
        "table" => Display::Table,
        "table-row" => Display::TableRow,
        "table-cell" => Display::TableCell,
        "table-row-group" => Display::TableRowGroup,
        "table-header-group" => Display::TableHeaderGroup,
        "table-footer-group" => Display::TableFooterGroup,
        "table-caption" => Display::TableCaption,
        "inline-table" => Display::InlineTable,
        "table-column" => Display::TableColumn,
        "table-column-group" => Display::TableColumnGroup,
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

/// Parse a CSS flex-direction value.
pub fn parse_flex_direction(value: &str) -> FlexDirection {
    match value.trim().to_ascii_lowercase().as_str() {
        "row-reverse" => FlexDirection::RowReverse,
        "column" => FlexDirection::Column,
        "column-reverse" => FlexDirection::ColumnReverse,
        _ => FlexDirection::Row,
    }
}

/// Parse a CSS flex-wrap value.
pub fn parse_flex_wrap(value: &str) -> FlexWrap {
    match value.trim().to_ascii_lowercase().as_str() {
        "wrap" => FlexWrap::Wrap,
        "wrap-reverse" => FlexWrap::WrapReverse,
        _ => FlexWrap::NoWrap,
    }
}

/// Parse a CSS justify-content value.
pub fn parse_justify_content(value: &str) -> JustifyContent {
    match value.trim().to_ascii_lowercase().as_str() {
        "flex-start" => JustifyContent::FlexStart,
        "flex-end" => JustifyContent::FlexEnd,
        "center" => JustifyContent::Center,
        "space-between" => JustifyContent::SpaceBetween,
        "space-around" => JustifyContent::SpaceAround,
        "space-evenly" => JustifyContent::SpaceEvenly,
        "start" => JustifyContent::Start,
        "end" => JustifyContent::End,
        _ => JustifyContent::FlexStart,
    }
}

/// Parse a CSS align-items value.
pub fn parse_align_items(value: &str) -> AlignItems {
    match value.trim().to_ascii_lowercase().as_str() {
        "flex-start" | "start" => AlignItems::FlexStart,
        "flex-end" | "end" => AlignItems::FlexEnd,
        "center" => AlignItems::Center,
        "baseline" => AlignItems::Baseline,
        _ => AlignItems::Stretch,
    }
}

/// Parse a CSS align-self value.
pub fn parse_align_self(value: &str) -> AlignSelf {
    match value.trim().to_ascii_lowercase().as_str() {
        "stretch" => AlignSelf::Stretch,
        "flex-start" | "start" => AlignSelf::FlexStart,
        "flex-end" | "end" => AlignSelf::FlexEnd,
        "center" => AlignSelf::Center,
        "baseline" => AlignSelf::Baseline,
        _ => AlignSelf::Auto,
    }
}

/// Parse a CSS flex-grow value (non-negative number, default 0.0).
pub fn parse_flex_grow(value: &str) -> f32 {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|&v| v >= 0.0)
        .unwrap_or(0.0)
}

/// Parse a CSS flex-shrink value (non-negative number, default 1.0).
pub fn parse_flex_shrink(value: &str) -> f32 {
    value
        .trim()
        .parse::<f32>()
        .ok()
        .filter(|&v| v >= 0.0)
        .unwrap_or(1.0)
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

/// Parse a CSS flex shorthand value (e.g. "1", "1 0 auto", "none") into (flex-grow, flex-shrink, flex-basis) strings.
pub fn parse_flex_shorthand(value: &str) -> (String, String, String) {
    let s = value.trim().to_ascii_lowercase();
    if s == "none" {
        return ("0".to_string(), "0".to_string(), "auto".to_string());
    }
    if s == "auto" {
        return ("1".to_string(), "1".to_string(), "auto".to_string());
    }
    if s == "initial" {
        return ("0".to_string(), "1".to_string(), "auto".to_string());
    }
    let parts: Vec<&str> = s.split_whitespace().collect();
    match parts.len() {
        1 => {
            let p0 = parts[0];
            if p0.parse::<f32>().is_ok() {
                // e.g. "flex: 1" -> grow 1, shrink 1, basis 0%
                (p0.to_string(), "1".to_string(), "0%".to_string())
            } else {
                // e.g. "flex: 100px" -> grow 1, shrink 1, basis 100px
                ("1".to_string(), "1".to_string(), p0.to_string())
            }
        }
        2 => {
            let p0 = parts[0];
            let p1 = parts[1];
            if p1.parse::<f32>().is_ok() {
                // <grow> <shrink> -> basis is 0%
                (p0.to_string(), p1.to_string(), "0%".to_string())
            } else {
                // <grow> <basis> -> shrink is 1
                (p0.to_string(), "1".to_string(), p1.to_string())
            }
        }
        3 => (
            parts[0].to_string(),
            parts[1].to_string(),
            parts[2].to_string(),
        ),
        _ => ("0".to_string(), "1".to_string(), "auto".to_string()),
    }
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
    parse_grid_tracks_inner(value.trim())
}

/// Inner recursive parser for grid tracks, handling `repeat()` and `minmax()`.
fn parse_grid_tracks_inner(value: &str) -> Vec<GridTrack> {
    let mut tracks = Vec::new();
    let mut chars = value.char_indices().peekable();
    let mut token_start: Option<usize> = None;

    while let Some(&(i, ch)) = chars.peek() {
        if ch.is_whitespace() {
            // Flush any current token
            if let Some(start) = token_start.take() {
                let token = value[start..i].trim();
                if !token.is_empty() {
                    tracks.push(parse_single_grid_track(token));
                }
            }
            chars.next();
        } else if ch.is_ascii_alphabetic() || ch == '-' {
            // Could be start of a function like repeat(...) or minmax(...)
            if token_start.is_none() {
                token_start = Some(i);
            }
            // Peek ahead to check for function call
            let rest = &value[i..];
            let lower = rest.to_ascii_lowercase();
            if lower.starts_with("repeat(") || lower.starts_with("minmax(") {
                // Find the matching close paren ("repeat(" and "minmax(" are both 7 chars)
                let func_name_len = 7;
                let paren_start = i + func_name_len;
                let mut depth = 1;
                let mut end = paren_start;
                for (j, c) in value[paren_start..].char_indices() {
                    match c {
                        '(' => depth += 1,
                        ')' => {
                            depth -= 1;
                            if depth == 0 {
                                end = paren_start + j;
                                break;
                            }
                        }
                        _ => {}
                    }
                }
                let func_str = &value[i..=end];
                if lower.starts_with("repeat(") {
                    tracks.extend(parse_repeat_function(func_str));
                } else {
                    tracks.push(parse_minmax_function(func_str));
                }
                token_start = None;
                // Advance past the function
                while chars.peek().is_some_and(|&(idx, _)| idx <= end) {
                    chars.next();
                }
            } else {
                chars.next();
            }
        } else {
            if token_start.is_none() {
                token_start = Some(i);
            }
            chars.next();
        }
    }

    // Flush trailing token
    if let Some(start) = token_start {
        let token = value[start..].trim();
        if !token.is_empty() {
            tracks.push(parse_single_grid_track(token));
        }
    }

    tracks
}

/// Parse a single grid track value (not a function).
fn parse_single_grid_track(part: &str) -> GridTrack {
    let part = part.trim();
    if part.eq_ignore_ascii_case("auto") {
        GridTrack::Auto
    } else if let Some(num) = part.strip_suffix("fr") {
        num.parse::<f32>()
            .map(GridTrack::Fr)
            .unwrap_or(GridTrack::Auto)
    } else if let Some(num) = part.strip_suffix("px") {
        num.parse::<f32>()
            .map(GridTrack::Px)
            .unwrap_or(GridTrack::Auto)
    } else if let Some(num) = part.strip_suffix('%') {
        num.parse::<f32>()
            .map(GridTrack::Percent)
            .unwrap_or(GridTrack::Auto)
    } else if part.parse::<f32>().is_ok_and(|val| val == 0.0) {
        GridTrack::Px(0.0)
    } else {
        GridTrack::Auto
    }
}

/// Parse a `repeat(count, track_list)` function.
/// e.g. `repeat(3, 1fr)` → [Fr(1.0), Fr(1.0), Fr(1.0)]
/// e.g. `repeat(2, 100px 1fr)` → [Px(100.0), Fr(1.0), Px(100.0), Fr(1.0)]
fn parse_repeat_function(func_str: &str) -> Vec<GridTrack> {
    if let Some(inner) = strip_function_call(func_str, "repeat") {
        // Split on first comma: count, track_list
        if let Some(comma_pos) = inner.find(',') {
            let count_str = inner[..comma_pos].trim();
            let track_str = inner[comma_pos + 1..].trim();
            if let Ok(count) = count_str.parse::<usize>() {
                let pattern = parse_grid_tracks_inner(track_str);
                let mut result = Vec::with_capacity(pattern.len() * count);
                for _ in 0..count {
                    result.extend(pattern.iter().cloned());
                }
                return result;
            }
        }
    }
    Vec::new()
}

/// Parse a `minmax(min, max)` function.
/// e.g. `minmax(80px, 1fr)` → MinMax(Px(80.0), Fr(1.0))
fn parse_minmax_function(func_str: &str) -> GridTrack {
    if let Some(inner) = strip_function_call(func_str, "minmax") {
        // Split on first comma
        if let Some(comma_pos) = inner.find(',') {
            let min_str = inner[..comma_pos].trim();
            let max_str = inner[comma_pos + 1..].trim();
            let min_track = parse_single_grid_track(min_str);
            let max_track = parse_single_grid_track(max_str);
            return GridTrack::MinMax(Box::new(min_track), Box::new(max_track));
        }
    }
    GridTrack::Auto
}

/// Parse a CSS grid-column / grid-row value into a `GridPlacement`.
/// Supports: `auto`, `<line>`, `span <N>`, `<start> / <end>`, `<start> / span <N>`,
/// and negative line indices (e.g. `-1`).
pub fn parse_grid_placement(value: &str) -> GridPlacement {
    let s = value.trim();

    // Check for start / end syntax
    if let Some(slash_pos) = s.find('/') {
        let start_str = s[..slash_pos].trim();
        let end_str = s[slash_pos + 1..].trim();
        let start = parse_grid_line(start_str);
        let end = parse_grid_line(end_str);
        return GridPlacement { start, end };
    }

    // Single value
    let start = parse_grid_line(s);
    GridPlacement {
        start,
        end: GridLine::Auto,
    }
}

/// Parse a single grid line reference: `auto`, `span N`, or a line number (including negative).
fn parse_grid_line(s: &str) -> GridLine {
    let s = s.trim();
    if s.eq_ignore_ascii_case("auto") || s.is_empty() {
        return GridLine::Auto;
    }
    if let Some(span_str) = s
        .to_ascii_lowercase()
        .strip_prefix("span")
        .map(|rest| rest.trim().to_string())
    {
        if let Ok(val) = span_str.parse::<i32>() {
            return GridLine::Span(val.max(1));
        }
        return GridLine::Span(1);
    }
    if let Ok(val) = s.parse::<i32>() {
        return GridLine::Line(val);
    }
    GridLine::Auto
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

// ─── Table & Visual Rendering Parsers ───────────────────────────

/// Parse a CSS border-collapse value.
pub fn parse_border_collapse(value: &str) -> BorderCollapse {
    match value.trim().to_ascii_lowercase().as_str() {
        "collapse" => BorderCollapse::Collapse,
        _ => BorderCollapse::Separate,
    }
}

/// Parse a CSS vertical-align value.
pub fn parse_vertical_align(value: &str) -> VerticalAlign {
    match value.trim().to_ascii_lowercase().as_str() {
        "top" => VerticalAlign::Top,
        "middle" => VerticalAlign::Middle,
        "bottom" => VerticalAlign::Bottom,
        _ => VerticalAlign::Baseline,
    }
}

/// Parse a CSS opacity value (0.0 to 1.0), accepting decimals or percentages.
pub fn parse_opacity(value: &str) -> f32 {
    let s = value.trim();
    let parsed = if let Some(pct) = s.strip_suffix('%') {
        pct.trim().parse::<f32>().map(|v| v / 100.0)
    } else {
        s.parse::<f32>()
    };
    parsed.unwrap_or(1.0).clamp(0.0, 1.0)
}

/// Parse a CSS overflow value.
pub fn parse_overflow(value: &str) -> Overflow {
    match value.trim().to_ascii_lowercase().as_str() {
        "hidden" => Overflow::Hidden,
        "scroll" => Overflow::Scroll,
        "auto" => Overflow::Auto,
        _ => Overflow::Visible,
    }
}

/// Parse a CSS border-radius shorthand value.
/// Supports 1-4 values: `10px`, `10px 20px`, `10px 20px 30px`, `10px 20px 30px 40px`.
pub fn parse_border_radius(value: &str, em_base: f32, rem_base: f32) -> BorderRadius {
    let trimmed = value.trim().to_ascii_lowercase();
    if trimmed == "0" || trimmed.is_empty() {
        return BorderRadius::ZERO;
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    match parts.len() {
        1 => {
            let r = parse_length(parts[0], em_base, rem_base);
            BorderRadius::uniform(r)
        }
        2 => {
            let tl_br = parse_length(parts[0], em_base, rem_base);
            let tr_bl = parse_length(parts[1], em_base, rem_base);
            BorderRadius {
                top_left: tl_br,
                top_right: tr_bl,
                bottom_right: tl_br,
                bottom_left: tr_bl,
            }
        }
        3 => {
            let tl = parse_length(parts[0], em_base, rem_base);
            let tr_bl = parse_length(parts[1], em_base, rem_base);
            let br = parse_length(parts[2], em_base, rem_base);
            BorderRadius {
                top_left: tl,
                top_right: tr_bl,
                bottom_right: br,
                bottom_left: tr_bl,
            }
        }
        4 => BorderRadius {
            top_left: parse_length(parts[0], em_base, rem_base),
            top_right: parse_length(parts[1], em_base, rem_base),
            bottom_right: parse_length(parts[2], em_base, rem_base),
            bottom_left: parse_length(parts[3], em_base, rem_base),
        },
        _ => BorderRadius::ZERO,
    }
}

fn split_commas_outside_parens(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth: usize = 0;
    for (idx, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(&s[start..idx]);
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&s[start..]);
    parts
}

fn tokenize_shadow_tokens(s: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let mut start = None;
    let mut depth: usize = 0;
    for (idx, ch) in s.char_indices() {
        if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth = depth.saturating_sub(1);
        } else if ch.is_whitespace() && depth == 0 {
            if let Some(st) = start {
                tokens.push(&s[st..idx]);
                start = None;
            }
            continue;
        }
        if start.is_none() {
            start = Some(idx);
        }
    }
    if let Some(st) = start {
        tokens.push(&s[st..]);
    }
    tokens
}

fn try_parse_shadow_length(value: &str, em_base: f32, rem_base: f32) -> Option<f32> {
    let s = value.trim();
    if s == "0" {
        return Some(0.0);
    }
    if let Some(num) = s.strip_suffix("px") {
        return num.trim().parse::<f32>().ok();
    }
    if let Some(num) = s.strip_suffix("rem") {
        return num.trim().parse::<f32>().ok().map(|n| n * rem_base);
    }
    if let Some(num) = s.strip_suffix("em") {
        return num.trim().parse::<f32>().ok().map(|n| n * em_base);
    }
    s.parse::<f32>().ok()
}

/// Parse a CSS box-shadow value.
/// Supports: `none`, `<offset-x> <offset-y> [blur] [spread] [color] [inset]`
/// Multiple shadows separated by commas.
pub fn parse_box_shadow(value: &str, em_base: f32, rem_base: f32) -> Vec<BoxShadow> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("none") || trimmed.is_empty() {
        return Vec::new();
    }

    let mut shadows = Vec::new();
    for shadow_str in split_commas_outside_parens(trimmed) {
        let shadow_str = shadow_str.trim();
        if !shadow_str.is_empty()
            && let Some(shadow) = parse_single_box_shadow(shadow_str, em_base, rem_base)
        {
            shadows.push(shadow);
        }
    }
    shadows
}

/// Parse a single box-shadow value.
fn parse_single_box_shadow(value: &str, em_base: f32, rem_base: f32) -> Option<BoxShadow> {
    let parts = tokenize_shadow_tokens(value);
    if parts.len() < 2 {
        return None;
    }

    let mut inset = false;
    let mut color = Color::BLACK;
    let mut lengths = Vec::new();

    for &part in &parts {
        if part.eq_ignore_ascii_case("inset") {
            inset = true;
        } else if let Some(css_color) = try_parse_css_color(part) {
            color = match css_color {
                CssColor::Rgba(c) => c,
                CssColor::CurrentColor => Color::BLACK,
            };
        } else if let Some(v) = try_parse_shadow_length(part, em_base, rem_base) {
            lengths.push(v);
        }
    }

    if lengths.len() < 2 {
        return None;
    }

    Some(BoxShadow {
        offset_x: lengths[0],
        offset_y: lengths[1],
        blur_radius: *lengths.get(2).unwrap_or(&0.0),
        spread_radius: *lengths.get(3).unwrap_or(&0.0),
        color,
        inset,
    })
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
        assert_eq!(s.width, LengthOrPercentage::Auto);
        assert_eq!(s.height, LengthOrPercentage::Auto);
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
        assert_eq!(parse_css_color("currentColor"), CssColor::CurrentColor);
        assert_eq!(parse_css_color("currentcolor"), CssColor::CurrentColor);
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

    #[test]
    fn test_flex_property_parsing() {
        assert_eq!(parse_flex_direction("column"), FlexDirection::Column);
        assert_eq!(
            parse_flex_direction("row-reverse"),
            FlexDirection::RowReverse
        );
        assert_eq!(parse_flex_direction("invalid"), FlexDirection::Row);

        assert_eq!(parse_flex_wrap("wrap"), FlexWrap::Wrap);
        assert_eq!(parse_flex_wrap("wrap-reverse"), FlexWrap::WrapReverse);
        assert_eq!(parse_flex_wrap("nowrap"), FlexWrap::NoWrap);

        assert_eq!(parse_justify_content("center"), JustifyContent::Center);
        assert_eq!(
            parse_justify_content("space-between"),
            JustifyContent::SpaceBetween
        );
        assert_eq!(
            parse_justify_content("space-evenly"),
            JustifyContent::SpaceEvenly
        );
        assert_eq!(parse_justify_content("start"), JustifyContent::Start);
        assert_eq!(parse_justify_content("end"), JustifyContent::End);
        assert_eq!(
            parse_justify_content("flex-start"),
            JustifyContent::FlexStart
        );
        assert_eq!(parse_justify_content("flex-end"), JustifyContent::FlexEnd);

        assert_eq!(parse_align_items("center"), AlignItems::Center);
        assert_eq!(parse_align_items("flex-end"), AlignItems::FlexEnd);
        assert_eq!(parse_align_items("stretch"), AlignItems::Stretch);

        assert_eq!(parse_align_self("auto"), AlignSelf::Auto);
        assert_eq!(parse_align_self("center"), AlignSelf::Center);

        assert_eq!(parse_flex_grow("2.5"), 2.5);
        assert_eq!(parse_flex_grow("-1.0"), 0.0);
        assert_eq!(parse_flex_shrink("0.5"), 0.5);
        assert_eq!(parse_flex_shrink("invalid"), 1.0);
    }

    #[test]
    fn test_flex_shorthand_parsing() {
        assert_eq!(
            parse_flex_shorthand("1"),
            ("1".to_string(), "1".to_string(), "0%".to_string())
        );
        assert_eq!(
            parse_flex_shorthand("none"),
            ("0".to_string(), "0".to_string(), "auto".to_string())
        );
        assert_eq!(
            parse_flex_shorthand("auto"),
            ("1".to_string(), "1".to_string(), "auto".to_string())
        );
        assert_eq!(
            parse_flex_shorthand("2 1 100px"),
            ("2".to_string(), "1".to_string(), "100px".to_string())
        );
    }

    // ─── calc() Tests ────────────────────────────────────────────

    #[test]
    fn test_calc_simple_addition() {
        let result = evaluate_calc("10px + 20px", 16.0, 16.0);
        assert_eq!(result, Some((30.0, 0.0)));
    }

    #[test]
    fn test_calc_px_and_percentage() {
        let result = evaluate_calc("50% + 10px", 16.0, 16.0);
        assert_eq!(result, Some((10.0, 50.0)));
    }

    #[test]
    fn test_calc_subtraction() {
        let result = evaluate_calc("100px - 30px", 16.0, 16.0);
        assert_eq!(result, Some((70.0, 0.0)));
    }

    #[test]
    fn test_calc_multiplication() {
        let result = evaluate_calc("10px * 3", 16.0, 16.0);
        assert_eq!(result, Some((30.0, 0.0)));
    }

    #[test]
    fn test_calc_division() {
        let result = evaluate_calc("100px / 4", 16.0, 16.0);
        assert_eq!(result, Some((25.0, 0.0)));
    }

    #[test]
    fn test_calc_operator_precedence() {
        // 10px + 2px * 5 = 10 + 10 = 20
        let result = evaluate_calc("10px + 2px * 5", 16.0, 16.0);
        assert_eq!(result, Some((20.0, 0.0)));
    }

    #[test]
    fn test_calc_nested_parens() {
        // (10px + 20px) * 2 = 60
        let result = evaluate_calc("(10px + 20px) * 2", 16.0, 16.0);
        assert_eq!(result, Some((60.0, 0.0)));
    }

    #[test]
    fn test_calc_em_units() {
        // 2em with em_base=16 = 32px
        let result = evaluate_calc("2em + 10px", 16.0, 16.0);
        assert_eq!(result, Some((42.0, 0.0)));
    }

    #[test]
    fn test_calc_rem_units() {
        // 1rem with rem_base=20 = 20px
        let result = evaluate_calc("1rem + 5px", 16.0, 20.0);
        assert_eq!(result, Some((25.0, 0.0)));
    }

    #[test]
    fn test_calc_negative_value() {
        let result = evaluate_calc("-10px + 30px", 16.0, 16.0);
        assert_eq!(result, Some((20.0, 0.0)));
    }

    #[test]
    fn test_calc_percentage_multiply_by_scalar() {
        // 50% * 2 = 100%
        let result = evaluate_calc("50% * 2", 16.0, 16.0);
        assert_eq!(result, Some((0.0, 100.0)));
    }

    #[test]
    fn test_calc_division_by_zero_returns_none() {
        let result = evaluate_calc("10px / 0", 16.0, 16.0);
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_length_with_calc() {
        let result = parse_length("calc(10px + 20px)", 16.0, 16.0);
        assert!((result - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_length_or_percentage_with_calc_mixed() {
        let result = parse_length_or_percentage("calc(50% + 10px)", 16.0, 16.0);
        match result {
            LengthOrPercentage::Calc { px, percentage } => {
                assert!((px - 10.0).abs() < 0.001);
                assert!((percentage - 50.0).abs() < 0.001);
            }
            _ => panic!("Expected Calc variant, got {:?}", result),
        }
    }

    #[test]
    fn test_parse_length_or_percentage_calc_pure_px() {
        // calc(10px + 20px) should simplify to Px(30)
        let result = parse_length_or_percentage("calc(10px + 20px)", 16.0, 16.0);
        assert_eq!(result, LengthOrPercentage::Px(30.0));
    }

    #[test]
    fn test_parse_length_or_percentage_calc_pure_pct() {
        // calc(30% + 20%) should simplify to Percentage(50)
        let result = parse_length_or_percentage("calc(30% + 20%)", 16.0, 16.0);
        assert_eq!(result, LengthOrPercentage::Percentage(50.0));
    }

    #[test]
    fn test_calc_resolve_against() {
        let lop = LengthOrPercentage::Calc {
            px: 10.0,
            percentage: 50.0,
        };
        // With base=200: 10 + 50/100 * 200 = 10 + 100 = 110
        assert_eq!(lop.resolve_against(200.0), Some(110.0));
    }

    // ─── Grid Track Enhanced Parsing Tests ────────────────────────

    #[test]
    fn test_parse_grid_tracks_basic() {
        let tracks = parse_grid_tracks("100px 1fr auto");
        assert_eq!(
            tracks,
            vec![GridTrack::Px(100.0), GridTrack::Fr(1.0), GridTrack::Auto]
        );
    }

    #[test]
    fn test_parse_grid_tracks_repeat() {
        let tracks = parse_grid_tracks("repeat(3, 1fr)");
        assert_eq!(
            tracks,
            vec![GridTrack::Fr(1.0), GridTrack::Fr(1.0), GridTrack::Fr(1.0)]
        );
    }

    #[test]
    fn test_parse_grid_tracks_repeat_multi_pattern() {
        let tracks = parse_grid_tracks("repeat(2, 100px 1fr)");
        assert_eq!(
            tracks,
            vec![
                GridTrack::Px(100.0),
                GridTrack::Fr(1.0),
                GridTrack::Px(100.0),
                GridTrack::Fr(1.0),
            ]
        );
    }

    #[test]
    fn test_parse_grid_tracks_minmax() {
        let tracks = parse_grid_tracks("minmax(80px, 1fr)");
        assert_eq!(
            tracks,
            vec![GridTrack::MinMax(
                Box::new(GridTrack::Px(80.0)),
                Box::new(GridTrack::Fr(1.0))
            )]
        );
    }

    #[test]
    fn test_parse_grid_tracks_mixed() {
        let tracks = parse_grid_tracks("100px minmax(50px, 1fr) auto");
        assert_eq!(
            tracks,
            vec![
                GridTrack::Px(100.0),
                GridTrack::MinMax(Box::new(GridTrack::Px(50.0)), Box::new(GridTrack::Fr(1.0))),
                GridTrack::Auto,
            ]
        );
    }

    // ─── Grid Placement Enhanced Parsing Tests ────────────────────

    #[test]
    fn test_parse_grid_placement_auto() {
        let p = parse_grid_placement("auto");
        assert_eq!(
            p,
            GridPlacement {
                start: GridLine::Auto,
                end: GridLine::Auto
            }
        );
    }

    #[test]
    fn test_parse_grid_placement_single_line() {
        let p = parse_grid_placement("2");
        assert_eq!(
            p,
            GridPlacement {
                start: GridLine::Line(2),
                end: GridLine::Auto
            }
        );
    }

    #[test]
    fn test_parse_grid_placement_span() {
        let p = parse_grid_placement("span 3");
        assert_eq!(
            p,
            GridPlacement {
                start: GridLine::Span(3),
                end: GridLine::Auto
            }
        );
    }

    #[test]
    fn test_parse_grid_placement_start_end() {
        let p = parse_grid_placement("1 / 3");
        assert_eq!(
            p,
            GridPlacement {
                start: GridLine::Line(1),
                end: GridLine::Line(3)
            }
        );
    }

    #[test]
    fn test_parse_grid_placement_start_span() {
        let p = parse_grid_placement("1 / span 2");
        assert_eq!(
            p,
            GridPlacement {
                start: GridLine::Line(1),
                end: GridLine::Span(2)
            }
        );
    }

    #[test]
    fn test_parse_grid_placement_negative_line() {
        let p = parse_grid_placement("1 / -1");
        assert_eq!(
            p,
            GridPlacement {
                start: GridLine::Line(1),
                end: GridLine::Line(-1)
            }
        );
    }

    // ─── strip_function_call Tests ───────────────────────────────

    #[test]
    fn test_strip_function_call_basic() {
        assert_eq!(
            strip_function_call("calc(10px + 20px)", "calc"),
            Some("10px + 20px")
        );
    }

    #[test]
    fn test_strip_function_call_nested() {
        assert_eq!(
            strip_function_call("var(--x, calc(1px + 2px))", "var"),
            Some("--x, calc(1px + 2px)")
        );
    }

    #[test]
    fn test_strip_function_call_no_match() {
        assert_eq!(strip_function_call("10px", "calc"), None);
    }

    #[test]
    fn test_format_lop_calc() {
        let lop = LengthOrPercentage::Calc {
            px: 10.0,
            percentage: 50.0,
        };
        assert_eq!(format_lop(&lop), "calc(50% + 10px)");
    }

    // ─── Table & Visual Rendering Tests ─────────────────────────

    #[test]
    fn test_parse_border_collapse() {
        assert_eq!(parse_border_collapse("collapse"), BorderCollapse::Collapse);
        assert_eq!(parse_border_collapse("separate"), BorderCollapse::Separate);
        assert_eq!(parse_border_collapse("invalid"), BorderCollapse::Separate);
    }

    #[test]
    fn test_parse_vertical_align() {
        assert_eq!(parse_vertical_align("top"), VerticalAlign::Top);
        assert_eq!(parse_vertical_align("middle"), VerticalAlign::Middle);
        assert_eq!(parse_vertical_align("bottom"), VerticalAlign::Bottom);
        assert_eq!(parse_vertical_align("baseline"), VerticalAlign::Baseline);
        assert_eq!(parse_vertical_align("auto"), VerticalAlign::Baseline);
    }

    #[test]
    fn test_parse_opacity() {
        assert_eq!(parse_opacity("1.0"), 1.0);
        assert_eq!(parse_opacity("0.5"), 0.5);
        assert_eq!(parse_opacity("0"), 0.0);
        assert_eq!(parse_opacity("1.5"), 1.0);
        assert_eq!(parse_opacity("-0.2"), 0.0);
        assert_eq!(parse_opacity("invalid"), 1.0);
    }

    #[test]
    fn test_parse_overflow() {
        assert_eq!(parse_overflow("visible"), Overflow::Visible);
        assert_eq!(parse_overflow("hidden"), Overflow::Hidden);
        assert_eq!(parse_overflow("scroll"), Overflow::Scroll);
        assert_eq!(parse_overflow("auto"), Overflow::Auto);
        assert_eq!(parse_overflow("unknown"), Overflow::Visible);
    }

    #[test]
    fn test_parse_border_radius() {
        assert_eq!(parse_border_radius("0", 16.0, 16.0), BorderRadius::ZERO);
        assert_eq!(
            parse_border_radius("10px", 16.0, 16.0),
            BorderRadius::uniform(10.0)
        );
        assert_eq!(
            parse_border_radius("10px 20px", 16.0, 16.0),
            BorderRadius {
                top_left: 10.0,
                top_right: 20.0,
                bottom_right: 10.0,
                bottom_left: 20.0,
            }
        );
        assert_eq!(
            parse_border_radius("10px 20px 30px", 16.0, 16.0),
            BorderRadius {
                top_left: 10.0,
                top_right: 20.0,
                bottom_right: 30.0,
                bottom_left: 20.0,
            }
        );
        assert_eq!(
            parse_border_radius("10px 20px 30px 40px", 16.0, 16.0),
            BorderRadius {
                top_left: 10.0,
                top_right: 20.0,
                bottom_right: 30.0,
                bottom_left: 40.0,
            }
        );
    }

    #[test]
    fn test_parse_box_shadow() {
        assert_eq!(parse_box_shadow("none", 16.0, 16.0), Vec::new());
        let shadows = parse_box_shadow("2px 4px 6px 8px red", 16.0, 16.0);
        assert_eq!(shadows.len(), 1);
        assert_eq!(shadows[0].offset_x, 2.0);
        assert_eq!(shadows[0].offset_y, 4.0);
        assert_eq!(shadows[0].blur_radius, 6.0);
        assert_eq!(shadows[0].spread_radius, 8.0);
        assert_eq!(shadows[0].color, Color::rgb(255, 0, 0));
        assert!(!shadows[0].inset);

        let inset_shadow = parse_box_shadow("inset 0 2px 4px #000", 16.0, 16.0);
        assert_eq!(inset_shadow.len(), 1);
        assert!(inset_shadow[0].inset);
        assert_eq!(inset_shadow[0].offset_x, 0.0);
        assert_eq!(inset_shadow[0].offset_y, 2.0);

        let multi = parse_box_shadow("2px 2px red, 4px 4px blue", 16.0, 16.0);
        assert_eq!(multi.len(), 2);
    }

    #[test]
    fn test_color_alpha_multiplier() {
        let c = Color::new(100, 150, 200, 255);
        let c_half = c.with_alpha_multiplier(0.5);
        assert_eq!(c_half.r, 100);
        assert_eq!(c_half.g, 150);
        assert_eq!(c_half.b, 200);
        assert_eq!(c_half.a, 128);

        let c_zero = c.with_alpha_multiplier(0.0);
        assert_eq!(c_zero.a, 0);
    }

    #[test]
    fn test_parse_opacity_percentage() {
        assert_eq!(parse_opacity("50%"), 0.5);
        assert_eq!(parse_opacity("100%"), 1.0);
        assert_eq!(parse_opacity("0%"), 0.0);
        assert_eq!(parse_opacity(" 75% "), 0.75);
        assert_eq!(parse_opacity("150%"), 1.0);
        assert_eq!(parse_opacity("0.4"), 0.4);
    }

    #[test]
    fn test_parse_box_shadow_spaced_rgba() {
        let shadows = parse_box_shadow("0 4px 6px rgba(0, 0, 0, 0.1)", 16.0, 16.0);
        assert_eq!(shadows.len(), 1);
        assert_eq!(shadows[0].offset_x, 0.0);
        assert_eq!(shadows[0].offset_y, 4.0);
        assert_eq!(shadows[0].blur_radius, 6.0);
        assert_eq!(shadows[0].spread_radius, 0.0);
        assert_eq!(shadows[0].color, Color::new(0, 0, 0, 25));
        assert!(!shadows[0].inset);
    }

    #[test]
    fn test_parse_box_shadow_multiple_rgba() {
        let shadows = parse_box_shadow(
            "0 2px 4px rgba(0, 0, 0, 0.2), inset 0 1px 0 rgba(255, 255, 255, 0.5)",
            16.0,
            16.0,
        );
        assert_eq!(shadows.len(), 2);
        assert_eq!(shadows[0].offset_x, 0.0);
        assert_eq!(shadows[0].offset_y, 2.0);
        assert_eq!(shadows[0].blur_radius, 4.0);
        assert_eq!(shadows[0].color, Color::new(0, 0, 0, 51));
        assert!(!shadows[0].inset);

        assert_eq!(shadows[1].offset_x, 0.0);
        assert_eq!(shadows[1].offset_y, 1.0);
        assert_eq!(shadows[1].blur_radius, 0.0);
        assert_eq!(shadows[1].color, Color::new(255, 255, 255, 127));
        assert!(shadows[1].inset);
    }

    #[test]
    fn test_border_spacing_two_values() {
        let mut style = ComputedStyle::default();
        style.set_property(
            crate::properties::PropertyId::BorderSpacing,
            "10px 20px",
            16.0,
            16.0,
        );
        assert_eq!(style.border_spacing, 10.0);
    }
}
