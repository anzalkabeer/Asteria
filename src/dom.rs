use crate::tokens::Attribute;

// ─── Node Identity ───────────────────────────────────────────────
//
// Instead of using pointers (Box, Rc, etc.) to link nodes together,
// we use a simple index into a Vec. This is the "arena allocation"
// approach from the master plan.
//
// Think of it like this:
//   - All nodes live in a big Vec<Node> (the arena)
//   - NodeId(3) means "the node at index 3 in the arena"
//   - Parent/child relationships are just NodeId values pointing at each other
//
// Why? Better cache locality, lower RAM, no reference counting overhead.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

impl NodeId {
    /// Get the index into the arena Vec
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

// ─── Node Types ──────────────────────────────────────────────────
//
// Each node in the DOM tree is one of these kinds.
// Element and Text store offsets into the original HTML source buffer (zero-copy).

#[derive(Debug)]
pub enum NodeKind {
    /// The root of the document — there's exactly one of these
    Document,

    /// An HTML element like <div>, <p>, <h1>, etc.
    /// tag_start..tag_end is the slice of the tag name in the source buffer
    /// e.g. for "<div>" → tag_start=1, tag_end=4 → "div"
    Element { tag_start: u32, tag_end: u32 },

    /// Raw text content between tags
    /// e.g. for "<p>Hello</p>" → start points to 'H', end points past 'o'
    Text { start: u32, end: u32 },

    /// A comment like <!-- ... -->
    /// start..end is the comment content (without the <!-- and --> markers)
    Comment { start: u32, end: u32 },
}

// ─── DOM Node ────────────────────────────────────────────────────
//
// A single node in the DOM tree. Stores:
// - What kind of node it is (element, text, etc.)
// - Its parent (as a NodeId, or None for the Document root)
// - Its children (as a list of NodeIds)
// - Its attributes (offset pairs, only relevant for elements)

#[derive(Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    /// Attributes stored as offset pairs: (name_start, name_end, value_start, value_end)
    /// Only populated for Element nodes. Zero-copy — references the source buffer.
    pub attributes: Vec<(u32, u32, u32, u32)>,
}

// ─── The DOM Arena ───────────────────────────────────────────────
//
// This is the main data structure. All nodes are stored in a single Vec.
// The index of a node IS its NodeId.
//
// Example:
//   dom.nodes[0] → Document (root)
//   dom.nodes[1] → Element <html>
//   dom.nodes[2] → Element <head>
//   ... and so on
//
// To get a node: dom.get(NodeId(2)) → &Node (the <head> element)

#[derive(Debug)]
pub struct Dom {
    pub nodes: Vec<Node>,
}

impl Default for Dom {
    fn default() -> Self {
        Self::new()
    }
}

impl Dom {
    /// Create a new DOM with just a Document root node (NodeId(0))
    pub fn new() -> Self {
        let root = Node {
            kind: NodeKind::Document,
            parent: None,
            children: Vec::new(),
            attributes: Vec::new(),
        };
        Dom { nodes: vec![root] }
    }

    /// Get a reference to a node by its ID
    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id.index()]
    }

    /// Get a mutable reference to a node by its ID
    pub fn get_mut(&mut self, id: NodeId) -> &mut Node {
        &mut self.nodes[id.index()]
    }

    /// The Document root is always at index 0
    pub fn root(&self) -> NodeId {
        NodeId(0)
    }

    /// Add an Element node as a child of `parent`.
    /// `tag_start` and `tag_end` are offsets into the source buffer for the tag name.
    /// `attrs` are the attributes from the tokenizer, converted to offset tuples.
    /// Returns the NodeId of the newly created element.
    pub fn add_element(
        &mut self,
        parent: NodeId,
        tag_start: u32,
        tag_end: u32,
        attrs: &[Attribute],
    ) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);

        let attributes: Vec<(u32, u32, u32, u32)> = attrs
            .iter()
            .map(|a| (a.name_start, a.name_end, a.value_start, a.value_end))
            .collect();

        let node = Node {
            kind: NodeKind::Element { tag_start, tag_end },
            parent: Some(parent),
            children: Vec::new(),
            attributes,
        };

        self.nodes.push(node);

        // Register this node as a child of its parent
        self.nodes[parent.index()].children.push(id);

        id
    }

    /// Add a Text node as a child of `parent`.
    /// `start` and `end` are offsets into the source buffer.
    /// Returns the NodeId of the newly created text node.
    pub fn add_text(&mut self, parent: NodeId, start: u32, end: u32) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);

        let node = Node {
            kind: NodeKind::Text { start, end },
            parent: Some(parent),
            children: Vec::new(),
            attributes: Vec::new(),
        };

        self.nodes.push(node);
        self.nodes[parent.index()].children.push(id);

        id
    }

    /// Add a Comment node as a child of `parent`.
    /// `start` and `end` are offsets into the source buffer for the comment content.
    /// Returns the NodeId of the newly created comment node.
    pub fn add_comment(&mut self, parent: NodeId, start: u32, end: u32) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);

        let node = Node {
            kind: NodeKind::Comment { start, end },
            parent: Some(parent),
            children: Vec::new(),
            attributes: Vec::new(),
        };

        self.nodes.push(node);
        self.nodes[parent.index()].children.push(id);

        id
    }

    // ─── Tree Printer ────────────────────────────────────────────
    //
    // This is the "DOM printer" deliverable from Phase 1.
    // It walks the tree recursively and prints each node with indentation.
    // The `source` parameter is the original HTML byte buffer — we need it
    // to resolve the offset pairs back into readable strings.

    /// Pretty-print the entire DOM tree to stdout.
    /// `source` is the original HTML input so we can show tag names and text content.
    pub fn print_tree(&self, source: &[u8]) {
        let output = self.format_tree(source);
        print!("{}", output);
    }

    /// Format the DOM tree as a string (useful for testing).
    /// Same as print_tree but returns the string instead of printing it.
    pub fn format_tree(&self, source: &[u8]) -> String {
        let mut output = String::new();
        self.format_node(self.root(), source, 0, &mut output);
        output
    }

    /// Recursively format a single node and its children.
    /// `depth` controls indentation (2 spaces per level).
    fn format_node(&self, id: NodeId, source: &[u8], depth: usize, output: &mut String) {
        let node = self.get(id);
        let indent = "  ".repeat(depth);

        match &node.kind {
            NodeKind::Document => {
                output.push_str(&format!("{}Document\n", indent));
            }
            NodeKind::Element { tag_start, tag_end } => {
                let tag_name = std::str::from_utf8(&source[*tag_start as usize..*tag_end as usize])
                    .unwrap_or("???");

                // Build attribute string for display
                if node.attributes.is_empty() {
                    output.push_str(&format!("{}Element <{}>\n", indent, tag_name));
                } else {
                    let mut attr_parts = Vec::new();
                    for &(ns, ne, vs, ve) in &node.attributes {
                        let name =
                            std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap_or("???");
                        if vs == 0 && ve == 0 {
                            // Attribute with no value (e.g. "disabled")
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
            }
            NodeKind::Text { start, end } => {
                let text =
                    std::str::from_utf8(&source[*start as usize..*end as usize]).unwrap_or("???");
                // Trim whitespace for display, skip whitespace-only text nodes
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

        // Recurse into children
        for &child_id in &node.children {
            self.format_node(child_id, source, depth + 1, output);
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_dom() {
        let dom = Dom::new();
        assert_eq!(dom.nodes.len(), 1);
        assert!(matches!(dom.get(dom.root()).kind, NodeKind::Document));
    }

    #[test]
    fn test_add_element() {
        let source = b"<html>";
        //             01234 5
        // tag name "html" is at positions 1..5
        let mut dom = Dom::new();
        let root = dom.root();
        let html_id = dom.add_element(root, 1, 5, &[]);

        assert_eq!(dom.nodes.len(), 2);
        assert_eq!(dom.get(root).children.len(), 1);
        assert_eq!(dom.get(root).children[0], html_id);
        assert_eq!(dom.get(html_id).parent, Some(root));

        if let NodeKind::Element { tag_start, tag_end } = dom.get(html_id).kind {
            assert_eq!(
                std::str::from_utf8(&source[tag_start as usize..tag_end as usize]).unwrap(),
                "html"
            );
        } else {
            panic!("Expected Element node");
        }
    }

    #[test]
    fn test_add_text() {
        let source = b"Hello";
        let mut dom = Dom::new();
        let root = dom.root();
        let text_id = dom.add_text(root, 0, 5);

        assert_eq!(dom.get(root).children.len(), 1);
        if let NodeKind::Text { start, end } = dom.get(text_id).kind {
            assert_eq!(
                std::str::from_utf8(&source[start as usize..end as usize]).unwrap(),
                "Hello"
            );
        } else {
            panic!("Expected Text node");
        }
    }

    #[test]
    fn test_add_comment() {
        let source = b" a comment ";
        let mut dom = Dom::new();
        let root = dom.root();
        let comment_id = dom.add_comment(root, 0, 11);

        if let NodeKind::Comment { start, end } = dom.get(comment_id).kind {
            assert_eq!(
                std::str::from_utf8(&source[start as usize..end as usize]).unwrap(),
                " a comment "
            );
        } else {
            panic!("Expected Comment node");
        }
    }

    #[test]
    fn test_tree_structure() {
        // Manually build: Document → html → (head, body)
        let _source = b"htmlheadbody";
        //             0123456789...
        // "html" = 0..4, "head" = 4..8, "body" = 8..12

        let mut dom = Dom::new();
        let root = dom.root();
        let html = dom.add_element(root, 0, 4, &[]);
        let head = dom.add_element(html, 4, 8, &[]);
        let body = dom.add_element(html, 8, 12, &[]);

        // html has 2 children
        assert_eq!(dom.get(html).children.len(), 2);
        assert_eq!(dom.get(html).children[0], head);
        assert_eq!(dom.get(html).children[1], body);

        // head and body both have html as parent
        assert_eq!(dom.get(head).parent, Some(html));
        assert_eq!(dom.get(body).parent, Some(html));
    }

    #[test]
    fn test_print_tree() {
        // Build a small tree and verify the formatted output
        let source = b"<html><body><h1>Hello</h1></body></html>";
        //                  ^   ^    ^  ^
        // tag names: "html"=1..5, "body"=7..11, "h1"=13..15
        // text "Hello" = 16..21

        let mut dom = Dom::new();
        let root = dom.root();
        let html = dom.add_element(root, 1, 5, &[]);
        let body = dom.add_element(html, 7, 11, &[]);
        let h1 = dom.add_element(body, 13, 15, &[]);
        dom.add_text(h1, 16, 21);

        let output = dom.format_tree(source);
        assert!(output.contains("Document"));
        assert!(output.contains("Element <html>"));
        assert!(output.contains("Element <body>"));
        assert!(output.contains("Element <h1>"));
        assert!(output.contains("Text \"Hello\""));
    }

    #[test]
    fn test_print_tree_with_attributes() {
        let source = b"<div class=\"main\">";
        // tag name "div" = 1..4
        // attr name "class" = 5..10, attr value "main" = 12..16

        let attr = Attribute {
            name_start: 5,
            name_end: 10,
            value_start: 12,
            value_end: 16,
        };

        let mut dom = Dom::new();
        let root = dom.root();
        dom.add_element(root, 1, 4, &[attr]);

        let output = dom.format_tree(source);
        assert!(output.contains("Element <div class=\"main\">"));
    }
}
