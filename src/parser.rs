use crate::dom::{Dom, NodeId};
use crate::tokens::{Token, TokenKind};

// ─── HTML Void Elements ──────────────────────────────────────────
//
// These are HTML elements that CANNOT have children and don't need a closing tag.
// e.g. <br>, <img>, <input>, <hr> are all valid without </br>, </img>, etc.
//
// When the parser sees a StartTag for one of these, it does NOT push it onto
// the open_elements stack — because there will never be a matching EndTag.
//
// Full list from the HTML spec:
// https://html.spec.whatwg.org/multipage/syntax.html#void-elements

const VOID_ELEMENTS: &[&[u8]] = &[
    b"area", b"base", b"br", b"col", b"embed", b"hr", b"img", b"input", b"link", b"meta", b"param",
    b"source", b"track", b"wbr",
];

/// Check if a tag name (from the source buffer) is a void element.
/// Comparison is case-insensitive since HTML tag names are case-insensitive.
fn is_void_element(source: &[u8], tag_start: u32, tag_end: u32) -> bool {
    let tag_name = &source[tag_start as usize..tag_end as usize];
    VOID_ELEMENTS.iter().any(|void| {
        void.len() == tag_name.len()
            && void
                .iter()
                .zip(tag_name.iter())
                .all(|(a, b)| a.eq_ignore_ascii_case(b))
    })
}

// ─── The Parser ──────────────────────────────────────────────────
//
// The parser consumes a list of tokens (from the tokenizer) and builds a DOM tree.
//
// How it works:
//   1. Start with a Document root node
//   2. Maintain a stack of "open elements" — the path from root to the current insertion point
//   3. For each token:
//      - StartTag → create an Element, append it to the current parent,
//                    push it onto the stack (unless it's a void element)
//      - EndTag   → pop elements from the stack until we find a matching tag
//      - Text     → create a Text node, append to current parent
//      - Comment  → create a Comment node, append to current parent
//      - SelfClosingTag → like StartTag but never pushed onto the stack
//      - Doctype  → skip (could store as metadata in the future)
//      - Eof      → done
//
// The "current parent" is always the top of the open_elements stack.
//
// Example walkthrough for "<div><p>Hi</p></div>":
//
//   Token           | Action                              | Stack (top = right)
//   ──────────────────────────────────────────────────────────────────────────
//   StartTag "div"  | create Element, push                | [Document, div]
//   StartTag "p"    | create Element, push                | [Document, div, p]
//   Text "Hi"       | create Text under p                 | [Document, div, p]
//   EndTag "p"      | pop p                               | [Document, div]
//   EndTag "div"    | pop div                              | [Document]
//   Eof             | done                                 | [Document]

pub struct Parser {
    /// Stack of open element NodeIds — the current "insertion path"
    /// The top of the stack is the current parent for new nodes.
    open_elements: Vec<NodeId>,
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        // Assume Document root (NodeId(0)) is always the starting point
        Parser {
            open_elements: vec![NodeId(0)],
        }
    }

    pub fn push_tokens(&mut self, dom: &mut Dom, tokens: &[Token], source: &[u8]) -> Vec<NodeId> {
        let mut dirty = Vec::new();

        for token in tokens {
            match token.kind {
                TokenKind::StartTag => self.handle_start_tag(dom, token, source, &mut dirty),
                TokenKind::EndTag => self.handle_end_tag(dom, token, source, &mut dirty),
                TokenKind::SelfClosingTag => {
                    self.handle_self_closing_tag(dom, token, source, &mut dirty)
                }
                TokenKind::Text => self.handle_text(dom, token, &mut dirty),
                TokenKind::Comment => self.handle_comment(dom, token, &mut dirty),
                TokenKind::Doctype => {}
                TokenKind::Eof => break,
            }
        }

        dirty
    }

    // ─── Token Handlers ──────────────────────────────────────────

    fn handle_start_tag(
        &mut self,
        dom: &mut Dom,
        token: &Token,
        source: &[u8],
        dirty: &mut Vec<NodeId>,
    ) {
        let parent = self.current_parent();

        let node_id = dom.add_element(parent, token.start, token.end, &token.attributes);

        if !is_void_element(source, token.start, token.end) {
            self.open_elements.push(node_id);
        } else {
            // Void elements complete immediately, mark them dirty
            dirty.push(node_id);
        }

        // Also mark parent as dirty since its children changed
        dirty.push(parent);
    }

    fn handle_end_tag(
        &mut self,
        dom: &mut Dom,
        token: &Token,
        source: &[u8],
        dirty: &mut Vec<NodeId>,
    ) {
        let end_tag_name = &source[token.start as usize..token.end as usize];

        let mut match_index = None;
        for i in (1..self.open_elements.len()).rev() {
            let node = dom.get(self.open_elements[i]);
            if let crate::dom::NodeKind::Element { tag_start, tag_end } = &node.kind {
                let open_tag_name = &source[*tag_start as usize..*tag_end as usize];
                if tag_names_match(open_tag_name, end_tag_name) {
                    match_index = Some(i);
                    break;
                }
            }
        }

        if let Some(idx) = match_index {
            for i in idx..self.open_elements.len() {
                dirty.push(self.open_elements[i]);
            }
            self.open_elements.truncate(idx);
        }
    }

    fn handle_self_closing_tag(
        &mut self,
        dom: &mut Dom,
        token: &Token,
        _source: &[u8],
        dirty: &mut Vec<NodeId>,
    ) {
        let parent = self.current_parent();
        let node_id = dom.add_element(parent, token.start, token.end, &token.attributes);
        dirty.push(node_id);
        dirty.push(parent);
    }

    fn handle_text(&mut self, dom: &mut Dom, token: &Token, dirty: &mut Vec<NodeId>) {
        let parent = self.current_parent();
        let node_id = dom.add_text(parent, token.start, token.end);
        dirty.push(node_id);
        dirty.push(parent);
    }

    fn handle_comment(&mut self, dom: &mut Dom, token: &Token, dirty: &mut Vec<NodeId>) {
        let parent = self.current_parent();
        let node_id = dom.add_comment(parent, token.start, token.end);
        dirty.push(node_id);
        dirty.push(parent);
    }

    // ─── Helpers ─────────────────────────────────────────────────

    /// The current parent is the top of the open_elements stack.
    /// New nodes get appended as children of this node.
    fn current_parent(&self) -> NodeId {
        *self
            .open_elements
            .last()
            .expect("open_elements stack should never be empty (Document root is always there)")
    }
}

/// Case-insensitive comparison of two tag name byte slices.
/// HTML tag names are case-insensitive: <DIV> and <div> are the same.
fn tag_names_match(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(x, y)| x.eq_ignore_ascii_case(y))
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::dom::NodeKind;
    use crate::tokenizer::Tokenizer;

    use super::*;

    /// Helper: tokenize + parse an HTML string, return the DOM
    fn parse_html(html: &str) -> (Dom, Vec<u8>) {
        let bytes = html.as_bytes().to_vec();
        let mut processor = crate::streaming_parser::StreamingHtmlProcessor::new();
        let _ = processor.receive_network_chunk(&bytes, true);
        (processor.finish(), bytes)
    }

    /// Helper: get the tag name of a node as a string
    fn tag_name<'a>(dom: &Dom, id: NodeId, source: &'a [u8]) -> &'a str {
        if let NodeKind::Element { tag_start, tag_end } = dom.get(id).kind {
            std::str::from_utf8(&source[tag_start as usize..tag_end as usize]).unwrap()
        } else {
            panic!("Expected Element node");
        }
    }

    /// Helper: get the text content of a node as a string
    fn text_content<'a>(dom: &Dom, id: NodeId, source: &'a [u8]) -> &'a str {
        if let NodeKind::Text { start, end } = dom.get(id).kind {
            std::str::from_utf8(&source[start as usize..end as usize]).unwrap()
        } else {
            panic!("Expected Text node");
        }
    }

    #[test]
    fn test_simple_document() {
        let (dom, source) = parse_html("<html><body></body></html>");

        // Document root should have 1 child (html)
        let root = dom.root();
        assert_eq!(dom.get(root).children.len(), 1);

        let html = dom.get(root).children[0];
        assert_eq!(tag_name(&dom, html, &source), "html");

        // html should have 1 child (body)
        assert_eq!(dom.get(html).children.len(), 1);
        let body = dom.get(html).children[0];
        assert_eq!(tag_name(&dom, body, &source), "body");
    }

    #[test]
    fn test_text_content() {
        let (dom, source) = parse_html("<p>Hello</p>");

        let root = dom.root();
        let p = dom.get(root).children[0];
        assert_eq!(tag_name(&dom, p, &source), "p");

        // p should have 1 child (text "Hello")
        assert_eq!(dom.get(p).children.len(), 1);
        let text = dom.get(p).children[0];
        assert_eq!(text_content(&dom, text, &source), "Hello");
    }

    #[test]
    fn test_nested_elements() {
        let (dom, source) = parse_html("<div><p><span>Deep</span></p></div>");

        let root = dom.root();
        let div = dom.get(root).children[0];
        let p = dom.get(div).children[0];
        let span = dom.get(p).children[0];
        let text = dom.get(span).children[0];

        assert_eq!(tag_name(&dom, div, &source), "div");
        assert_eq!(tag_name(&dom, p, &source), "p");
        assert_eq!(tag_name(&dom, span, &source), "span");
        assert_eq!(text_content(&dom, text, &source), "Deep");
    }

    #[test]
    fn test_self_closing_tag() {
        let (dom, source) = parse_html("<div><br/><p>Text</p></div>");

        let root = dom.root();
        let div = dom.get(root).children[0];

        // div should have 2 children: br and p
        assert_eq!(dom.get(div).children.len(), 2);
        assert_eq!(tag_name(&dom, dom.get(div).children[0], &source), "br");
        assert_eq!(tag_name(&dom, dom.get(div).children[1], &source), "p");
    }

    #[test]
    fn test_void_element() {
        // <br> without the /> should still not become a parent
        let (dom, source) = parse_html("<div><br><p>Text</p></div>");

        let root = dom.root();
        let div = dom.get(root).children[0];

        // div should have 2 children: br and p (br is void, not pushed on stack)
        assert_eq!(dom.get(div).children.len(), 2);
        assert_eq!(tag_name(&dom, dom.get(div).children[0], &source), "br");
        assert_eq!(tag_name(&dom, dom.get(div).children[1], &source), "p");
    }

    #[test]
    fn test_attributes_preserved() {
        let (dom, source) = parse_html(r#"<div class="main" id="container"></div>"#);

        let root = dom.root();
        let div = dom.get(root).children[0];
        let attrs = &dom.get(div).attributes;

        assert_eq!(attrs.len(), 2);

        // First attribute: class="main"
        let (ns, ne, vs, ve) = attrs[0];
        assert_eq!(
            std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap(),
            "class"
        );
        assert_eq!(
            std::str::from_utf8(&source[vs as usize..ve as usize]).unwrap(),
            "main"
        );

        // Second attribute: id="container"
        let (ns, ne, vs, ve) = attrs[1];
        assert_eq!(
            std::str::from_utf8(&source[ns as usize..ne as usize]).unwrap(),
            "id"
        );
        assert_eq!(
            std::str::from_utf8(&source[vs as usize..ve as usize]).unwrap(),
            "container"
        );
    }

    #[test]
    fn test_comment_node() {
        let (dom, source) = parse_html("<div><!-- hello --></div>");

        let root = dom.root();
        let div = dom.get(root).children[0];

        assert_eq!(dom.get(div).children.len(), 1);
        let comment = dom.get(div).children[0];
        if let NodeKind::Comment { start, end } = dom.get(comment).kind {
            let text = std::str::from_utf8(&source[start as usize..end as usize]).unwrap();
            assert_eq!(text.trim(), "hello");
        } else {
            panic!("Expected Comment node");
        }
    }

    #[test]
    fn test_mismatched_tags() {
        // The parser should handle mismatched tags gracefully (lenient parsing)
        // <div><p></div> — the </div> should close both p and div
        let (dom, source) = parse_html("<div><p>Text</div>");

        let root = dom.root();
        let div = dom.get(root).children[0];
        assert_eq!(tag_name(&dom, div, &source), "div");

        // p is a child of div
        let p = dom.get(div).children[0];
        assert_eq!(tag_name(&dom, p, &source), "p");

        // text is a child of p
        let text = dom.get(p).children[0];
        assert_eq!(text_content(&dom, text, &source), "Text");
    }

    #[test]
    fn test_full_document() {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>Asteria Test</title>
</head>
<body>
    <h1 class="main">Hello, Asteria!</h1>
    <p>This is a <strong>test</strong> page.</p>
    <!-- This is a comment -->
    <br/>
    <div id="container">
        <span>Nested content</span>
    </div>
</body>
</html>"#;

        let (dom, source) = parse_html(html);

        // Print the tree for visual inspection
        let output = dom.format_tree(&source);

        // Verify the structure
        assert!(output.contains("Document"));
        assert!(output.contains("Element <html>"));
        assert!(output.contains("Element <head>"));
        assert!(output.contains("Element <title>"));
        assert!(output.contains("Text \"Asteria Test\""));
        assert!(output.contains("Element <body>"));
        assert!(output.contains("Element <h1 class=\"main\">"));
        assert!(output.contains("Text \"Hello, Asteria!\""));
        assert!(output.contains("Element <strong>"));
        assert!(output.contains("Text \"test\""));
        assert!(output.contains("Comment \"This is a comment\""));
        assert!(output.contains("Element <br>"));
        assert!(output.contains("Element <div id=\"container\">"));
        assert!(output.contains("Element <span>"));
        assert!(output.contains("Text \"Nested content\""));
    }

    #[test]
    fn test_print_tree_output() {
        // Verify the exact tree structure for a simple document
        let html = "<html><head><title>Hello</title></head><body><h1>World</h1></body></html>";
        let (dom, source) = parse_html(html);
        let output = dom.format_tree(&source);

        let expected = "\
Document
  Element <html>
    Element <head>
      Element <title>
        Text \"Hello\"
    Element <body>
      Element <h1>
        Text \"World\"
";
        assert_eq!(output, expected);
    }

    #[test]
    fn test_multiple_void_elements() {
        // Multiple void elements in a row should all be siblings, not nested
        let (dom, source) = parse_html("<div><br><hr><img><input></div>");

        let root = dom.root();
        let div = dom.get(root).children[0];

        // All 4 void elements should be direct children of div
        assert_eq!(dom.get(div).children.len(), 4);
        assert_eq!(tag_name(&dom, dom.get(div).children[0], &source), "br");
        assert_eq!(tag_name(&dom, dom.get(div).children[1], &source), "hr");
        assert_eq!(tag_name(&dom, dom.get(div).children[2], &source), "img");
        assert_eq!(tag_name(&dom, dom.get(div).children[3], &source), "input");
    }

    #[test]
    fn test_deeply_nested() {
        let (dom, source) = parse_html("<a><b><c><d><e>Deep</e></d></c></b></a>");

        let root = dom.root();
        let a = dom.get(root).children[0];
        let b = dom.get(a).children[0];
        let c = dom.get(b).children[0];
        let d = dom.get(c).children[0];
        let e = dom.get(d).children[0];
        let text = dom.get(e).children[0];

        assert_eq!(tag_name(&dom, a, &source), "a");
        assert_eq!(tag_name(&dom, e, &source), "e");
        assert_eq!(text_content(&dom, text, &source), "Deep");
    }

    #[test]
    fn test_siblings_with_mixed_content() {
        let (dom, source) = parse_html("<div><p>One</p>Between<p>Two</p></div>");

        let root = dom.root();
        let div = dom.get(root).children[0];

        // div has 3 children: p, text("Between"), p
        assert_eq!(dom.get(div).children.len(), 3);
        assert_eq!(tag_name(&dom, dom.get(div).children[0], &source), "p");
        assert_eq!(
            text_content(&dom, dom.get(div).children[1], &source),
            "Between"
        );
        assert_eq!(tag_name(&dom, dom.get(div).children[2], &source), "p");
    }

    #[test]
    fn test_case_insensitive_end_tags() {
        // <DIV> closed by </div> should work (case-insensitive matching)
        let (dom, source) = parse_html("<DIV><P>Text</p></div>");

        let root = dom.root();
        let div = dom.get(root).children[0];
        assert_eq!(tag_name(&dom, div, &source), "DIV");

        let p = dom.get(div).children[0];
        assert_eq!(tag_name(&dom, p, &source), "P");

        // After </div>, the div should be closed — no children after it on root
        assert_eq!(dom.get(root).children.len(), 1);
    }

    #[test]
    fn test_orphan_end_tag_ignored() {
        // An end tag with no matching start tag should be silently ignored
        let (dom, source) = parse_html("<div>Hello</span></div>");

        let root = dom.root();
        let div = dom.get(root).children[0];
        assert_eq!(tag_name(&dom, div, &source), "div");

        // The </span> is orphaned and ignored — div's children are just the text
        assert_eq!(dom.get(div).children.len(), 1);
        assert_eq!(
            text_content(&dom, dom.get(div).children[0], &source),
            "Hello"
        );
    }
}
