use std::collections::HashMap;

use crate::css_parser::{Selector, SimpleSelector, Stylesheet};
use crate::dom::{Dom, NodeId, NodeKind};
use crate::properties::{self, ALL_PROPERTIES, PropertyId};
use crate::values::{self, ComputedStyle, Display};

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
//   - RuleSet indexing (O(elements*rules) is fine for small pages)
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
                SimpleSelector::Class(_) | SimpleSelector::PseudoClass(_) => classes += 1,
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
/// cascade metadata for sorting.
#[derive(Debug)]
struct MatchedDeclaration {
    property: String,
    value: String,
    specificity: Specificity,
    source_order: usize,
    origin: Origin,
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
        "html"
            | "body"
            | "div"
            | "p"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "header"
            | "footer"
            | "section"
            | "article"
            | "nav"
            | "main"
            | "ul"
            | "ol"
            | "li"
            | "form"
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
    let root_style = ComputedStyle::default();
    build_styled_node(
        dom,
        dom.root(),
        stylesheet,
        source,
        &root_style,
        ROOT_FONT_SIZE,
        viewport_width,
    )
}

/// Recursively build a StyledNode for a DOM node and its descendants.
///
/// `parent_style` — the parent's computed style (for inheritance)
/// `root_font_size` — the root element's computed font-size (for rem units)
#[allow(clippy::collapsible_if)]
fn build_styled_node(
    dom: &Dom,
    node_id: NodeId,
    stylesheet: &Stylesheet,
    source: &[u8],
    parent_style: &ComputedStyle,
    root_font_size: f32,
    viewport_width: f32,
) -> StyledNode {
    let node = dom.get(node_id);

    // Compute styles for this node (only Element nodes get matched)
    let styles = match &node.kind {
        NodeKind::Element { .. } => {
            // ── Step 1: Collect all matching declarations ──────────
            let mut declarations = Vec::new();

            // Collect top-level rules and applicable @media rules
            let mut all_rules: Vec<&crate::css_parser::StyleRule> =
                stylesheet.rules.iter().collect();

            for media in &stylesheet.media_rules {
                let matches_min = media.min_width.map_or(true, |mw| viewport_width >= mw);
                let matches_max = media.max_width.map_or(true, |mw| viewport_width <= mw);
                if matches_min && matches_max {
                    all_rules.extend(media.rules.iter());
                }
            }

            // Test every rule against this node
            for (rule_index, rule) in all_rules.iter().enumerate() {
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
                        declarations.push(MatchedDeclaration {
                            property: decl.property.clone(),
                            value: decl.value.clone(),
                            specificity,
                            source_order: rule_index,
                            origin: Origin::Author,
                        });
                    }
                }
            }

            // Check for inline style="" attribute (highest cascade priority)
            for &(ns, ne, vs, ve) in &node.attributes {
                let attr_name =
                    std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap_or("");
                if attr_name.eq_ignore_ascii_case("style") && vs != 0 && ve != 0 {
                    let style_text = &source[vs as usize..ve as usize];
                    let inline_decls = parse_inline_style(style_text);
                    for (prop, val) in inline_decls {
                        declarations.push(MatchedDeclaration {
                            property: prop,
                            value: val,
                            specificity: (0, 0, 0), // doesn't matter — origin wins
                            source_order: usize::MAX,
                            origin: Origin::Inline,
                        });
                    }
                }
            }

            // ── Step 2: Sort by cascade priority ──────────────────
            // (origin ASC, specificity ASC, source_order ASC)
            // → last entry per property wins
            declarations.sort_by(|a, b| {
                a.origin
                    .cmp(&b.origin)
                    .then(a.specificity.cmp(&b.specificity))
                    .then(a.source_order.cmp(&b.source_order))
            });

            // ── Step 3: Pick winners per property ─────────────────
            // Last declaration for each property wins (since sorted ascending)
            let mut specified: HashMap<String, String> = HashMap::new();
            for decl in &declarations {
                specified.insert(decl.property.clone(), decl.value.clone());
            }

            // ── Step 4: Expand shorthands ─────────────────────────
            let mut expanded: HashMap<String, String> = HashMap::new();
            for (prop, value) in &specified {
                if properties::is_shorthand(prop) {
                    if prop == "border" {
                        let (w, s, c) = values::parse_border_shorthand(value);
                        if let Some(w_val) = w {
                            for edge_name in &[
                                "border-top-width",
                                "border-right-width",
                                "border-bottom-width",
                                "border-left-width",
                            ] {
                                if !specified.contains_key(*edge_name) {
                                    expanded.insert(edge_name.to_string(), w_val.clone());
                                }
                            }
                        }
                        if let Some(s_val) = s {
                            if !specified.contains_key("border-style") {
                                expanded.insert("border-style".to_string(), s_val);
                            }
                        }
                        if let Some(c_val) = c {
                            if !specified.contains_key("border-color") {
                                expanded.insert("border-color".to_string(), c_val);
                            }
                        }
                    } else if let Some(longhand_ids) = properties::expand_shorthand(prop) {
                        let edges =
                            values::parse_edges(value, parent_style.font_size, root_font_size);
                        let edge_values = [edges.top, edges.right, edges.bottom, edges.left];
                        for (id, px_val) in longhand_ids.iter().zip(edge_values.iter()) {
                            let longhand_name = property_id_to_name(*id);
                            // Only set if not already explicitly set by a longhand
                            if !specified.contains_key(longhand_name) {
                                expanded.insert(longhand_name.to_string(), format!("{}px", px_val));
                            }
                        }
                    }
                }
            }
            // Merge expanded shorthands (longhands take priority)
            for (prop, value) in expanded {
                specified.entry(prop).or_insert(value);
            }

            // ── Step 5: Build ComputedStyle with inheritance ──────
            let mut computed = ComputedStyle::default();

            // First: resolve font-size (other em values depend on it)
            if let Some(fs_value) = specified.get("font-size") {
                if fs_value == "inherit" {
                    computed.font_size = parent_style.font_size;
                } else if fs_value == "initial" {
                    computed.font_size = 16.0;
                } else {
                    computed.font_size =
                        values::parse_length(fs_value, parent_style.font_size, root_font_size);
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

                let prop_name = property_id_to_name(prop_id);

                if let Some(value) = specified.get(prop_name) {
                    if value == "inherit" {
                        copy_property(&mut computed, parent_style, prop_id);
                    } else if value == "initial" {
                        // keep initial from Default impl
                    } else {
                        computed.set_property(
                            prop_id,
                            value,
                            parent_style.font_size,
                            root_font_size,
                        );
                    }
                } else {
                    // Property not specified — apply defaulting
                    if properties::is_inherited(prop_id) {
                        copy_property(&mut computed, parent_style, prop_id);
                    }
                    // Non-inherited: keep initial value from Default impl
                    // User-Agent default stylesheet: block tags default to Display::Block & base colors
                    if let NodeKind::Element { tag_start, tag_end } = &node.kind {
                        let tag_name =
                            std::str::from_utf8(&source[*tag_start as usize..*tag_end as usize])
                                .unwrap_or("")
                                .to_ascii_lowercase();

                        if prop_id == PropertyId::Display {
                            match tag_name.as_str() {
                                "head" | "title" | "meta" | "script" | "style" | "link" => {
                                    computed.display = Display::None;
                                }
                                "img" => {
                                    computed.display = Display::InlineBlock;
                                    if computed.width.is_none() {
                                        computed.width = Some(160.0);
                                    }
                                    if computed.height.is_none() {
                                        computed.height = Some(100.0);
                                    }
                                }
                                _ if is_default_block_tag(&tag_name) => {
                                    computed.display = Display::Block;
                                }
                                _ => {}
                            }
                        }

                        if !specified.contains_key("background-color") {
                            match tag_name.as_str() {
                                "body" => {
                                    computed.background_color = values::Color::rgb(248, 250, 252);
                                    if !specified.contains_key("margin-top") {
                                        computed.margin = values::Edges::uniform(8.0);
                                    }
                                }
                                "h1" => {
                                    computed.background_color = values::Color::rgb(240, 249, 255);
                                    computed.border_color = values::Color::rgb(2, 132, 199);
                                    computed.border_width.left = 4.0;
                                }
                                "div" => {
                                    computed.background_color = values::Color::rgb(248, 250, 252);
                                    computed.border_color = values::Color::rgb(203, 213, 225);
                                    computed.border_width = values::Edges::uniform(1.0);
                                }
                                "img" => {
                                    computed.background_color = values::Color::rgb(226, 232, 240);
                                    computed.border_color = values::Color::rgb(203, 213, 225);
                                    computed.border_width = values::Edges::uniform(1.0);
                                }
                                _ => {}
                            }
                        }

                        if !specified.contains_key("color") {
                            match tag_name.as_str() {
                                "h1" => {
                                    computed.color = values::Color::rgb(3, 105, 161);
                                }
                                _ => {}
                            }
                        }

                        if !specified.contains_key("padding-top") {
                            if tag_name == "h1" || tag_name == "div" {
                                computed.padding = values::Edges::uniform(12.0);
                            }
                        }
                    }
                }
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
                stylesheet,
                source,
                &styles,
                root_font_size,
                viewport_width,
            )
        })
        .collect();

    StyledNode {
        node_id,
        styles,
        children,
    }
}

/// Copy a single property value from parent to child.
fn copy_property(child: &mut ComputedStyle, parent: &ComputedStyle, prop: PropertyId) {
    match prop {
        PropertyId::Display => child.display = parent.display,
        PropertyId::Position => child.position = parent.position,
        PropertyId::Width => child.width = parent.width,
        PropertyId::Height => child.height = parent.height,
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
    }
}

/// Map a PropertyId back to its CSS property name string.
fn property_id_to_name(id: PropertyId) -> &'static str {
    match id {
        PropertyId::Display => "display",
        PropertyId::Position => "position",
        PropertyId::Width => "width",
        PropertyId::Height => "height",
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
    }
}

/// Parse inline style declarations from a style="" attribute value.
fn parse_inline_style(style_bytes: &[u8]) -> Vec<(String, String)> {
    let style_str = std::str::from_utf8(style_bytes).unwrap_or("");
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

    let last = &selector.parts[selector.parts.len() - 1];
    if !compound_matches(last, node_id, dom, source) {
        return false;
    }

    if selector.parts.len() == 1 {
        return true;
    }

    let mut current = dom.get(node_id).parent;
    let mut part_idx = selector.parts.len() - 2;

    loop {
        match current {
            None => return false,
            Some(ancestor_id) => {
                if compound_matches(&selector.parts[part_idx], ancestor_id, dom, source) {
                    if part_idx == 0 {
                        return true;
                    }
                    part_idx -= 1;
                }
                current = dom.get(ancestor_id).parent;
            }
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

    let last_step = &steps[steps.len() - 1];
    if !compound_matches(&last_step.compound, node_id, dom, source) {
        return false;
    }

    if steps.len() == 1 {
        return true;
    }

    let mut current_node = node_id;
    let mut step_idx = steps.len() - 1;

    while step_idx > 0 {
        let combinator = steps[step_idx].combinator;
        let target_step = &steps[step_idx - 1];

        match combinator {
            crate::css_parser::Combinator::Child => {
                let parent_id = match dom.get(current_node).parent {
                    Some(pid) => pid,
                    None => return false,
                };
                if !compound_matches(&target_step.compound, parent_id, dom, source) {
                    return false;
                }
                current_node = parent_id;
            }
            crate::css_parser::Combinator::Descendant => {
                let mut parent_opt = dom.get(current_node).parent;
                let mut matched = false;
                while let Some(parent_id) = parent_opt {
                    if compound_matches(&target_step.compound, parent_id, dom, source) {
                        current_node = parent_id;
                        matched = true;
                        break;
                    }
                    parent_opt = dom.get(parent_id).parent;
                }
                if !matched {
                    return false;
                }
            }
            crate::css_parser::Combinator::NextSibling => {
                let sibling_id = match get_previous_element_sibling(current_node, dom) {
                    Some(sid) => sid,
                    None => return false,
                };
                if !compound_matches(&target_step.compound, sibling_id, dom, source) {
                    return false;
                }
                current_node = sibling_id;
            }
            crate::css_parser::Combinator::SubsequentSibling => {
                let mut sibling_opt = get_previous_element_sibling(current_node, dom);
                let mut matched = false;
                while let Some(sibling_id) = sibling_opt {
                    if compound_matches(&target_step.compound, sibling_id, dom, source) {
                        current_node = sibling_id;
                        matched = true;
                        break;
                    }
                    sibling_opt = get_previous_element_sibling(sibling_id, dom);
                }
                if !matched {
                    return false;
                }
            }
        }

        step_idx -= 1;
    }

    true
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

/// Check if all simple selectors in a compound selector match a node.
/// ALL of them must match (it's an AND — e.g. div.main means both Tag and Class).
fn compound_matches(
    compound: &[SimpleSelector],
    node_id: NodeId,
    dom: &Dom,
    source: &[u8],
) -> bool {
    let node = dom.get(node_id);

    let (tag_start, tag_end) = match &node.kind {
        NodeKind::Element { tag_start, tag_end } => (*tag_start, *tag_end),
        _ => return false,
    };

    let tag_name = std::str::from_utf8(&source[tag_start as usize..tag_end as usize])
        .unwrap_or("")
        .to_ascii_lowercase();

    for simple in compound {
        let matches = match simple {
            SimpleSelector::Tag(name) => tag_name == *name,
            SimpleSelector::Class(class_name) => node_has_class(node, class_name, source),
            SimpleSelector::Id(id_name) => node_has_id(node, id_name, source),
            SimpleSelector::Universal => true,
            SimpleSelector::PseudoClass(pseudo) => match pseudo.as_str() {
                "first-child" => is_first_child(node_id, dom),
                "last-child" => is_last_child(node_id, dom),
                "hover" => false,
                _ => false,
            },
        };

        if !matches {
            return false;
        }
    }

    true
}

/// Check if a node has a specific class in its class attribute.
fn node_has_class(node: &crate::dom::Node, class_name: &str, source: &[u8]) -> bool {
    for &(ns, ne, vs, ve) in &node.attributes {
        let attr_name = std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap_or("");

        if attr_name.eq_ignore_ascii_case("class") && vs != 0 && ve != 0 {
            let attr_value = std::str::from_utf8(&source[vs as usize..ve as usize]).unwrap_or("");
            return attr_value.split_whitespace().any(|c| c == class_name);
        }
    }
    false
}

/// Check if a node has a specific id attribute value.
fn node_has_id(node: &crate::dom::Node, id_name: &str, source: &[u8]) -> bool {
    for &(ns, ne, vs, ve) in &node.attributes {
        let attr_name = std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap_or("");

        if attr_name.eq_ignore_ascii_case("id") && vs != 0 && ve != 0 {
            let attr_value = std::str::from_utf8(&source[vs as usize..ve as usize]).unwrap_or("");
            return attr_value == id_name;
        }
    }
    false
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
                s.width
                    .map(|v| format!("{}px", v))
                    .unwrap_or("auto".to_string()),
            ));
        }
        if s.height != d.height {
            entries.push((
                "height",
                s.height
                    .map(|v| format!("{}px", v))
                    .unwrap_or("auto".to_string()),
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
    use crate::parser::Parser;
    use crate::tokenizer::Tokenizer;
    use crate::values::{Color, Display, Edges, TextAlign};

    /// Helper: parse HTML and CSS, resolve styles, return the styled tree
    fn styled_tree(html: &str, css: &str) -> (StyledNode, Dom, Vec<u8>) {
        let html_bytes = html.as_bytes().to_vec();
        let mut tokenizer = Tokenizer::new(&html_bytes);
        let tokens = tokenizer.tokenize();
        let parser = Parser::new(&tokens, &html_bytes);
        let dom = parser.parse();

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
        assert_eq!(div.styles.width, Some(960.0));
    }

    #[test]
    fn test_universal_selector() {
        let (styled, _, _) = styled_tree("<p>Text</p>", "* { margin: 5px; }");
        let p = &styled.children[0];
        assert_eq!(p.styles.margin, Edges::uniform(5.0));
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
        assert_eq!(div.styles.margin, Edges::uniform(20.0));
        assert_eq!(p.styles.margin, Edges::ZERO);
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
        assert_eq!(div.styles.margin.top, 10.0);
        assert_eq!(div.styles.margin.right, 20.0);
        assert_eq!(div.styles.margin.bottom, 10.0);
        assert_eq!(div.styles.margin.left, 20.0);
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
        assert_eq!(p.styles.margin, Edges::uniform(5.0));
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
        let html_tokens = crate::tokenizer::Tokenizer::new(source).tokenize();
        let dom = crate::parser::Parser::new(&html_tokens, source).parse();
        let stylesheet = crate::css_parser::Stylesheet::parse(
            b"@media (min-width: 600px) { div { color: red; } }",
        );

        let narrow = resolve_styles_with_viewport(&dom, &stylesheet, source, 500.0);
        assert_eq!(narrow.children[0].styles.color, Color::BLACK);

        let wide = resolve_styles_with_viewport(&dom, &stylesheet, source, 800.0);
        assert_eq!(wide.children[0].styles.color, Color::rgb(255, 0, 0));
    }
}
