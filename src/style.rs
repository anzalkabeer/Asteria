use std::collections::HashMap;

use crate::css_parser::{Selector, SimpleSelector, Stylesheet};
use crate::dom::{Dom, NodeId, NodeKind};

// ─── Style Resolution ────────────────────────────────────────────
//
// This module takes a DOM tree and a Stylesheet and produces a
// "styled tree" — a separate tree that mirrors the DOM structure
// but carries computed styles on each element node.
//
// The approach:
//   1. Walk the DOM tree recursively
//   2. For each Element node, test every rule's selectors against it
//   3. Collect matching declarations into a PropertyMap
//   4. Later rules override earlier ones (simplified cascade)
//   5. Build a StyledNode tree with the results
//
// This is intentionally simple for v1:
//   - No specificity calculation (last-match-wins)
//   - No inherited properties (each node has only directly matched styles)
//   - No !important support
//   - No shorthand expansion (margin → margin-top/right/bottom/left)

/// The computed style for a single DOM node.
/// Maps CSS property names to their values.
/// e.g. {"color": "red", "font-size": "16px"}
pub type PropertyMap = HashMap<String, String>;

/// A node in the styled tree. Mirrors the DOM structure but carries
/// computed styles attached to each element.
#[derive(Debug)]
pub struct StyledNode {
    /// Which DOM node this styled node corresponds to
    pub node_id: NodeId,
    /// Computed styles for this node (only populated for Element nodes)
    pub styles: PropertyMap,
    /// Styled children — same order as DOM children
    pub children: Vec<StyledNode>,
}

// ─── Style Resolution Entry Point ────────────────────────────────

/// Resolve styles for the entire DOM tree.
/// Returns a StyledNode tree rooted at the Document node.
///
/// `dom` — the parsed DOM tree
/// `stylesheet` — the parsed CSS stylesheet
/// `source` — the original HTML source buffer (needed to read tag names and attributes)
pub fn resolve_styles(dom: &Dom, stylesheet: &Stylesheet, source: &[u8]) -> StyledNode {
    build_styled_node(dom, dom.root(), stylesheet, source)
}

/// Recursively build a StyledNode for a DOM node and its descendants.
fn build_styled_node(
    dom: &Dom,
    node_id: NodeId,
    stylesheet: &Stylesheet,
    source: &[u8],
) -> StyledNode {
    let node = dom.get(node_id);

    // Compute styles for this node (only Element nodes get matched)
    let styles = match &node.kind {
        NodeKind::Element { .. } => {
            let mut map = PropertyMap::new();

            // Test every rule against this node
            for rule in &stylesheet.rules {
                let matches = rule
                    .selectors
                    .iter()
                    .any(|sel| selector_matches(sel, node_id, dom, source));

                if matches {
                    // Apply all declarations from this rule
                    for decl in &rule.declarations {
                        map.insert(decl.property.clone(), decl.value.clone());
                    }
                }
            }

            // Also check for inline style="" attribute
            for &(ns, ne, vs, ve) in &node.attributes {
                let attr_name =
                    std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap_or("");
                if attr_name.eq_ignore_ascii_case("style") && vs != 0 && ve != 0 {
                    // Parse inline style declarations
                    let style_text = &source[vs as usize..ve as usize];
                    parse_inline_style(style_text, &mut map);
                }
            }

            map
        }
        _ => PropertyMap::new(),
    };

    // Recurse into children
    let children = node
        .children
        .iter()
        .map(|&child_id| build_styled_node(dom, child_id, stylesheet, source))
        .collect();

    StyledNode {
        node_id,
        styles,
        children,
    }
}

/// Parse inline style declarations from a style="" attribute value.
/// e.g. "color: red; font-size: 16px" → inserts into the property map.
fn parse_inline_style(style_bytes: &[u8], map: &mut PropertyMap) {
    let style_str = std::str::from_utf8(style_bytes).unwrap_or("");

    for declaration in style_str.split(';') {
        let declaration = declaration.trim();
        if declaration.is_empty() {
            continue;
        }

        if let Some((prop, val)) = declaration.split_once(':') {
            let property = prop.trim().to_ascii_lowercase();
            let value = val.trim().to_string();
            if !property.is_empty() && !value.is_empty() {
                map.insert(property, value);
            }
        }
    }
}

// ─── Selector Matching ───────────────────────────────────────────

/// Check if a selector matches a DOM node.
///
/// A selector has `parts` — a list of compound selector groups connected
/// by descendant combinators. The LAST part must match the target node,
/// and each earlier part must match some ancestor of the target.
///
/// e.g. for selector "div.main p":
///   parts = [[Tag("div"), Class("main")], [Tag("p")]]
///   - [Tag("p")] must match the target node
///   - [Tag("div"), Class("main")] must match some ancestor of the target
fn selector_matches(selector: &Selector, node_id: NodeId, dom: &Dom, source: &[u8]) -> bool {
    if selector.parts.is_empty() {
        return false;
    }

    // The last compound selector must match the target node
    let last = &selector.parts[selector.parts.len() - 1];
    if !compound_matches(last, node_id, dom, source) {
        return false;
    }

    // If there's only one part, we're done
    if selector.parts.len() == 1 {
        return true;
    }

    // For descendant combinator: each preceding compound selector must match
    // some ancestor, walking up the tree from the target's parent.
    // We match right-to-left through the parts.
    let mut current = dom.get(node_id).parent;
    let mut part_idx = selector.parts.len() - 2; // start from second-to-last

    loop {
        match current {
            None => return false, // ran out of ancestors
            Some(ancestor_id) => {
                if compound_matches(&selector.parts[part_idx], ancestor_id, dom, source) {
                    if part_idx == 0 {
                        return true; // all parts matched
                    }
                    part_idx -= 1;
                }
                current = dom.get(ancestor_id).parent;
            }
        }
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

    // Only Element nodes can match selectors
    let (tag_start, tag_end) = match &node.kind {
        NodeKind::Element {
            tag_start,
            tag_end,
        } => (*tag_start, *tag_end),
        _ => return false,
    };

    let tag_name = std::str::from_utf8(&source[tag_start as usize..tag_end as usize])
        .unwrap_or("")
        .to_ascii_lowercase();

    for simple in compound {
        let matches = match simple {
            SimpleSelector::Tag(name) => tag_name == *name,

            SimpleSelector::Class(class_name) => {
                // Check if the node has a class attribute containing this class
                node_has_class(node, class_name, source)
            }

            SimpleSelector::Id(id_name) => {
                // Check if the node has an id attribute matching this id
                node_has_id(node, id_name, source)
            }

            SimpleSelector::Universal => true,
        };

        if !matches {
            return false;
        }
    }

    true
}

/// Check if a node has a specific class in its class attribute.
/// The class attribute can contain multiple space-separated class names.
fn node_has_class(node: &crate::dom::Node, class_name: &str, source: &[u8]) -> bool {
    for &(ns, ne, vs, ve) in &node.attributes {
        let attr_name =
            std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap_or("");

        if attr_name.eq_ignore_ascii_case("class") && vs != 0 && ve != 0 {
            let attr_value =
                std::str::from_utf8(&source[vs as usize..ve as usize]).unwrap_or("");

            // Split by whitespace and check each class
            return attr_value.split_whitespace().any(|c| c == class_name);
        }
    }
    false
}

/// Check if a node has a specific id attribute value.
fn node_has_id(node: &crate::dom::Node, id_name: &str, source: &[u8]) -> bool {
    for &(ns, ne, vs, ve) in &node.attributes {
        let attr_name =
            std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap_or("");

        if attr_name.eq_ignore_ascii_case("id") && vs != 0 && ve != 0 {
            let attr_value =
                std::str::from_utf8(&source[vs as usize..ve as usize]).unwrap_or("");
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
            NodeKind::Element {
                tag_start,
                tag_end,
            } => {
                let tag_name =
                    std::str::from_utf8(&source[*tag_start as usize..*tag_end as usize])
                        .unwrap_or("???");

                // Build attribute string for display
                if node.attributes.is_empty() {
                    output.push_str(&format!("{}Element <{}>\n", indent, tag_name));
                } else {
                    let mut attr_parts = Vec::new();
                    for &(ns, ne, vs, ve) in &node.attributes {
                        let name =
                            std::str::from_utf8(&source[ns as usize..ne as usize])
                                .unwrap_or("???");
                        if vs == 0 && ve == 0 {
                            attr_parts.push(name.to_string());
                        } else {
                            let value =
                                std::str::from_utf8(&source[vs as usize..ve as usize])
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

                // Print computed styles (if any)
                if !self.styles.is_empty() {
                    // Sort properties for deterministic output
                    let mut props: Vec<_> = self.styles.iter().collect();
                    props.sort_by_key(|(k, _)| k.to_string());
                    for (prop, value) in props {
                        output.push_str(&format!("{}  [{}:{}]\n", indent, prop, value));
                    }
                }
            }
            NodeKind::Text { start, end } => {
                let text = std::str::from_utf8(&source[*start as usize..*end as usize])
                    .unwrap_or("???");
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    output.push_str(&format!("{}Text \"{}\"\n", indent, trimmed));
                }
            }
            NodeKind::Comment { start, end } => {
                let comment = std::str::from_utf8(&source[*start as usize..*end as usize])
                    .unwrap_or("???");
                output.push_str(&format!("{}Comment \"{}\"\n", indent, comment.trim()));
            }
        }

        // Recurse into styled children
        for child in &self.children {
            child.format_node(dom, source, depth + 1, output);
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css_parser::Stylesheet;
    use crate::parser::Parser;
    use crate::tokenizer::Tokenizer;

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

    #[test]
    fn test_tag_selector_match() {
        let (styled, _, _) = styled_tree("<h1>Hello</h1>", "h1 { color: red; }");

        // styled root = Document, first child = h1
        let h1 = &styled.children[0];
        assert_eq!(h1.styles.get("color"), Some(&"red".to_string()));
    }

    #[test]
    fn test_class_selector_match() {
        let (styled, _, _) = styled_tree(
            r#"<div class="main">Content</div>"#,
            ".main { background: white; }",
        );

        let div = &styled.children[0];
        assert_eq!(div.styles.get("background"), Some(&"white".to_string()));
    }

    #[test]
    fn test_id_selector_match() {
        let (styled, _, _) = styled_tree(
            r#"<div id="container">Content</div>"#,
            "#container { width: 960px; }",
        );

        let div = &styled.children[0];
        assert_eq!(div.styles.get("width"), Some(&"960px".to_string()));
    }

    #[test]
    fn test_universal_selector() {
        let (styled, _, _) = styled_tree("<p>Text</p>", "* { margin: 0; }");

        let p = &styled.children[0];
        assert_eq!(p.styles.get("margin"), Some(&"0".to_string()));
    }

    #[test]
    fn test_no_match() {
        let (styled, _, _) = styled_tree("<p>Text</p>", "h1 { color: red; }");

        // p should have no styles — h1 selector doesn't match
        let p = &styled.children[0];
        assert!(p.styles.is_empty());
    }

    #[test]
    fn test_compound_selector() {
        let (styled, _, _) = styled_tree(
            r#"<div class="main">A</div><div>B</div>"#,
            "div.main { color: red; }",
        );

        // First div has class="main" → should match
        let div1 = &styled.children[0];
        assert_eq!(div1.styles.get("color"), Some(&"red".to_string()));

        // Second div has no class → should NOT match
        let div2 = &styled.children[1];
        assert!(div2.styles.is_empty());
    }

    #[test]
    fn test_descendant_selector() {
        let (styled, _, _) = styled_tree(
            "<div><p>Hello</p></div><p>World</p>",
            "div p { color: blue; }",
        );

        // p inside div should match
        let div = &styled.children[0];
        let p_inside = &div.children[0];
        assert_eq!(p_inside.styles.get("color"), Some(&"blue".to_string()));

        // p outside div should NOT match
        let p_outside = &styled.children[1];
        assert!(p_outside.styles.is_empty());
    }

    #[test]
    fn test_cascade_last_wins() {
        let (styled, _, _) = styled_tree(
            "<h1>Hello</h1>",
            "h1 { color: red; } h1 { color: blue; }",
        );

        // Later rule should win
        let h1 = &styled.children[0];
        assert_eq!(h1.styles.get("color"), Some(&"blue".to_string()));
    }

    #[test]
    fn test_multiple_properties() {
        let (styled, _, _) = styled_tree(
            "<p>Text</p>",
            "p { color: green; font-size: 14px; margin: 5px; }",
        );

        let p = &styled.children[0];
        assert_eq!(p.styles.get("color"), Some(&"green".to_string()));
        assert_eq!(p.styles.get("font-size"), Some(&"14px".to_string()));
        assert_eq!(p.styles.get("margin"), Some(&"5px".to_string()));
    }

    #[test]
    fn test_inline_style() {
        let (styled, _, _) = styled_tree(
            r#"<p style="color: red; font-size: 20px">Text</p>"#,
            "",
        );

        let p = &styled.children[0];
        assert_eq!(p.styles.get("color"), Some(&"red".to_string()));
        assert_eq!(p.styles.get("font-size"), Some(&"20px".to_string()));
    }

    #[test]
    fn test_inline_style_overrides_stylesheet() {
        let (styled, _, _) = styled_tree(
            r#"<p style="color: green">Text</p>"#,
            "p { color: red; font-size: 14px; }",
        );

        let p = &styled.children[0];
        // inline style should override stylesheet for color
        assert_eq!(p.styles.get("color"), Some(&"green".to_string()));
        // font-size should still come from stylesheet
        assert_eq!(p.styles.get("font-size"), Some(&"14px".to_string()));
    }

    #[test]
    fn test_styled_tree_format() {
        let (styled, dom, source) = styled_tree("<h1>Hello</h1>", "h1 { color: red; }");

        let output = styled.format_tree(&dom, &source);
        assert!(output.contains("Element <h1>"));
        assert!(output.contains("[color:red]"));
        assert!(output.contains("Text \"Hello\""));
    }

    #[test]
    fn test_multiple_classes() {
        let (styled, _, _) = styled_tree(
            r#"<div class="one two three">Content</div>"#,
            ".two { color: blue; }",
        );

        let div = &styled.children[0];
        assert_eq!(div.styles.get("color"), Some(&"blue".to_string()));
    }
}
