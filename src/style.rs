use std::borrow::Cow;
use std::collections::HashMap;

use crate::css_parser::{Selector, SimpleSelector, StyleRule, Stylesheet};
use crate::dom::{Dom, NodeId, NodeKind};
use crate::properties::{self, ALL_PROPERTIES, PropertyId};
use crate::values::{self, ComputedStyle, Display};

// ─── Rule Index ──────────────────────────────────────────────────
//
// Pre-indexes stylesheet rules by the "key selector" (the rightmost /
// subject compound) so that element matching only checks candidate rules
// instead of scanning the entire stylesheet.

/// A pre-built index that buckets CSS rules by their key selector component.
///
/// For a selector like `div.main > p.intro`, the key selector is the rightmost
/// compound (`p.intro`), and the rule is indexed under both `by_tag["p"]` and
/// `by_class["intro"]`.  During matching an element only needs to check rules
/// from buckets that correspond to its own tag name, class list, and ID.
struct RuleIndex<'a> {
    by_id: HashMap<String, Vec<&'a StyleRule>>,
    by_class: HashMap<String, Vec<&'a StyleRule>>,
    by_tag: HashMap<String, Vec<&'a StyleRule>>,
    universal: Vec<&'a StyleRule>,
}

impl<'a> RuleIndex<'a> {
    /// Build a rule index from a flat list of style rules.
    ///
    /// Each rule's selectors are examined independently for their key selector
    /// (the last step's compound). A rule is indexed under every ID, class, and tag
    /// found in each selector's key compound. When a selector contributes no
    /// indexable simple selector (or has empty steps), the rule is added to the
    /// universal bucket for that selector.
    fn build(rules: &[&'a StyleRule]) -> Self {
        let mut index = RuleIndex {
            by_id: HashMap::new(),
            by_class: HashMap::new(),
            by_tag: HashMap::new(),
            universal: Vec::new(),
        };

        for &rule in rules {
            for sel in &rule.selectors {
                let mut indexed = false;

                // The key selector is the last (rightmost) step's compound.
                if let Some(last_step) = sel.steps.last() {
                    for simple in &last_step.compound {
                        match simple {
                            SimpleSelector::Id(id) => {
                                index
                                    .by_id
                                    .entry(id.to_ascii_lowercase())
                                    .or_default()
                                    .push(rule);
                                indexed = true;
                            }
                            SimpleSelector::Class(cls) => {
                                index
                                    .by_class
                                    .entry(cls.to_ascii_lowercase())
                                    .or_default()
                                    .push(rule);
                                indexed = true;
                            }
                            SimpleSelector::Tag(tag) => {
                                index
                                    .by_tag
                                    .entry(tag.to_ascii_lowercase())
                                    .or_default()
                                    .push(rule);
                                indexed = true;
                            }
                            _ => {}
                        }
                    }
                }

                // If this selector contributed no indexable ID/class/tag, add to universal.
                if !indexed {
                    index.universal.push(rule);
                }
            }
        }

        index
    }

    /// Return an iterator of candidate rules for an element with the given
    /// tag name, list of classes, and optional ID.
    fn candidates(&self, tag: &str, classes: &[&str], id: Option<&str>) -> Vec<&'a StyleRule> {
        let mut seen = std::collections::HashSet::new();
        let mut result = Vec::new();

        // Helper: add a rule if not already seen (deduplicate)
        let mut push = |rule: &'a StyleRule| {
            let ptr = rule as *const StyleRule as usize;
            if seen.insert(ptr) {
                result.push(rule);
            }
        };

        // ID bucket
        if let Some(id_val) = id
            && let Some(rules) = self.by_id.get(&id_val.to_ascii_lowercase())
        {
            for &r in rules {
                push(r);
            }
        }

        // Class buckets
        for cls in classes {
            if let Some(rules) = self.by_class.get(&cls.to_ascii_lowercase()) {
                for &r in rules {
                    push(r);
                }
            }
        }

        // Tag bucket
        if let Some(rules) = self.by_tag.get(&tag.to_ascii_lowercase()) {
            for &r in rules {
                push(r);
            }
        }

        // Universal rules always apply as candidates
        for &r in &self.universal {
            push(r);
        }

        result
    }
}

// ─── Style Resolution ────────────────────────────────────────────
//
// This module takes a DOM tree and a Stylesheet and produces a
// "styled tree" — a separate tree that mirrors the DOM structure
// but carries typed ComputedStyle on each element node.
//
// The pipeline for each element:
//   1. Collect all matching rules from the stylesheet
//   2. Calculate specificity for each matching selector
//   3. Sort by cascade priority: (origin, specificity, source_order)
//   4. For each property, pick the winning declaration
//   5. Apply shorthand expansion (margin → margin-top/right/bottom/left)
//   6. Default unset properties: inherited → copy parent, non-inherited → initial
//   7. Resolve font-size first (em/% depend on parent's font-size)
//   8. Compute absolute values for all other properties (em → px, colors, etc.)
//
// This is V1 — does NOT include:
//   - Bloom filter optimization (step 2/5 of production engines)
//   - Style sharing cache
//   - !important support
//   - var() / custom properties
//   - Pseudo-elements (::before, ::after)

// ─── Specificity ─────────────────────────────────────────────────

/// CSS specificity as (id_count, class_count, tag_count).
/// Higher tuple wins. Compared lexicographically (ids beat classes beat tags).
pub type Specificity = (u32, u32, u32);

/// Calculate the specificity of a selector.
///
/// For each simple selector across all compound parts:
///   - Id(#foo)       → increments id_count
///   - Class(.bar)    → increments class_count
///   - Tag(div)       → increments tag_count
///   - Universal(*)   → contributes nothing
pub fn compute_specificity(selector: &Selector) -> Specificity {
    let mut ids = 0u32;
    let mut classes = 0u32;
    let mut tags = 0u32;

    for compound in &selector.parts {
        for simple in compound {
            match simple {
                SimpleSelector::Id(_) => ids += 1,
                SimpleSelector::Class(_)
                | SimpleSelector::PseudoClass(_)
                | SimpleSelector::Attribute(_, _) => classes += 1,
                SimpleSelector::Tag(_) => tags += 1,
                SimpleSelector::Universal => {} // contributes 0
            }
        }
    }

    (ids, classes, tags)
}

// ─── Cascade Types ───────────────────────────────────────────────

/// The origin of a CSS declaration — determines cascade priority.
/// Higher numeric value = higher priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// Stylesheet rules (author origin)
    Author = 0,
    /// Inline style="" attribute (always beats author)
    Inline = 1,
}

/// A single declaration that matched an element, along with its
/// cascade metadata for sorting. Zero-copy with Cow<'a, str>.
#[derive(Debug)]
struct MatchedDeclaration<'a> {
    property: Cow<'a, str>,
    value: Cow<'a, str>,
    specificity: Specificity,
    source_order: usize,
    origin: Origin,
    /// True if the declaration has `!important` annotation.
    important: bool,
}

// ─── Styled Node ─────────────────────────────────────────────────

/// A node in the styled tree. Mirrors the DOM structure but carries
/// a fully resolved ComputedStyle attached to each element.
#[derive(Debug)]
pub struct StyledNode {
    /// Which DOM node this styled node corresponds to
    pub node_id: NodeId,
    /// Computed styles for this node (fully resolved, typed values)
    pub styles: ComputedStyle,
    /// Styled children — same order as DOM children
    pub children: Vec<StyledNode>,
}

// ─── Style Resolution Entry Point ────────────────────────────────

/// Default root font size in px (browser standard).
const ROOT_FONT_SIZE: f32 = 16.0;

/// Resolve styles for the entire DOM tree.
/// Returns a StyledNode tree rooted at the Document node.
///
/// `dom` — the parsed DOM tree
/// `stylesheet` — the parsed CSS stylesheet
/// `source` — the original HTML source buffer (needed to read tag names and attributes)
/// Check if an HTML tag defaults to display: block in User-Agent stylesheet
fn is_default_block_tag(tag: &str) -> bool {
    matches!(
        tag.to_ascii_lowercase().as_str(),
        // Document structure
        "html"
            | "body"
            | "div"
            | "p"
            // Headings
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            // Sectioning
            | "header"
            | "footer"
            | "section"
            | "article"
            | "nav"
            | "main"
            | "aside"
            // Lists
            | "ul"
            | "ol"
            | "li"
            | "dl"
            | "dt"
            | "dd"
            // Semantic HTML5
            | "blockquote"
            | "pre"
            | "figure"
            | "figcaption"
            | "details"
            | "summary"
            | "address"
            | "fieldset"
            | "legend"
            // Table-level (display: table etc. handled separately; these are at minimum block)
            | "table"
            | "thead"
            | "tbody"
            | "tfoot"
            | "tr"
            | "caption"
            // Forms
            | "form"
            // Misc block
            | "hr"
    )
}

/// Check if an HTML tag defaults to display: inline-block in User-Agent stylesheet
fn is_default_inline_block_tag(tag: &str) -> bool {
    matches!(
        tag.to_ascii_lowercase().as_str(),
        "img"
            | "input"
            | "button"
            | "select"
            | "textarea"
            | "video"
            | "audio"
            | "canvas"
            | "iframe"
            | "embed"
            | "object"
    )
}

pub fn resolve_styles(dom: &Dom, stylesheet: &Stylesheet, source: &[u8]) -> StyledNode {
    resolve_styles_with_viewport(dom, stylesheet, source, 800.0)
}

pub fn resolve_styles_with_viewport(
    dom: &Dom,
    stylesheet: &Stylesheet,
    source: &[u8],
    viewport_width: f32,
) -> StyledNode {
    // ── Build the rule index once for the entire style resolution pass ──
    // Collect all applicable rules (top-level + matching @media rules)
    let mut all_rules: Vec<&StyleRule> = stylesheet.rules.iter().collect();
    for media in &stylesheet.media_rules {
        let matches_min = media.min_width.is_none_or(|mw| viewport_width >= mw);
        let matches_max = media.max_width.is_none_or(|mw| viewport_width <= mw);
        if matches_min && matches_max {
            all_rules.extend(media.rules.iter());
        }
    }
    let rule_index = RuleIndex::build(&all_rules);

    let root_style = ComputedStyle::default();
    build_styled_node(
        dom,
        dom.root(),
        &rule_index,
        source,
        &root_style,
        ROOT_FONT_SIZE,
        viewport_width,
    )
}

/// Split the inner content of a `var()` call into the variable name and an
/// optional fallback value, correctly handling nested parentheses.
/// e.g. `"--x, calc(1px + 2px)"` → `("--x", Some("calc(1px + 2px)"))`.
fn split_var_args(inner: &str) -> (&str, Option<String>) {
    let mut depth = 0;
    for (i, ch) in inner.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                let name = inner[..i].trim();
                let fallback = inner[i + 1..].trim();
                return (name, Some(fallback.to_string()));
            }
            _ => {}
        }
    }
    (inner.trim(), None)
}

/// Recursively build a StyledNode for a DOM node and its descendants.
///
/// `parent_style` — the parent's computed style (for inheritance)
/// `root_font_size` — the root element's computed font-size (for rem units)
fn build_styled_node(
    dom: &Dom,
    node_id: NodeId,
    rule_index: &RuleIndex,
    source: &[u8],
    parent_style: &ComputedStyle,
    root_font_size: f32,
    _viewport_width: f32,
) -> StyledNode {
    let node = dom.get(node_id);

    // Compute styles for this node (only Element nodes get matched)
    let styles = match &node.kind {
        NodeKind::Element { .. } => {
            // ── Step 1: Collect all matching declarations ──────────
            let mut declarations = Vec::new();

            // Retrieve the element's tag, classes, and ID for indexed lookup
            let tag = node.tag_name(source).to_ascii_lowercase();
            let id = node.get_id(source);
            let class_attr = node.get_attribute("class", source).unwrap_or("");
            let classes: Vec<&str> = class_attr.split_whitespace().collect();

            // Query only candidate rules from the index (not the full stylesheet)
            let candidate_rules = rule_index.candidates(&tag, &classes, id);

            for rule in &candidate_rules {
                // Find the highest-specificity selector that matches
                let mut best_specificity: Option<Specificity> = None;

                for sel in &rule.selectors {
                    if selector_matches(sel, node_id, dom, source) {
                        let spec = compute_specificity(sel);
                        match best_specificity {
                            None => best_specificity = Some(spec),
                            Some(prev) if spec > prev => best_specificity = Some(spec),
                            _ => {}
                        }
                    }
                }

                if let Some(specificity) = best_specificity {
                    for decl in &rule.declarations {
                        // Strip !important flag from value and record it
                        let (clean_value, important) = strip_important(decl.value.as_str());
                        declarations.push(MatchedDeclaration {
                            property: Cow::Borrowed(&decl.property),
                            value: Cow::Owned(clean_value),
                            specificity,
                            source_order: rule.position,
                            origin: Origin::Author,
                            important,
                        });
                    }
                }
            }

            // Check for inline style="" attribute (highest cascade priority)
            if let Some(style_text) = node.get_attribute("style", source) {
                let inline_decls = parse_inline_style(style_text);
                for (prop, val) in inline_decls {
                    let (clean_val, important) = strip_important(&val);
                    declarations.push(MatchedDeclaration {
                        property: Cow::Owned(prop),
                        value: Cow::Owned(clean_val),
                        specificity: (0, 0, 0), // doesn't matter — origin wins
                        source_order: usize::MAX,
                        origin: Origin::Inline,
                        important,
                    });
                }
            }

            // ── Step 2: Expand shorthands before cascade sorting ──
            // Every shorthand declaration generates longhand declarations retaining
            // the exact same specificity, origin, source_order, and important metadata.
            let mut normalized_decls = Vec::with_capacity(declarations.len());
            for decl in declarations {
                let prop = decl.property.as_ref();
                let val_trimmed = decl.value.trim();
                let val_lower = val_trimmed.to_ascii_lowercase();
                let is_css_wide =
                    val_lower == "inherit" || val_lower == "initial" || val_lower == "unset";

                if prop == "z-index" {
                    if !is_css_wide
                        && !val_trimmed.contains("var(")
                        && values::try_parse_z_index(val_trimmed).is_none()
                    {
                        // Discard invalid z-index declaration before cascade sorting
                        continue;
                    }
                    normalized_decls.push(decl);
                } else if prop == "margin" || prop == "padding" {
                    let prefix = if prop == "margin" {
                        "margin"
                    } else {
                        "padding"
                    };
                    if is_css_wide {
                        for edge in &["top", "right", "bottom", "left"] {
                            normalized_decls.push(MatchedDeclaration {
                                property: Cow::Owned(format!("{}-{}", prefix, edge)),
                                value: Cow::Owned(val_lower.clone()),
                                specificity: decl.specificity,
                                source_order: decl.source_order,
                                origin: decl.origin,
                                important: decl.important,
                            });
                        }
                    } else {
                        let parts: Vec<&str> = val_trimmed.split_whitespace().collect();
                        let (top, right, bottom, left) = match parts.len() {
                            1 => (parts[0], parts[0], parts[0], parts[0]),
                            2 => (parts[0], parts[1], parts[0], parts[1]),
                            3 => (parts[0], parts[1], parts[2], parts[1]),
                            4 => (parts[0], parts[1], parts[2], parts[3]),
                            _ => ("0px", "0px", "0px", "0px"),
                        };
                        for (edge, val) in &[
                            ("top", top),
                            ("right", right),
                            ("bottom", bottom),
                            ("left", left),
                        ] {
                            normalized_decls.push(MatchedDeclaration {
                                property: Cow::Owned(format!("{}-{}", prefix, edge)),
                                value: Cow::Owned(val.to_string()),
                                specificity: decl.specificity,
                                source_order: decl.source_order,
                                origin: decl.origin,
                                important: decl.important,
                            });
                        }
                    }
                } else if prop == "border" {
                    if is_css_wide {
                        for edge_name in &[
                            "border-top-width",
                            "border-right-width",
                            "border-bottom-width",
                            "border-left-width",
                        ] {
                            normalized_decls.push(MatchedDeclaration {
                                property: Cow::Borrowed(edge_name),
                                value: Cow::Owned(val_lower.clone()),
                                specificity: decl.specificity,
                                source_order: decl.source_order,
                                origin: decl.origin,
                                important: decl.important,
                            });
                        }
                        normalized_decls.push(MatchedDeclaration {
                            property: Cow::Borrowed("border-style"),
                            value: Cow::Owned(val_lower.clone()),
                            specificity: decl.specificity,
                            source_order: decl.source_order,
                            origin: decl.origin,
                            important: decl.important,
                        });
                        normalized_decls.push(MatchedDeclaration {
                            property: Cow::Borrowed("border-color"),
                            value: Cow::Owned(val_lower),
                            specificity: decl.specificity,
                            source_order: decl.source_order,
                            origin: decl.origin,
                            important: decl.important,
                        });
                    } else {
                        let (w, s, c) = values::parse_border_shorthand(val_trimmed);
                        if let Some(w_val) = w {
                            for edge_name in &[
                                "border-top-width",
                                "border-right-width",
                                "border-bottom-width",
                                "border-left-width",
                            ] {
                                normalized_decls.push(MatchedDeclaration {
                                    property: Cow::Borrowed(edge_name),
                                    value: Cow::Owned(w_val.clone()),
                                    specificity: decl.specificity,
                                    source_order: decl.source_order,
                                    origin: decl.origin,
                                    important: decl.important,
                                });
                            }
                        }
                        if let Some(s_val) = s {
                            normalized_decls.push(MatchedDeclaration {
                                property: Cow::Borrowed("border-style"),
                                value: Cow::Owned(s_val),
                                specificity: decl.specificity,
                                source_order: decl.source_order,
                                origin: decl.origin,
                                important: decl.important,
                            });
                        }
                        if let Some(c_val) = c {
                            normalized_decls.push(MatchedDeclaration {
                                property: Cow::Borrowed("border-color"),
                                value: Cow::Owned(c_val),
                                specificity: decl.specificity,
                                source_order: decl.source_order,
                                origin: decl.origin,
                                important: decl.important,
                            });
                        }
                    }
                } else if prop == "flex" {
                    if is_css_wide {
                        for longhand in &["flex-grow", "flex-shrink", "flex-basis"] {
                            normalized_decls.push(MatchedDeclaration {
                                property: Cow::Borrowed(longhand),
                                value: Cow::Owned(val_lower.clone()),
                                specificity: decl.specificity,
                                source_order: decl.source_order,
                                origin: decl.origin,
                                important: decl.important,
                            });
                        }
                    } else {
                        let (grow, shrink, basis) = values::parse_flex_shorthand(val_trimmed);
                        normalized_decls.push(MatchedDeclaration {
                            property: Cow::Borrowed("flex-grow"),
                            value: Cow::Owned(grow),
                            specificity: decl.specificity,
                            source_order: decl.source_order,
                            origin: decl.origin,
                            important: decl.important,
                        });
                        normalized_decls.push(MatchedDeclaration {
                            property: Cow::Borrowed("flex-shrink"),
                            value: Cow::Owned(shrink),
                            specificity: decl.specificity,
                            source_order: decl.source_order,
                            origin: decl.origin,
                            important: decl.important,
                        });
                        normalized_decls.push(MatchedDeclaration {
                            property: Cow::Borrowed("flex-basis"),
                            value: Cow::Owned(basis),
                            specificity: decl.specificity,
                            source_order: decl.source_order,
                            origin: decl.origin,
                            important: decl.important,
                        });
                    }
                } else if prop == "gap" {
                    normalized_decls.push(MatchedDeclaration {
                        property: Cow::Borrowed("grid-gap"),
                        value: decl.value,
                        specificity: decl.specificity,
                        source_order: decl.source_order,
                        origin: decl.origin,
                        important: decl.important,
                    });
                } else {
                    normalized_decls.push(decl);
                }
            }

            // ── Step 3: Sort by cascade priority ──────────────────
            // Sort order (ascending, last wins):
            //   1. !important status (normal < important)
            //   2. Origin (Author < Inline)
            //   3. Specificity
            //   4. Source order
            normalized_decls.sort_by(|a, b| {
                a.important
                    .cmp(&b.important)
                    .then(a.origin.cmp(&b.origin))
                    .then(a.specificity.cmp(&b.specificity))
                    .then(a.source_order.cmp(&b.source_order))
            });

            // ── Step 4: Pick winners per property ─────────────────
            // Last declaration for each property wins (since sorted ascending)
            let mut specified: HashMap<Cow<str>, Cow<str>> = HashMap::new();
            for decl in normalized_decls {
                specified.insert(decl.property, decl.value);
            }

            // ── Step 5: Build ComputedStyle with inheritance ──────
            let mut current_variables = parent_style.variables.clone();

            // Extract and update custom properties from specified
            for (prop, value) in &specified {
                if prop.starts_with("--") {
                    current_variables.insert(prop.to_string(), value.to_string());
                }
            }

            // A helper to substitute `var()` in a value string.
            // Uses paren-aware scanning to correctly handle nested parens in
            // fallback values (e.g. `var(--x, calc(1px + 2px))`).
            // Detects cycles via a visited set to prevent infinite loops.
            let substitute_vars =
                |val: &str, vars: &std::collections::HashMap<String, String>| -> String {
                    if !val.contains("var(") {
                        return val.to_string();
                    }
                    let mut result = val.to_string();
                    let mut iterations = 0;
                    const MAX_VAR_DEPTH: usize = 32;
                    let mut visited: std::collections::HashSet<String> =
                        std::collections::HashSet::new();

                    while let Some(start) = result.find("var(") {
                        iterations += 1;
                        if iterations > MAX_VAR_DEPTH {
                            break; // Safety limit
                        }

                        // Find matching close paren using depth tracking
                        let inner_start = start + 4;
                        let mut depth = 1;
                        let mut end_pos = None;
                        for (i, ch) in result[inner_start..].char_indices() {
                            match ch {
                                '(' => depth += 1,
                                ')' => {
                                    depth -= 1;
                                    if depth == 0 {
                                        end_pos = Some(inner_start + i);
                                        break;
                                    }
                                }
                                _ => {}
                            }
                        }

                        let end = match end_pos {
                            Some(e) => e,
                            None => break, // Unmatched paren
                        };

                        let var_inner = result[inner_start..end].trim().to_string();

                        // Split on the first comma (paren-aware) for fallback
                        let (var_name, fallback) = split_var_args(&var_inner);
                        let var_name = var_name.trim();

                        // Cycle detection
                        if visited.contains(var_name) {
                            // Cycle detected — use fallback or empty string
                            let resolved = fallback.unwrap_or_default();
                            result.replace_range(start..=end, &resolved);
                            continue;
                        }
                        visited.insert(var_name.to_string());

                        let resolved_val = if let Some(v) = vars.get(var_name) {
                            v.clone()
                        } else {
                            fallback.unwrap_or_default()
                        };

                        result.replace_range(start..=end, &resolved_val);
                    }
                    result
                };

            let mut computed = ComputedStyle::default();

            // First: resolve font-size (other em values depend on it)
            if let Some(raw_fs_value) = specified.get("font-size") {
                let fs_value = substitute_vars(raw_fs_value, &current_variables);
                if fs_value == "inherit" {
                    computed.font_size = parent_style.font_size;
                } else if fs_value == "initial" {
                    computed.font_size = 16.0;
                } else {
                    computed.font_size =
                        values::parse_length(&fs_value, parent_style.font_size, root_font_size);
                }
            } else if properties::is_inherited(PropertyId::FontSize) {
                // font-size inherits — copy from parent
                computed.font_size = parent_style.font_size;
            }
            // else: keep default (16.0)

            // Update line-height default based on resolved font-size
            computed.line_height = computed.font_size * 1.2;

            // Now resolve all other properties
            for &prop_id in ALL_PROPERTIES {
                if prop_id == PropertyId::FontSize {
                    continue; // already handled above
                }

                let prop_name = prop_id.name();

                let maybe_raw = specified.get(prop_name).or_else(|| {
                    if prop_id == PropertyId::GridGap {
                        specified.get("gap")
                    } else {
                        None
                    }
                });

                if let Some(raw_value) = maybe_raw {
                    let value = substitute_vars(raw_value, &current_variables);
                    if value == "inherit" {
                        copy_property(&mut computed, parent_style, prop_id);
                    } else if value == "initial" {
                        // keep initial from Default impl
                    } else {
                        computed.set_property(
                            prop_id,
                            &value,
                            parent_style.font_size,
                            root_font_size,
                        );
                    }
                } else if properties::is_inherited(prop_id) {
                    copy_property(&mut computed, parent_style, prop_id);
                }
            }

            // Assign the resolved variables to the computed style
            computed.variables = current_variables.clone();

            // Resolve `currentColor` keyword for border_color / background_color
            if let Some(raw_bc) = specified.get("border-color") {
                let bc = substitute_vars(raw_bc, &current_variables);
                if values::try_parse_css_color(&bc) == Some(values::CssColor::CurrentColor) {
                    computed.border_color = computed.color;
                }
            }
            if let Some(raw_bg) = specified.get("background-color") {
                let bg = substitute_vars(raw_bg, &current_variables);
                if values::try_parse_css_color(&bg) == Some(values::CssColor::CurrentColor) {
                    computed.background_color = computed.color;
                }
            }

            // User-Agent default stylesheet: apply tag-specific defaults for un-specified properties
            if let NodeKind::Element { .. } = &node.kind {
                let tag_name = node.tag_name(source).to_ascii_lowercase();
                apply_user_agent_defaults(&tag_name, &specified, &mut computed);
            }

            computed
        }
        _ => {
            // Text/Comment/Document nodes inherit everything from parent
            let mut computed = ComputedStyle::default();
            if matches!(node.kind, NodeKind::Document) {
                computed.display = Display::Block;
            }
            for &prop_id in ALL_PROPERTIES {
                if properties::is_inherited(prop_id) {
                    copy_property(&mut computed, parent_style, prop_id);
                }
            }
            computed
        }
    };

    // Recurse into children, passing our computed style as parent
    let children = node
        .children
        .iter()
        .map(|&child_id| {
            build_styled_node(
                dom,
                child_id,
                rule_index,
                source,
                &styles,
                root_font_size,
                _viewport_width,
            )
        })
        .collect();

    StyledNode {
        node_id,
        styles,
        children,
    }
}

/// User-Agent default stylesheet: apply tag-specific defaults for un-specified properties
fn apply_user_agent_defaults(
    tag_name: &str,
    specified: &HashMap<Cow<str>, Cow<str>>,
    computed: &mut ComputedStyle,
) {
    if !specified.contains_key("display") {
        match tag_name {
            "head" | "title" | "meta" | "script" | "style" | "link" | "noscript" => {
                computed.display = Display::None;
            }
            _ if is_default_inline_block_tag(tag_name) => {
                computed.display = Display::InlineBlock;
            }
            _ if is_default_block_tag(tag_name) => {
                computed.display = Display::Block;
            }
            _ => {}
        }
    }

    // Default sizing for replaced / form / media elements
    if computed.width.is_auto() && !specified.contains_key("width") {
        match tag_name {
            "img" => computed.width = values::LengthOrPercentage::Px(160.0),
            "video" | "canvas" => computed.width = values::LengthOrPercentage::Px(300.0),
            "iframe" => computed.width = values::LengthOrPercentage::Px(300.0),
            "textarea" => computed.width = values::LengthOrPercentage::Px(200.0),
            "select" | "input" | "button" => computed.width = values::LengthOrPercentage::Px(120.0),
            _ => {}
        }
    }
    if computed.height.is_auto() && !specified.contains_key("height") {
        match tag_name {
            "img" => computed.height = values::LengthOrPercentage::Px(100.0),
            "video" => computed.height = values::LengthOrPercentage::Px(150.0),
            "canvas" => computed.height = values::LengthOrPercentage::Px(150.0),
            "iframe" => computed.height = values::LengthOrPercentage::Px(150.0),
            "textarea" => computed.height = values::LengthOrPercentage::Px(80.0),
            "select" | "input" | "button" => computed.height = values::LengthOrPercentage::Px(24.0),
            _ => {}
        }
    }

    if !specified.contains_key("background-color") {
        match tag_name {
            "body" | "div" => {
                computed.background_color = values::Color::rgb(248, 250, 252);
            }
            "h1" => {
                computed.background_color = values::Color::rgb(240, 249, 255);
            }
            "img" => {
                computed.background_color = values::Color::rgb(226, 232, 240);
            }
            _ => {}
        }
    }

    if !specified.contains_key("color")
        && computed.color == values::Color::BLACK
        && tag_name == "h1"
    {
        computed.color = values::Color::rgb(3, 105, 161);
    }

    if !specified.contains_key("border") && !specified.contains_key("border-color") {
        match tag_name {
            "h1" => {
                computed.border_color = values::Color::rgb(2, 132, 199);
            }
            "div" | "img" | "hr" => {
                computed.border_color = values::Color::rgb(203, 213, 225);
            }
            _ => {}
        }
    }

    if !specified.contains_key("border")
        && !specified.contains_key("border-width")
        && !specified.contains_key("border-left-width")
        && !specified.contains_key("border-top-width")
        && !specified.contains_key("border-right-width")
        && !specified.contains_key("border-bottom-width")
    {
        match tag_name {
            "h1" => {
                computed.border_width.left = 4.0;
            }
            "div" | "img" | "hr" => {
                computed.border_width = values::Edges::uniform(1.0);
            }
            _ => {}
        }
    }

    if !specified.contains_key("border")
        && !specified.contains_key("border-style")
        && tag_name == "hr"
    {
        computed.border_style = values::BorderStyleValue::Solid;
    }

    if computed.height.is_auto() && !specified.contains_key("height") && tag_name == "hr" {
        computed.height = values::LengthOrPercentage::Px(0.0);
    }

    if tag_name == "hr" {
        if !specified.contains_key("margin") && !specified.contains_key("margin-top") {
            computed.margin.top = Some(8.0);
        }
        if !specified.contains_key("margin") && !specified.contains_key("margin-bottom") {
            computed.margin.bottom = Some(8.0);
        }
    }

    if !specified.contains_key("margin")
        && !specified.contains_key("margin-top")
        && tag_name == "body"
    {
        computed.margin = values::Margin::uniform(8.0);
    }

    if !specified.contains_key("padding")
        && !specified.contains_key("padding-top")
        && (tag_name == "h1" || tag_name == "div")
    {
        computed.padding = values::Edges::uniform(12.0);
    }
}

/// Copy a single CSS property value from parent to child style.
///
/// This function serves two purposes:
///
/// 1. **Default inheritance**: For CSS properties that inherit by default
///    (e.g., `color`, `font-size`, `text-align`), this is called when no
///    explicit value is specified on the element. The child automatically
///    inherits the parent's computed value.
///
/// 2. **Explicit `inherit` keyword**: For ANY property (including non-inherited
///    ones like `display`, `width`, `margin`), when the CSS value is literally
///    `"inherit"`, this function copies the parent's value to the child.
///    This is why non-inherited properties like `Display` and `Width` have
///    match arms here — they're needed for the `inherit` keyword to work.
///
/// See CSS Cascading and Inheritance Level 4 §7.1 for the full specification.
fn copy_property(child: &mut ComputedStyle, parent: &ComputedStyle, prop: PropertyId) {
    match prop {
        PropertyId::Display => child.display = parent.display,
        PropertyId::Position => child.position = parent.position,
        PropertyId::ZIndex => child.z_index = parent.z_index,
        PropertyId::Width => child.width = parent.width,
        PropertyId::Height => child.height = parent.height,
        PropertyId::BoxSizing => child.box_sizing = parent.box_sizing,
        PropertyId::MarginTop => child.margin.top = parent.margin.top,
        PropertyId::MarginRight => child.margin.right = parent.margin.right,
        PropertyId::MarginBottom => child.margin.bottom = parent.margin.bottom,
        PropertyId::MarginLeft => child.margin.left = parent.margin.left,
        PropertyId::PaddingTop => child.padding.top = parent.padding.top,
        PropertyId::PaddingRight => child.padding.right = parent.padding.right,
        PropertyId::PaddingBottom => child.padding.bottom = parent.padding.bottom,
        PropertyId::PaddingLeft => child.padding.left = parent.padding.left,
        PropertyId::BorderTopWidth => child.border_width.top = parent.border_width.top,
        PropertyId::BorderRightWidth => child.border_width.right = parent.border_width.right,
        PropertyId::BorderBottomWidth => child.border_width.bottom = parent.border_width.bottom,
        PropertyId::BorderLeftWidth => child.border_width.left = parent.border_width.left,
        PropertyId::BorderColor => child.border_color = parent.border_color,
        PropertyId::BorderStyle => child.border_style = parent.border_style,
        PropertyId::Color => child.color = parent.color,
        PropertyId::BackgroundColor => child.background_color = parent.background_color,
        PropertyId::FontSize => child.font_size = parent.font_size,
        PropertyId::FontWeight => child.font_weight = parent.font_weight,
        PropertyId::TextAlign => child.text_align = parent.text_align,
        PropertyId::LineHeight => child.line_height = parent.line_height,
        PropertyId::GridTemplateColumns => {
            child.grid_template_columns = parent.grid_template_columns.clone()
        }
        PropertyId::GridTemplateRows => {
            child.grid_template_rows = parent.grid_template_rows.clone()
        }
        PropertyId::GridColumn => child.grid_column = parent.grid_column.clone(),
        PropertyId::GridRow => child.grid_row = parent.grid_row.clone(),
        PropertyId::GridGap => child.grid_gap = parent.grid_gap,
        PropertyId::AnimationName => child.animation_name = parent.animation_name.clone(),
        PropertyId::AnimationDuration => child.animation_duration = parent.animation_duration,
        PropertyId::AnimationTimingFunction => {
            child.animation_timing_function = parent.animation_timing_function.clone()
        }
        PropertyId::AnimationIterationCount => {
            child.animation_iteration_count = parent.animation_iteration_count
        }
        PropertyId::Top => child.top = parent.top,
        PropertyId::Right => child.right = parent.right,
        PropertyId::Bottom => child.bottom = parent.bottom,
        PropertyId::Left => child.left = parent.left,
        PropertyId::FlexDirection => child.flex_direction = parent.flex_direction,
        PropertyId::FlexWrap => child.flex_wrap = parent.flex_wrap,
        PropertyId::JustifyContent => child.justify_content = parent.justify_content,
        PropertyId::AlignItems => child.align_items = parent.align_items,
        PropertyId::AlignSelf => child.align_self = parent.align_self,
        PropertyId::FlexGrow => child.flex_grow = parent.flex_grow,
        PropertyId::FlexShrink => child.flex_shrink = parent.flex_shrink,
        PropertyId::FlexBasis => child.flex_basis = parent.flex_basis,
    }
}

/// Strip `!important` annotation from a CSS value string.
/// Returns `(clean_value, was_important)`.
/// e.g. `"red !important"` → `("red", true)`
fn strip_important(value: &str) -> (String, bool) {
    let trimmed = value.trim();
    if let Some(without) = trimmed.strip_suffix("!important") {
        return (without.trim().to_string(), true);
    }
    // Handle `! important` with a space
    if let Some(without) = trimmed.strip_suffix("important") {
        let without = without.trim();
        if let Some(stripped) = without.strip_suffix('!') {
            return (stripped.trim().to_string(), true);
        }
    }
    (trimmed.to_string(), false)
}

/// NOTE: This implementation naively splits on ';' which will break if a
/// property value contains a semicolon (e.g. `content: "a;b"` or data URIs).
/// A more robust solution would use the CSS tokenizer to parse inline styles.
/// Parse inline style declarations from a style="" attribute value string.
fn parse_inline_style(style_str: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();

    for declaration in style_str.split(';') {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }

        if let Some((prop, val)) = declaration.split_once(':') {
            let property = prop.trim().to_ascii_lowercase();
            let value = val.trim().to_string();
            if !property.is_empty() && !value.is_empty() {
                result.push((property, value));
            }
        }
    }

    result
}

// ─── Selector Matching ───────────────────────────────────────────

/// Check if a selector matches a DOM node.
fn selector_matches(selector: &Selector, node_id: NodeId, dom: &Dom, source: &[u8]) -> bool {
    if !selector.steps.is_empty() {
        return selector_steps_match(&selector.steps, node_id, dom, source);
    }

    if selector.parts.is_empty() {
        return false;
    }

    // Fallback: build steps with descendant combinators if only legacy parts are present
    let steps: Vec<crate::css_parser::SelectorStep> = selector
        .parts
        .iter()
        .map(|p| crate::css_parser::SelectorStep {
            combinator: crate::css_parser::Combinator::Descendant,
            compound: p.clone(),
        })
        .collect();
    selector_steps_match(&steps, node_id, dom, source)
}

fn match_selector_from_step(
    steps: &[crate::css_parser::SelectorStep],
    step_idx: usize,
    current_node: NodeId,
    dom: &Dom,
    source: &[u8],
) -> bool {
    let current_step = &steps[step_idx];

    if !compound_matches(&current_step.compound, current_node, dom, source) {
        return false;
    }

    if step_idx == 0 {
        return true;
    }

    let combinator = current_step.combinator;
    let prev_step_idx = step_idx - 1;

    match combinator {
        crate::css_parser::Combinator::Child => {
            if let Some(parent_id) = dom.get(current_node).parent {
                match_selector_from_step(steps, prev_step_idx, parent_id, dom, source)
            } else {
                false
            }
        }
        crate::css_parser::Combinator::Descendant => {
            let mut parent_opt = dom.get(current_node).parent;
            while let Some(parent_id) = parent_opt {
                if match_selector_from_step(steps, prev_step_idx, parent_id, dom, source) {
                    return true;
                }
                parent_opt = dom.get(parent_id).parent;
            }
            false
        }
        crate::css_parser::Combinator::NextSibling => {
            if let Some(sibling_id) = get_previous_element_sibling(current_node, dom) {
                match_selector_from_step(steps, prev_step_idx, sibling_id, dom, source)
            } else {
                false
            }
        }
        crate::css_parser::Combinator::SubsequentSibling => {
            let mut sibling_opt = get_previous_element_sibling(current_node, dom);
            while let Some(sibling_id) = sibling_opt {
                if match_selector_from_step(steps, prev_step_idx, sibling_id, dom, source) {
                    return true;
                }
                sibling_opt = get_previous_element_sibling(sibling_id, dom);
            }
            false
        }
    }
}

fn selector_steps_match(
    steps: &[crate::css_parser::SelectorStep],
    node_id: NodeId,
    dom: &Dom,
    source: &[u8],
) -> bool {
    if steps.is_empty() {
        return false;
    }
    match_selector_from_step(steps, steps.len() - 1, node_id, dom, source)
}

fn get_previous_element_sibling(node_id: NodeId, dom: &Dom) -> Option<NodeId> {
    let node = dom.get(node_id);
    let parent_id = node.parent?;
    let parent = dom.get(parent_id);
    let idx = parent.children.iter().position(|&child| child == node_id)?;
    parent.children[..idx]
        .iter()
        .rev()
        .copied()
        .find(|&child_id| matches!(dom.get(child_id).kind, NodeKind::Element { .. }))
}

fn is_first_child(node_id: NodeId, dom: &Dom) -> bool {
    let node = dom.get(node_id);
    if let Some(parent_id) = node.parent {
        let parent = dom.get(parent_id);
        let first_elem = parent
            .children
            .iter()
            .find(|&&child_id| matches!(dom.get(child_id).kind, NodeKind::Element { .. }));
        first_elem == Some(&node_id)
    } else {
        false
    }
}

fn is_last_child(node_id: NodeId, dom: &Dom) -> bool {
    let node = dom.get(node_id);
    if let Some(parent_id) = node.parent {
        let parent = dom.get(parent_id);
        let last_elem = parent
            .children
            .iter()
            .rev()
            .find(|&&child_id| matches!(dom.get(child_id).kind, NodeKind::Element { .. }));
        last_elem == Some(&node_id)
    } else {
        false
    }
}

fn node_has_attribute(
    node: &crate::dom::Node,
    attr_name: &str,
    expected_val: &Option<(String, String)>,
    source: &[u8],
) -> bool {
    if let Some(actual_val) = node.get_attribute(attr_name, source) {
        if let Some((op, val)) = expected_val {
            match op.as_str() {
                "=" => actual_val == val,
                "^=" => actual_val.starts_with(val.as_str()),
                "$=" => actual_val.ends_with(val.as_str()),
                "*=" => actual_val.contains(val.as_str()),
                "~=" => actual_val.split_whitespace().any(|word| word == val),
                "|=" => actual_val == val || actual_val.starts_with(&format!("{}-", val)),
                _ => false,
            }
        } else {
            true
        }
    } else {
        false
    }
}

/// Check if all simple selectors in a compound selector match a node.
/// ALL of them must match (it's an AND — e.g. div.main means both Tag and Class).
fn compound_matches(
    compound: &[SimpleSelector],
    node_id: NodeId,
    dom: &Dom,
    source: &[u8],
) -> bool {
    let node = dom.get(node_id);
    if !matches!(node.kind, NodeKind::Element { .. }) {
        return false;
    }

    let tag_name = node.tag_name(source).to_ascii_lowercase();

    for simple in compound {
        let matches = match simple {
            SimpleSelector::Tag(name) => tag_name == *name,
            SimpleSelector::Class(class_name) => node.has_class(class_name, source),
            SimpleSelector::Id(id_name) => node.get_id(source) == Some(id_name.as_str()),
            SimpleSelector::Universal => true,
            SimpleSelector::PseudoClass(pseudo) => match pseudo.as_str() {
                "first-child" => is_first_child(node_id, dom),
                "last-child" => is_last_child(node_id, dom),
                "root" => tag_name == "html" || node.parent == Some(NodeId(0)),
                "hover" => false,
                _ => false,
            },
            SimpleSelector::Attribute(attr, val) => node_has_attribute(node, attr, val, source),
        };

        if !matches {
            return false;
        }
    }

    true
}

// ─── Styled Tree Printer ─────────────────────────────────────────

impl StyledNode {
    /// Pretty-print the styled tree to stdout.
    pub fn print_tree(&self, dom: &Dom, source: &[u8]) {
        let output = self.format_tree(dom, source);
        print!("{}", output);
    }

    /// Format the styled tree as a string (useful for testing).
    pub fn format_tree(&self, dom: &Dom, source: &[u8]) -> String {
        let mut output = String::new();
        self.format_node(dom, source, 0, &mut output);
        output
    }

    fn format_node(&self, dom: &Dom, source: &[u8], depth: usize, output: &mut String) {
        let node = dom.get(self.node_id);
        let indent = "  ".repeat(depth);

        match &node.kind {
            NodeKind::Document => {
                output.push_str(&format!("{}Document\n", indent));
            }
            NodeKind::Element { tag_start, tag_end } => {
                let tag_name = std::str::from_utf8(&source[*tag_start as usize..*tag_end as usize])
                    .unwrap_or("???");

                if node.attributes.is_empty() {
                    output.push_str(&format!("{}Element <{}>\n", indent, tag_name));
                } else {
                    let mut attr_parts = Vec::new();
                    for &(ns, ne, vs, ve) in &node.attributes {
                        let name =
                            std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap_or("???");
                        if vs == 0 && ve == 0 {
                            attr_parts.push(name.to_string());
                        } else {
                            let value = std::str::from_utf8(&source[vs as usize..ve as usize])
                                .unwrap_or("???");
                            attr_parts.push(format!("{}=\"{}\"", name, value));
                        }
                    }
                    output.push_str(&format!(
                        "{}Element <{} {}>\n",
                        indent,
                        tag_name,
                        attr_parts.join(" ")
                    ));
                }

                // Print computed styles — only non-default values for readability
                let defaults = ComputedStyle::default();
                let style_entries = self.get_non_default_styles(&defaults);
                if !style_entries.is_empty() {
                    for (prop, value) in &style_entries {
                        output.push_str(&format!("{}  [{}:{}]\n", indent, prop, value));
                    }
                }
            }
            NodeKind::Text { start, end } => {
                let text =
                    std::str::from_utf8(&source[*start as usize..*end as usize]).unwrap_or("???");
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    output.push_str(&format!("{}Text \"{}\"\n", indent, trimmed));
                }
            }
            NodeKind::Comment { start, end } => {
                let comment =
                    std::str::from_utf8(&source[*start as usize..*end as usize]).unwrap_or("???");
                output.push_str(&format!("{}Comment \"{}\"\n", indent, comment.trim()));
            }
        }

        for child in &self.children {
            child.format_node(dom, source, depth + 1, output);
        }
    }

    /// Get a list of (property_name, value_string) for properties that
    /// differ from their default/initial values. Makes output cleaner.
    fn get_non_default_styles(&self, defaults: &ComputedStyle) -> Vec<(&'static str, String)> {
        let mut entries = Vec::new();
        let s = &self.styles;
        let d = defaults;

        if s.display != d.display {
            entries.push(("display", format!("{}", s.display)));
        }
        if s.color != d.color {
            entries.push(("color", format!("{}", s.color)));
        }
        if s.background_color != d.background_color {
            entries.push(("background-color", format!("{}", s.background_color)));
        }
        if s.font_size != d.font_size {
            entries.push(("font-size", format!("{}px", s.font_size)));
        }
        if s.font_weight != d.font_weight {
            entries.push(("font-weight", format!("{}", s.font_weight)));
        }
        if s.margin != d.margin {
            entries.push(("margin", format!("{}", s.margin)));
        }
        if s.padding != d.padding {
            entries.push(("padding", format!("{}", s.padding)));
        }
        if s.width != d.width {
            entries.push((
                "width",
                match s.width {
                    values::LengthOrPercentage::Px(v) => format!("{}px", v),
                    values::LengthOrPercentage::Percentage(p) => format!("{}%", p),
                    values::LengthOrPercentage::Calc { px, percentage } => format!("calc({}% + {}px)", percentage, px),
                    values::LengthOrPercentage::Auto => "auto".to_string(),
                },
            ));
        }
        if s.height != d.height {
            entries.push((
                "height",
                match s.height {
                    values::LengthOrPercentage::Px(v) => format!("{}px", v),
                    values::LengthOrPercentage::Percentage(p) => format!("{}%", p),
                    values::LengthOrPercentage::Calc { px, percentage } => format!("calc({}% + {}px)", percentage, px),
                    values::LengthOrPercentage::Auto => "auto".to_string(),
                },
            ));
        }
        if s.text_align != d.text_align {
            entries.push(("text-align", format!("{}", s.text_align)));
        }
        if s.line_height != d.line_height {
            entries.push(("line-height", format!("{}px", s.line_height)));
        }
        if s.position != d.position {
            entries.push(("position", format!("{:?}", s.position).to_ascii_lowercase()));
        }

        entries
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css_parser::Stylesheet;
    use crate::values::{Color, Display, TextAlign};

    /// Helper: parse HTML and CSS, resolve styles, return the styled tree
    fn styled_tree(html: &str, css: &str) -> (StyledNode, Dom, Vec<u8>) {
        let html_bytes = html.as_bytes().to_vec();
        let mut processor = crate::streaming_parser::StreamingHtmlProcessor::new();
        let _ = processor.receive_network_chunk(&html_bytes, true);
        let dom = processor.finish();

        let stylesheet = Stylesheet::parse(css.as_bytes());
        let styled = resolve_styles(&dom, &stylesheet, &html_bytes);

        (styled, dom, html_bytes)
    }

    // ── Specificity Tests ────────────────────────────────────────

    #[test]
    fn test_specificity_calculation() {
        use crate::css_parser::SimpleSelector;

        let sel = Selector {
            parts: vec![vec![SimpleSelector::Tag("div".into())]],
            steps: Vec::new(),
        };
        assert_eq!(compute_specificity(&sel), (0, 0, 1));

        let sel = Selector {
            parts: vec![vec![SimpleSelector::Class("main".into())]],
            steps: Vec::new(),
        };
        assert_eq!(compute_specificity(&sel), (0, 1, 0));

        let sel = Selector {
            parts: vec![vec![SimpleSelector::Id("header".into())]],
            steps: Vec::new(),
        };
        assert_eq!(compute_specificity(&sel), (1, 0, 0));

        let sel = Selector {
            parts: vec![vec![
                SimpleSelector::Tag("div".into()),
                SimpleSelector::Class("main".into()),
                SimpleSelector::Id("hero".into()),
            ]],
            steps: Vec::new(),
        };
        assert_eq!(compute_specificity(&sel), (1, 1, 1));

        let sel = Selector {
            parts: vec![
                vec![SimpleSelector::Tag("div".into())],
                vec![SimpleSelector::Tag("p".into())],
            ],
            steps: Vec::new(),
        };
        assert_eq!(compute_specificity(&sel), (0, 0, 2));
    }

    // ── Basic Selector Matching (typed) ──────────────────────────

    #[test]
    fn test_tag_selector_match() {
        let (styled, _, _) = styled_tree("<h1>Hello</h1>", "h1 { color: red; }");
        let h1 = &styled.children[0];
        assert_eq!(h1.styles.color, Color::rgb(255, 0, 0));
    }

    #[test]
    fn test_class_selector_match() {
        let (styled, _, _) = styled_tree(
            r#"<div class="main">Content</div>"#,
            ".main { background-color: white; }",
        );
        let div = &styled.children[0];
        assert_eq!(div.styles.background_color, Color::rgb(255, 255, 255));
    }

    #[test]
    fn test_id_selector_match() {
        let (styled, _, _) = styled_tree(
            r#"<div id="container">Content</div>"#,
            "#container { width: 960px; }",
        );
        let div = &styled.children[0];
        assert_eq!(div.styles.width, values::LengthOrPercentage::Px(960.0));
    }

    #[test]
    fn test_universal_selector() {
        let (styled, _, _) = styled_tree("<p>Text</p>", "* { margin: 5px; }");
        let p = &styled.children[0];
        assert_eq!(p.styles.margin, values::Margin::uniform(5.0));
    }

    #[test]
    fn test_no_match() {
        let (styled, _, _) = styled_tree("<p>Text</p>", "h1 { color: red; }");
        let p = &styled.children[0];
        assert_eq!(p.styles.color, Color::BLACK);
    }

    // ── Specificity-Based Cascade ────────────────────────────────

    #[test]
    fn test_specificity_id_beats_class() {
        let (styled, _, _) = styled_tree(
            r#"<div id="header" class="section">Content</div>"#,
            ".section { color: red; } #header { color: blue; }",
        );
        let div = &styled.children[0];
        assert_eq!(div.styles.color, Color::rgb(0, 0, 255));
    }

    #[test]
    fn test_specificity_class_beats_tag() {
        let (styled, _, _) = styled_tree(
            r#"<div class="main">Content</div>"#,
            "div { color: red; } .main { color: blue; }",
        );
        let div = &styled.children[0];
        assert_eq!(div.styles.color, Color::rgb(0, 0, 255));
    }

    #[test]
    fn test_same_specificity_last_wins() {
        let (styled, _, _) =
            styled_tree("<h1>Hello</h1>", "h1 { color: red; } h1 { color: blue; }");
        let h1 = &styled.children[0];
        assert_eq!(h1.styles.color, Color::rgb(0, 0, 255));
    }

    // ── Inheritance ──────────────────────────────────────────────

    #[test]
    fn test_color_inherits() {
        let (styled, _, _) = styled_tree("<div><p>Hello</p></div>", "div { color: green; }");
        let div = &styled.children[0];
        let p = &div.children[0];
        assert_eq!(div.styles.color, Color::rgb(0, 128, 0));
        assert_eq!(p.styles.color, Color::rgb(0, 128, 0));
    }

    #[test]
    fn test_font_size_inherits() {
        let (styled, _, _) = styled_tree("<div><p>Hello</p></div>", "div { font-size: 24px; }");
        let div = &styled.children[0];
        let p = &div.children[0];
        assert_eq!(div.styles.font_size, 24.0);
        assert_eq!(p.styles.font_size, 24.0);
    }

    #[test]
    fn test_margin_does_not_inherit() {
        let (styled, _, _) = styled_tree("<div><p>Hello</p></div>", "div { margin: 20px; }");
        let div = &styled.children[0];
        let p = &div.children[0];
        assert_eq!(div.styles.margin, values::Margin::uniform(20.0));
        assert_eq!(p.styles.margin, values::Margin::ZERO);
    }

    #[test]
    fn test_background_does_not_inherit() {
        let (styled, _, _) = styled_tree(
            "<div><p>Hello</p></div>",
            "div { background-color: yellow; }",
        );
        let div = &styled.children[0];
        let p = &div.children[0];
        assert_eq!(div.styles.background_color, Color::rgb(255, 255, 0));
        assert_eq!(p.styles.background_color, Color::TRANSPARENT);
    }

    // ── Value Computation ────────────────────────────────────────

    #[test]
    fn test_em_to_px() {
        let (styled, _, _) = styled_tree(
            "<div><p>Hello</p></div>",
            "div { font-size: 20px; } p { font-size: 2em; }",
        );
        let div = &styled.children[0];
        let p = &div.children[0];
        assert_eq!(div.styles.font_size, 20.0);
        assert_eq!(p.styles.font_size, 40.0);
    }

    #[test]
    fn test_hex_color_parsing() {
        let (styled, _, _) = styled_tree(
            "<h1>Hello</h1>",
            "h1 { color: #ff0000; background-color: #0f0; }",
        );
        let h1 = &styled.children[0];
        assert_eq!(h1.styles.color, Color::rgb(255, 0, 0));
        assert_eq!(h1.styles.background_color, Color::rgb(0, 255, 0));
    }

    // ── Inline Styles ────────────────────────────────────────────

    #[test]
    fn test_inline_style() {
        let (styled, _, _) = styled_tree(r#"<p style="color: red; font-size: 20px">Text</p>"#, "");
        let p = &styled.children[0];
        assert_eq!(p.styles.color, Color::rgb(255, 0, 0));
        assert_eq!(p.styles.font_size, 20.0);
    }

    #[test]
    fn test_inline_style_beats_author() {
        let (styled, _, _) = styled_tree(
            r#"<p style="color: green">Text</p>"#,
            "p { color: red; font-size: 14px; }",
        );
        let p = &styled.children[0];
        assert_eq!(p.styles.color, Color::rgb(0, 128, 0));
        assert_eq!(p.styles.font_size, 14.0);
    }

    #[test]
    fn test_inline_important_beats_author_important() {
        let (styled, _, _) = styled_tree(
            r#"<p style="color: green !important">Text</p>"#,
            "p { color: red !important; }",
        );
        let p = &styled.children[0];
        assert_eq!(p.styles.color, Color::rgb(0, 128, 0));
    }

    #[test]
    fn test_author_important_beats_inline_normal() {
        let (styled, _, _) = styled_tree(
            r#"<p style="color: blue">Text</p>"#,
            "p { color: red !important; }",
        );
        let p = &styled.children[0];
        assert_eq!(p.styles.color, Color::rgb(255, 0, 0));
    }

    // ── Display Property ─────────────────────────────────────────

    #[test]
    fn test_display_none() {
        let (styled, _, _) = styled_tree("<div>Content</div>", "div { display: none; }");
        let div = &styled.children[0];
        assert_eq!(div.styles.display, Display::None);
    }

    #[test]
    fn test_display_block() {
        let (styled, _, _) = styled_tree("<span>Content</span>", "span { display: block; }");
        let span = &styled.children[0];
        assert_eq!(span.styles.display, Display::Block);
    }

    // ── Descendant Selector ──────────────────────────────────────

    #[test]
    fn test_descendant_selector() {
        let (styled, _, _) = styled_tree(
            "<div><p>Hello</p></div><p>World</p>",
            "div p { color: blue; }",
        );
        let div = &styled.children[0];
        let p_inside = &div.children[0];
        assert_eq!(p_inside.styles.color, Color::rgb(0, 0, 255));

        let p_outside = &styled.children[1];
        assert_eq!(p_outside.styles.color, Color::BLACK);
    }

    // ── Compound Selector ────────────────────────────────────────

    #[test]
    fn test_compound_selector() {
        let (styled, _, _) = styled_tree(
            r#"<div class="main">A</div><div>B</div>"#,
            "div.main { color: red; }",
        );
        let div1 = &styled.children[0];
        assert_eq!(div1.styles.color, Color::rgb(255, 0, 0));

        let div2 = &styled.children[1];
        assert_eq!(div2.styles.color, Color::BLACK);
    }

    // ── Shorthand Expansion ──────────────────────────────────────

    #[test]
    fn test_margin_shorthand() {
        let (styled, _, _) = styled_tree("<div>Content</div>", "div { margin: 10px 20px; }");
        let div = &styled.children[0];
        assert_eq!(div.styles.margin.top, Some(10.0));
        assert_eq!(div.styles.margin.right, Some(20.0));
        assert_eq!(div.styles.margin.bottom, Some(10.0));
        assert_eq!(div.styles.margin.left, Some(20.0));
    }

    #[test]
    fn test_shorthand_overrides_earlier_longhand_by_cascade() {
        // Earlier rule sets margin-top: 5px, later rule sets margin: 20px
        let (styled, _, _) = styled_tree(
            "<div>Content</div>",
            "div { margin-top: 5px; margin: 20px; }",
        );
        let div = &styled.children[0];
        assert_eq!(div.styles.margin.top, Some(20.0));
        assert_eq!(div.styles.margin.left, Some(20.0));
    }

    #[test]
    fn test_longhand_overrides_earlier_shorthand_by_cascade() {
        // Earlier rule sets margin: 20px, later rule sets margin-top: 5px
        let (styled, _, _) = styled_tree(
            "<div>Content</div>",
            "div { margin: 20px; margin-top: 5px; }",
        );
        let div = &styled.children[0];
        assert_eq!(div.styles.margin.top, Some(5.0));
        assert_eq!(div.styles.margin.left, Some(20.0));
    }

    #[test]
    fn test_border_shorthand_expands_to_longhands() {
        let (styled, _, _) = styled_tree("<div>Content</div>", "div { border: 2px dashed red; }");
        let div = &styled.children[0];
        assert_eq!(div.styles.border_width.top, 2.0);
        assert_eq!(div.styles.border_width.left, 2.0);
        assert_eq!(div.styles.border_style, values::BorderStyleValue::Dashed);
        assert_eq!(div.styles.border_color, Color::rgb(255, 0, 0));
    }

    // ── Multiple Properties ──────────────────────────────────────

    #[test]
    fn test_multiple_properties() {
        let (styled, _, _) = styled_tree(
            "<p>Text</p>",
            "p { color: green; font-size: 14px; margin: 5px; }",
        );
        let p = &styled.children[0];
        assert_eq!(p.styles.color, Color::rgb(0, 128, 0));
        assert_eq!(p.styles.font_size, 14.0);
        assert_eq!(p.styles.margin, values::Margin::uniform(5.0));
    }

    // ── Multiple Classes ─────────────────────────────────────────

    #[test]
    fn test_multiple_classes() {
        let (styled, _, _) = styled_tree(
            r#"<div class="one two three">Content</div>"#,
            ".two { color: blue; }",
        );
        let div = &styled.children[0];
        assert_eq!(div.styles.color, Color::rgb(0, 0, 255));
    }

    // ── Styled Tree Formatting ───────────────────────────────────

    #[test]
    fn test_styled_tree_format() {
        let (styled, dom, source) = styled_tree("<h1>Hello</h1>", "h1 { color: red; }");
        let output = styled.format_tree(&dom, &source);
        assert!(output.contains("Element <h1>"));
        assert!(output.contains("[color:rgb(255,0,0)]"));
        assert!(output.contains("Text \"Hello\""));
    }

    // ── Font Weight ──────────────────────────────────────────────

    #[test]
    fn test_font_weight_bold() {
        let (styled, _, _) = styled_tree("<strong>Bold</strong>", "strong { font-weight: bold; }");
        let strong = &styled.children[0];
        assert_eq!(strong.styles.font_weight, 700.0);
    }

    // ── Text Align ───────────────────────────────────────────────

    #[test]
    fn test_text_align_center() {
        let (styled, _, _) = styled_tree("<div>Content</div>", "div { text-align: center; }");
        let div = &styled.children[0];
        assert_eq!(div.styles.text_align, TextAlign::Center);
    }

    #[test]
    fn test_text_align_inherits() {
        let (styled, _, _) = styled_tree("<div><p>Hello</p></div>", "div { text-align: center; }");
        let div = &styled.children[0];
        let p = &div.children[0];
        assert_eq!(div.styles.text_align, TextAlign::Center);
        assert_eq!(p.styles.text_align, TextAlign::Center);
    }

    #[test]
    fn test_child_combinator_matching() {
        let (styled, _, _) = styled_tree(
            "<div><p>Direct</p><span><p>Nested</p></span></div>",
            "div > p { color: red; }",
        );
        let div = &styled.children[0];
        let p_direct = &div.children[0];
        assert_eq!(p_direct.styles.color, Color::rgb(255, 0, 0));

        let span = &div.children[1];
        let p_nested = &span.children[0];
        assert_eq!(p_nested.styles.color, Color::BLACK);
    }

    #[test]
    fn test_sibling_combinator_matching() {
        let (styled, _, _) = styled_tree(
            "<div><h1>Title</h1><p>Next</p><p>Subsequent</p></div>",
            "h1 + p { color: green; } h1 ~ p { font-weight: bold; }",
        );
        let div = &styled.children[0];
        let p1 = &div.children[1];
        assert_eq!(p1.styles.color, Color::rgb(0, 128, 0));
        assert_eq!(p1.styles.font_weight, 700.0);

        let p2 = &div.children[2];
        assert_eq!(p2.styles.color, Color::BLACK);
        assert_eq!(p2.styles.font_weight, 700.0);
    }

    #[test]
    fn test_first_and_last_child_pseudo_classes() {
        let (styled, _, _) = styled_tree(
            "<div><p>First</p><p>Middle</p><p>Last</p></div>",
            "p:first-child { color: red; } p:last-child { color: blue; }",
        );
        let div = &styled.children[0];
        let first = &div.children[0];
        assert_eq!(first.styles.color, Color::rgb(255, 0, 0));

        let last = &div.children[2];
        assert_eq!(last.styles.color, Color::rgb(0, 0, 255));
    }

    #[test]
    fn test_media_query_viewport_matching() {
        let source = b"<div>Content</div>";
        let mut processor = crate::streaming_parser::StreamingHtmlProcessor::new();
        let _ = processor.receive_network_chunk(source, true);
        let dom = processor.finish();
        let stylesheet = crate::css_parser::Stylesheet::parse(
            b"@media (min-width: 600px) { div { color: red; } }",
        );

        let narrow = resolve_styles_with_viewport(&dom, &stylesheet, source, 500.0);
        assert_eq!(narrow.children[0].styles.color, Color::BLACK);

        let wide = resolve_styles_with_viewport(&dom, &stylesheet, source, 800.0);
        assert_eq!(wide.children[0].styles.color, Color::rgb(255, 0, 0));
    }

    #[test]
    fn test_important_overrides_higher_specificity() {
        // #id { color: blue } beats div { color: red !important }
        // !important on a lower-specificity rule should beat the higher-specificity rule
        let (styled, _, _) = styled_tree(
            r#"<div id="box">Text</div>"#,
            "div { color: red !important; } #box { color: blue; }",
        );
        let div = &styled.children[0];
        // !important red should win over higher-specificity blue
        assert_eq!(div.styles.color, crate::values::Color::rgb(255, 0, 0));
    }

    #[test]
    fn test_shorthand_inherit_expands_to_longhands() {
        // When a shorthand is set to `inherit`, all longhands should also be inherit
        // Here parent has margin 20px; child `margin: inherit` should pick up 20px
        let source = b"<div><p></p></div>";
        let mut processor = crate::streaming_parser::StreamingHtmlProcessor::new();
        let _ = processor.receive_network_chunk(source, true);
        let dom = processor.finish();
        let stylesheet = crate::css_parser::Stylesheet::parse(
            b"div { margin-top: 20px; } p { margin: inherit; }",
        );
        let styled = resolve_styles(&dom, &stylesheet, source);
        let div = &styled.children[0];
        let p = &div.children[0];
        // p inherits margin-top from div
        assert_eq!(p.styles.margin.top, div.styles.margin.top);
    }

    #[test]
    fn test_currentcolor_resolves_to_element_color() {
        // border-color: currentColor should resolve to the element's color property
        let (styled, _, _) = styled_tree(
            "<div>Text</div>",
            "div { color: rgb(100, 150, 200); border-color: currentColor; }",
        );
        let div = &styled.children[0];
        // border_color should equal computed color
        assert_eq!(div.styles.border_color, div.styles.color);
        assert_eq!(div.styles.color, crate::values::Color::rgb(100, 150, 200));

        // Explicit rgba(1, 1, 1, 0) is not replaced with currentColor
        let (styled2, _, _) = styled_tree(
            "<div>Text</div>",
            "div { color: red; border-color: rgba(1, 1, 1, 0); }",
        );
        let div2 = &styled2.children[0];
        assert_eq!(div2.styles.border_color, Color::new(1, 1, 1, 0));
    }

    #[test]
    fn test_ua_stylesheet_semantic_elements_get_block_display() {
        // HTML5 semantic elements should default to display: block
        for tag in &["aside", "blockquote", "pre", "figure", "details", "summary"] {
            let html = format!("<{0}>content</{0}>", tag);
            let (styled, _, _) = styled_tree(&html, "");
            let el = &styled.children[0];
            assert_eq!(
                el.styles.display,
                crate::values::Display::Block,
                "{tag} should have display: block by default"
            );
        }
    }

    #[test]
    fn test_ua_stylesheet_replaced_elements_inline_block() {
        // img and input should default to display: inline-block
        for tag in &["img", "input", "button"] {
            let html = format!("<{}>", tag);
            let (styled, _, _) = styled_tree(&html, "");
            let el = &styled.children[0];
            assert_eq!(
                el.styles.display,
                crate::values::Display::InlineBlock,
                "{tag} should have display: inline-block by default"
            );
        }
    }

    #[test]
    fn test_strip_important_helper() {
        // Test via indirect effect: !important on a low-specificity rule beats high-specificity
        let (styled, _, _) = styled_tree(
            "<p>text</p>",
            "* { color: green !important; } p { color: red; }",
        );
        let p = &styled.children[0];
        // !important green beats non-important red from more specific `p` selector
        assert_eq!(p.styles.color, crate::values::Color::rgb(0, 128, 0));
    }
}
