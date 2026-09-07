// ─── CSS Property System ─────────────────────────────────────────
//
// Central registry of all CSS properties that Asteria V1 supports.
// For each property we track:
//   - A unique PropertyId enum variant
//   - Whether it inherits (child gets parent's value if unset)
//   - Its initial/default value
//
// This is queried by:
//   - The cascade (to know inherit vs initial for defaulting)
//   - Value computation (to know how to resolve units)
//   - Layout (to read typed values from ComputedStyle)

/// Every CSS property Asteria V1 knows about.
/// Each variant maps to exactly one longhand CSS property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PropertyId {
    // Box model
    Display,
    Position,
    ZIndex,
    Width,
    Height,
    /// CSS box-sizing: content-box | border-box
    BoxSizing,

    // Insets (Positioning)
    Top,
    Right,
    Bottom,
    Left,

    // Margins
    MarginTop,
    MarginRight,
    MarginBottom,
    MarginLeft,

    // Padding
    PaddingTop,
    PaddingRight,
    PaddingBottom,
    PaddingLeft,

    // Borders
    BorderTopWidth,
    BorderRightWidth,
    BorderBottomWidth,
    BorderLeftWidth,
    BorderColor,
    BorderStyle,

    // Text & font
    Color,
    BackgroundColor,
    FontSize,
    FontWeight,
    TextAlign,
    LineHeight,

    // Flexbox
    FlexDirection,
    FlexWrap,
    JustifyContent,
    AlignItems,
    AlignSelf,
    FlexGrow,
    FlexShrink,
    FlexBasis,

    // Grid
    GridTemplateColumns,
    GridTemplateRows,
    GridColumn,
    GridRow,
    GridColumnStart,
    GridColumnEnd,
    GridRowStart,
    GridRowEnd,
    GridGap,
    RowGap,
    ColumnGap,

    // Animation
    AnimationName,
    AnimationDuration,
    AnimationTimingFunction,
    AnimationIterationCount,

    // Table
    BorderCollapse,
    BorderSpacing,
    VerticalAlign,

    // Visual rendering
    Opacity,
    BorderRadius,
    BoxShadow,
    Overflow,
}

/// All known property IDs — useful for iterating over every property
/// during the defaulting pass (inherit/initial).
pub const ALL_PROPERTIES: &[PropertyId] = &[
    PropertyId::Display,
    PropertyId::Position,
    PropertyId::ZIndex,
    PropertyId::Width,
    PropertyId::Height,
    PropertyId::BoxSizing,
    PropertyId::Top,
    PropertyId::Right,
    PropertyId::Bottom,
    PropertyId::Left,
    PropertyId::MarginTop,
    PropertyId::MarginRight,
    PropertyId::MarginBottom,
    PropertyId::MarginLeft,
    PropertyId::PaddingTop,
    PropertyId::PaddingRight,
    PropertyId::PaddingBottom,
    PropertyId::PaddingLeft,
    PropertyId::BorderTopWidth,
    PropertyId::BorderRightWidth,
    PropertyId::BorderBottomWidth,
    PropertyId::BorderLeftWidth,
    PropertyId::BorderColor,
    PropertyId::BorderStyle,
    PropertyId::Color,
    PropertyId::BackgroundColor,
    PropertyId::FontSize,
    PropertyId::FontWeight,
    PropertyId::TextAlign,
    PropertyId::LineHeight,
    PropertyId::FlexDirection,
    PropertyId::FlexWrap,
    PropertyId::JustifyContent,
    PropertyId::AlignItems,
    PropertyId::AlignSelf,
    PropertyId::FlexGrow,
    PropertyId::FlexShrink,
    PropertyId::FlexBasis,
    PropertyId::GridTemplateColumns,
    PropertyId::GridTemplateRows,
    PropertyId::GridColumn,
    PropertyId::GridRow,
    PropertyId::GridColumnStart,
    PropertyId::GridColumnEnd,
    PropertyId::GridRowStart,
    PropertyId::GridRowEnd,
    PropertyId::GridGap,
    PropertyId::RowGap,
    PropertyId::ColumnGap,
    PropertyId::AnimationName,
    PropertyId::AnimationDuration,
    PropertyId::AnimationTimingFunction,
    PropertyId::AnimationIterationCount,
    // Table
    PropertyId::BorderCollapse,
    PropertyId::BorderSpacing,
    PropertyId::VerticalAlign,
    // Visual rendering
    PropertyId::Opacity,
    PropertyId::BorderRadius,
    PropertyId::BoxShadow,
    PropertyId::Overflow,
];

/// Returns true if this property is inherited by default.
///
/// Inherited properties (like color, font-size) flow from parent to child
/// when the child doesn't set them explicitly. Non-inherited properties
/// (like margin, padding, display) reset to their initial value.
///
/// Reference: https://www.w3.org/TR/CSS2/propidx.html
pub fn is_inherited(id: PropertyId) -> bool {
    match id {
        // These inherit by default per CSS spec
        PropertyId::Color => true,
        PropertyId::FontSize => true,
        PropertyId::FontWeight => true,
        PropertyId::TextAlign => true,
        PropertyId::LineHeight => true,

        // Everything else does NOT inherit
        PropertyId::Display => false,
        PropertyId::Position => false,
        PropertyId::ZIndex => false,
        PropertyId::Width => false,
        PropertyId::Height => false,
        PropertyId::BoxSizing => false,
        PropertyId::Top => false,
        PropertyId::Right => false,
        PropertyId::Bottom => false,
        PropertyId::Left => false,
        PropertyId::MarginTop => false,
        PropertyId::MarginRight => false,
        PropertyId::MarginBottom => false,
        PropertyId::MarginLeft => false,
        PropertyId::PaddingTop => false,
        PropertyId::PaddingRight => false,
        PropertyId::PaddingBottom => false,
        PropertyId::PaddingLeft => false,
        PropertyId::BorderTopWidth => false,
        PropertyId::BorderRightWidth => false,
        PropertyId::BorderBottomWidth => false,
        PropertyId::BorderLeftWidth => false,
        PropertyId::BorderColor => false,
        PropertyId::BorderStyle => false,
        PropertyId::BackgroundColor => false,
        PropertyId::FlexDirection => false,
        PropertyId::FlexWrap => false,
        PropertyId::JustifyContent => false,
        PropertyId::AlignItems => false,
        PropertyId::AlignSelf => false,
        PropertyId::FlexGrow => false,
        PropertyId::FlexShrink => false,
        PropertyId::FlexBasis => false,
        PropertyId::GridTemplateColumns => false,
        PropertyId::GridTemplateRows => false,
        PropertyId::GridColumn => false,
        PropertyId::GridRow => false,
        PropertyId::GridColumnStart => false,
        PropertyId::GridColumnEnd => false,
        PropertyId::GridRowStart => false,
        PropertyId::GridRowEnd => false,
        PropertyId::GridGap => false,
        PropertyId::RowGap => false,
        PropertyId::ColumnGap => false,
        PropertyId::AnimationName => false,
        PropertyId::AnimationDuration => false,
        PropertyId::AnimationTimingFunction => false,
        PropertyId::AnimationIterationCount => false,
        // Table (border-collapse inherits per spec)
        PropertyId::BorderCollapse => true,
        PropertyId::BorderSpacing => true,
        PropertyId::VerticalAlign => false,
        // Visual rendering
        PropertyId::Opacity => false,
        PropertyId::BorderRadius => false,
        PropertyId::BoxShadow => false,
        PropertyId::Overflow => false,
    }
}

impl PropertyId {
    /// Returns the canonical CSS property name string for this PropertyId.
    pub fn name(self) -> &'static str {
        property_id_to_name(self)
    }

    /// Parses a CSS property name string into a PropertyId.
    pub fn from_name(name: &str) -> Option<PropertyId> {
        property_from_name(name)
    }

    /// Checks if this property inherits by default.
    pub fn is_inherited(self) -> bool {
        is_inherited(self)
    }
}

/// Map a CSS property name string to a PropertyId.
/// Returns None for unknown properties (which we silently ignore in V1).
///
/// This also handles shorthand properties by returning the "primary"
/// PropertyId — actual shorthand expansion is done during value computation.
pub fn property_from_name(name: &str) -> Option<PropertyId> {
    match name {
        "display" => Some(PropertyId::Display),
        "position" => Some(PropertyId::Position),
        "z-index" => Some(PropertyId::ZIndex),
        "width" => Some(PropertyId::Width),
        "height" => Some(PropertyId::Height),
        "box-sizing" => Some(PropertyId::BoxSizing),

        // Insets
        "top" => Some(PropertyId::Top),
        "right" => Some(PropertyId::Right),
        "bottom" => Some(PropertyId::Bottom),
        "left" => Some(PropertyId::Left),

        // Longhands
        "margin-top" => Some(PropertyId::MarginTop),
        "margin-right" => Some(PropertyId::MarginRight),
        "margin-bottom" => Some(PropertyId::MarginBottom),
        "margin-left" => Some(PropertyId::MarginLeft),
        "padding-top" => Some(PropertyId::PaddingTop),
        "padding-right" => Some(PropertyId::PaddingRight),
        "padding-bottom" => Some(PropertyId::PaddingBottom),
        "padding-left" => Some(PropertyId::PaddingLeft),
        "border-top-width" => Some(PropertyId::BorderTopWidth),
        "border-right-width" => Some(PropertyId::BorderRightWidth),
        "border-bottom-width" => Some(PropertyId::BorderBottomWidth),
        "border-left-width" => Some(PropertyId::BorderLeftWidth),
        "border-color" => Some(PropertyId::BorderColor),
        "border-style" => Some(PropertyId::BorderStyle),

        "color" => Some(PropertyId::Color),
        "background-color" | "background" => Some(PropertyId::BackgroundColor),
        "font-size" => Some(PropertyId::FontSize),
        "font-weight" => Some(PropertyId::FontWeight),
        "text-align" => Some(PropertyId::TextAlign),
        "line-height" => Some(PropertyId::LineHeight),

        // Flexbox
        "flex-direction" => Some(PropertyId::FlexDirection),
        "flex-wrap" => Some(PropertyId::FlexWrap),
        "justify-content" => Some(PropertyId::JustifyContent),
        "align-items" => Some(PropertyId::AlignItems),
        "align-self" => Some(PropertyId::AlignSelf),
        "flex-grow" => Some(PropertyId::FlexGrow),
        "flex-shrink" => Some(PropertyId::FlexShrink),
        "flex-basis" => Some(PropertyId::FlexBasis),

        "grid-template-columns" => Some(PropertyId::GridTemplateColumns),
        "grid-template-rows" => Some(PropertyId::GridTemplateRows),
        "grid-column" => Some(PropertyId::GridColumn),
        "grid-row" => Some(PropertyId::GridRow),
        "grid-column-start" => Some(PropertyId::GridColumnStart),
        "grid-column-end" => Some(PropertyId::GridColumnEnd),
        "grid-row-start" => Some(PropertyId::GridRowStart),
        "grid-row-end" => Some(PropertyId::GridRowEnd),
        "grid-gap" | "gap" => Some(PropertyId::GridGap),
        "row-gap" => Some(PropertyId::RowGap),
        "column-gap" => Some(PropertyId::ColumnGap),

        "animation-name" => Some(PropertyId::AnimationName),
        "animation-duration" => Some(PropertyId::AnimationDuration),
        "animation-timing-function" => Some(PropertyId::AnimationTimingFunction),
        "animation-iteration-count" => Some(PropertyId::AnimationIterationCount),

        // Table
        "border-collapse" => Some(PropertyId::BorderCollapse),
        "border-spacing" => Some(PropertyId::BorderSpacing),
        "vertical-align" => Some(PropertyId::VerticalAlign),

        // Visual rendering
        "opacity" => Some(PropertyId::Opacity),
        "border-radius" => Some(PropertyId::BorderRadius),
        "box-shadow" => Some(PropertyId::BoxShadow),
        "overflow" => Some(PropertyId::Overflow),

        // Shorthands — handled specially in style.rs
        "margin" | "padding" | "flex" => None,
        _ => None,
    }
}

/// Map a PropertyId back to its canonical CSS property name string.
pub fn property_id_to_name(id: PropertyId) -> &'static str {
    match id {
        PropertyId::Display => "display",
        PropertyId::Position => "position",
        PropertyId::ZIndex => "z-index",
        PropertyId::Width => "width",
        PropertyId::Height => "height",
        PropertyId::BoxSizing => "box-sizing",
        PropertyId::Top => "top",
        PropertyId::Right => "right",
        PropertyId::Bottom => "bottom",
        PropertyId::Left => "left",
        PropertyId::MarginTop => "margin-top",
        PropertyId::MarginRight => "margin-right",
        PropertyId::MarginBottom => "margin-bottom",
        PropertyId::MarginLeft => "margin-left",
        PropertyId::PaddingTop => "padding-top",
        PropertyId::PaddingRight => "padding-right",
        PropertyId::PaddingBottom => "padding-bottom",
        PropertyId::PaddingLeft => "padding-left",
        PropertyId::BorderTopWidth => "border-top-width",
        PropertyId::BorderRightWidth => "border-right-width",
        PropertyId::BorderBottomWidth => "border-bottom-width",
        PropertyId::BorderLeftWidth => "border-left-width",
        PropertyId::BorderColor => "border-color",
        PropertyId::BorderStyle => "border-style",
        PropertyId::Color => "color",
        PropertyId::BackgroundColor => "background-color",
        PropertyId::FontSize => "font-size",
        PropertyId::FontWeight => "font-weight",
        PropertyId::TextAlign => "text-align",
        PropertyId::LineHeight => "line-height",
        PropertyId::FlexDirection => "flex-direction",
        PropertyId::FlexWrap => "flex-wrap",
        PropertyId::JustifyContent => "justify-content",
        PropertyId::AlignItems => "align-items",
        PropertyId::AlignSelf => "align-self",
        PropertyId::FlexGrow => "flex-grow",
        PropertyId::FlexShrink => "flex-shrink",
        PropertyId::FlexBasis => "flex-basis",
        PropertyId::GridTemplateColumns => "grid-template-columns",
        PropertyId::GridTemplateRows => "grid-template-rows",
        PropertyId::GridColumn => "grid-column",
        PropertyId::GridRow => "grid-row",
        PropertyId::GridColumnStart => "grid-column-start",
        PropertyId::GridColumnEnd => "grid-column-end",
        PropertyId::GridRowStart => "grid-row-start",
        PropertyId::GridRowEnd => "grid-row-end",
        PropertyId::GridGap => "grid-gap",
        PropertyId::RowGap => "row-gap",
        PropertyId::ColumnGap => "column-gap",
        PropertyId::AnimationName => "animation-name",
        PropertyId::AnimationDuration => "animation-duration",
        PropertyId::AnimationTimingFunction => "animation-timing-function",
        PropertyId::AnimationIterationCount => "animation-iteration-count",
        // Table
        PropertyId::BorderCollapse => "border-collapse",
        PropertyId::BorderSpacing => "border-spacing",
        PropertyId::VerticalAlign => "vertical-align",
        // Visual rendering
        PropertyId::Opacity => "opacity",
        PropertyId::BorderRadius => "border-radius",
        PropertyId::BoxShadow => "box-shadow",
        PropertyId::Overflow => "overflow",
    }
}

/// Returns true if the given property name is a shorthand that needs expansion.
/// Note: `border` and `flex` are handled by their own code paths in `style.rs` (not via `expand_shorthand`)
/// and are intentionally excluded here to preserve the is_shorthand/expand_shorthand contract:
/// every name that returns true here must also return Some(...) from expand_shorthand.
pub fn is_shorthand(name: &str) -> bool {
    matches!(name, "margin" | "padding")
}

/// Expand a shorthand property into its constituent longhand PropertyIds.
/// Returns the longhand IDs in CSS order: top, right, bottom, left.
/// Only margin and padding are handled here — border has a dedicated special path in style.rs.
pub fn expand_shorthand(name: &str) -> Option<[PropertyId; 4]> {
    match name {
        "margin" => Some([
            PropertyId::MarginTop,
            PropertyId::MarginRight,
            PropertyId::MarginBottom,
            PropertyId::MarginLeft,
        ]),
        "padding" => Some([
            PropertyId::PaddingTop,
            PropertyId::PaddingRight,
            PropertyId::PaddingBottom,
            PropertyId::PaddingLeft,
        ]),
        _ => None,
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inherited_properties() {
        assert!(is_inherited(PropertyId::Color));
        assert!(is_inherited(PropertyId::FontSize));
        assert!(is_inherited(PropertyId::FontWeight));
        assert!(is_inherited(PropertyId::TextAlign));
        assert!(is_inherited(PropertyId::LineHeight));
    }

    #[test]
    fn test_non_inherited_properties() {
        assert!(!is_inherited(PropertyId::Display));
        assert!(!is_inherited(PropertyId::Position));
        assert!(!is_inherited(PropertyId::ZIndex));
        assert!(!is_inherited(PropertyId::MarginTop));
        assert!(!is_inherited(PropertyId::PaddingTop));
        assert!(!is_inherited(PropertyId::Width));
        assert!(!is_inherited(PropertyId::BackgroundColor));
        assert!(!is_inherited(PropertyId::Top));
        assert!(!is_inherited(PropertyId::FlexDirection));
    }

    #[test]
    fn test_property_from_name() {
        assert_eq!(property_from_name("color"), Some(PropertyId::Color));
        assert_eq!(property_from_name("display"), Some(PropertyId::Display));
        assert_eq!(property_from_name("position"), Some(PropertyId::Position));
        assert_eq!(property_from_name("top"), Some(PropertyId::Top));
        assert_eq!(
            property_from_name("flex-direction"),
            Some(PropertyId::FlexDirection)
        );
        assert_eq!(
            property_from_name("justify-content"),
            Some(PropertyId::JustifyContent)
        );
        assert_eq!(property_from_name("z-index"), Some(PropertyId::ZIndex));
        assert_eq!(property_from_name("font-size"), Some(PropertyId::FontSize));
        assert_eq!(property_from_name("unknown-prop"), None);
        assert_eq!(PropertyId::ZIndex.name(), "z-index");
        assert_eq!(PropertyId::Top.name(), "top");
        assert_eq!(PropertyId::FlexDirection.name(), "flex-direction");
    }

    #[test]
    fn test_shorthand_detection() {
        assert!(is_shorthand("margin"));
        assert!(is_shorthand("padding"));
        assert!(!is_shorthand("color"));
        assert!(!is_shorthand("margin-top"));
    }

    #[test]
    fn test_shorthand_expansion() {
        let margin = expand_shorthand("margin").unwrap();
        assert_eq!(margin[0], PropertyId::MarginTop);
        assert_eq!(margin[1], PropertyId::MarginRight);
        assert_eq!(margin[2], PropertyId::MarginBottom);
        assert_eq!(margin[3], PropertyId::MarginLeft);
    }

    #[test]
    fn test_shorthand_api_contract() {
        // is_shorthand and expand_shorthand must be consistent:
        // every name where is_shorthand returns true must also return Some from expand_shorthand.
        for name in &["margin", "padding"] {
            assert!(is_shorthand(name), "{name} should be a shorthand");
            assert!(
                expand_shorthand(name).is_some(),
                "{name} must have expand_shorthand implementation"
            );
        }
        // border has its own code path; is_shorthand returns false for it
        assert!(
            !is_shorthand("border"),
            "border is handled separately, not via is_shorthand"
        );
    }

    #[test]
    fn test_all_properties_covered() {
        // Every property in ALL_PROPERTIES should have an is_inherited result
        for &prop in ALL_PROPERTIES {
            let _ = is_inherited(prop);
        }
        assert_eq!(ALL_PROPERTIES.len(), 60);
    }
}
