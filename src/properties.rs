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
    Width,
    Height,
    /// CSS box-sizing: content-box | border-box
    BoxSizing,

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

    // Grid
    GridTemplateColumns,
    GridTemplateRows,
    GridColumn,
    GridRow,
    GridGap,

    // Animation
    AnimationName,
    AnimationDuration,
    AnimationTimingFunction,
    AnimationIterationCount,
}

/// All known property IDs — useful for iterating over every property
/// during the defaulting pass (inherit/initial).
pub const ALL_PROPERTIES: &[PropertyId] = &[
    PropertyId::Display,
    PropertyId::Position,
    PropertyId::Width,
    PropertyId::Height,
    PropertyId::BoxSizing,
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
    PropertyId::GridTemplateColumns,
    PropertyId::GridTemplateRows,
    PropertyId::GridColumn,
    PropertyId::GridRow,
    PropertyId::GridGap,
    PropertyId::AnimationName,
    PropertyId::AnimationDuration,
    PropertyId::AnimationTimingFunction,
    PropertyId::AnimationIterationCount,
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
        PropertyId::Width => false,
        PropertyId::Height => false,
        PropertyId::BoxSizing => false,
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
        PropertyId::GridTemplateColumns => false,
        PropertyId::GridTemplateRows => false,
        PropertyId::GridColumn => false,
        PropertyId::GridRow => false,
        PropertyId::GridGap => false,
        PropertyId::AnimationName => false,
        PropertyId::AnimationDuration => false,
        PropertyId::AnimationTimingFunction => false,
        PropertyId::AnimationIterationCount => false,
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
        "width" => Some(PropertyId::Width),
        "height" => Some(PropertyId::Height),
        "box-sizing" => Some(PropertyId::BoxSizing),

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

        "grid-template-columns" => Some(PropertyId::GridTemplateColumns),
        "grid-template-rows" => Some(PropertyId::GridTemplateRows),
        "grid-column" => Some(PropertyId::GridColumn),
        "grid-row" => Some(PropertyId::GridRow),
        "grid-gap" | "gap" => Some(PropertyId::GridGap),

        "animation-name" => Some(PropertyId::AnimationName),
        "animation-duration" => Some(PropertyId::AnimationDuration),
        "animation-timing-function" => Some(PropertyId::AnimationTimingFunction),
        "animation-iteration-count" => Some(PropertyId::AnimationIterationCount),

        // Shorthands — handled specially in style.rs
        "margin" | "padding" => None,
        _ => None,
    }
}

/// Map a PropertyId back to its canonical CSS property name string.
pub fn property_id_to_name(id: PropertyId) -> &'static str {
    match id {
        PropertyId::Display => "display",
        PropertyId::Position => "position",
        PropertyId::Width => "width",
        PropertyId::Height => "height",
        PropertyId::BoxSizing => "box-sizing",
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
        PropertyId::GridTemplateColumns => "grid-template-columns",
        PropertyId::GridTemplateRows => "grid-template-rows",
        PropertyId::GridColumn => "grid-column",
        PropertyId::GridRow => "grid-row",
        PropertyId::GridGap => "grid-gap",
        PropertyId::AnimationName => "animation-name",
        PropertyId::AnimationDuration => "animation-duration",
        PropertyId::AnimationTimingFunction => "animation-timing-function",
        PropertyId::AnimationIterationCount => "animation-iteration-count",
    }
}

/// Returns true if the given property name is a shorthand that needs expansion.
/// Note: `border` is handled by its own code path in `style.rs` (not via `expand_shorthand`)
/// and is intentionally excluded here to preserve the is_shorthand/expand_shorthand contract:
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
        assert!(!is_inherited(PropertyId::MarginTop));
        assert!(!is_inherited(PropertyId::PaddingTop));
        assert!(!is_inherited(PropertyId::Width));
        assert!(!is_inherited(PropertyId::BackgroundColor));
    }

    #[test]
    fn test_property_from_name() {
        assert_eq!(property_from_name("color"), Some(PropertyId::Color));
        assert_eq!(property_from_name("display"), Some(PropertyId::Display));
        assert_eq!(property_from_name("font-size"), Some(PropertyId::FontSize));
        assert_eq!(property_from_name("unknown-prop"), None);
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
        assert!(!is_shorthand("border"), "border is handled separately, not via is_shorthand");
    }

    #[test]
    fn test_all_properties_covered() {
        // Every property in ALL_PROPERTIES should have an is_inherited result
        for &prop in ALL_PROPERTIES {
            let _ = is_inherited(prop);
        }
        assert_eq!(ALL_PROPERTIES.len(), 34);
    }
}
