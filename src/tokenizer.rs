use crate::tokens::{Attribute, Token, TokenKind};

// Tokenizer states — internal to the state machine, not exposed publicly.
// i thin i should have write the token's state oin a different file but i am doing it here and only for now (v1.0).
#[derive(Debug, Clone, Copy, PartialEq)]
enum State {
    Data,
    TagOpen,
    TagName,
    SelfClosingStartTag,
    EndTagOpen,
    BeforeAttributeName,
    AttributeName,
    AfterAttributeName,
    BeforeAttributeValue,
    AttributeValueDoubleQuoted,
    AttributeValueSingleQuoted,
    AttributeValueUnquoted,
    AfterAttributeValue,
    Comment,
    Doctype,
}

/// The HTML tokenizer. Consumes a byte slice and produces a vector of zero-copy tokens.
/// Tokens reference offsets into the original input buffer — no string allocations.
pub struct Tokenizer<'a> {
    input: &'a [u8],
    pos: usize,
    state: State,
    tokens: Vec<Token>,

    // Tracks where the current token started in the input buffer
    token_start: usize,

    // Tag name boundaries (used while building a tag token)
    tag_name_start: usize,
    tag_name_end: usize,

    // Are we currently inside an end tag (</...>)?
    is_end_tag: bool,

    // Attribute currently being constructed
    attr_name_start: usize,
    attr_name_end: usize,
    attr_value_start: usize,
    attr_value_end: usize,

    // Collected attributes for the current tag
    current_attrs: Vec<Attribute>,
}

impl<'a> Tokenizer<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Tokenizer {
            input,
            pos: 0,
            state: State::Data,
            tokens: Vec::new(),
            token_start: 0,
            tag_name_start: 0,
            tag_name_end: 0,
            is_end_tag: false,
            attr_name_start: 0,
            attr_name_end: 0,
            attr_value_start: 0,
            attr_value_end: 0,
            current_attrs: Vec::new(),
        }
    }

    /// Run the tokenizer and return all produced tokens.
    pub fn tokenize(&mut self) -> Vec<Token> {
        while self.pos <= self.input.len() {
            // If we've consumed every byte, flush any remaining text and emit Eof
            if self.pos == self.input.len() {
                if self.state == State::Data && self.pos > self.token_start {
                    self.emit_text();
                }
                self.tokens.push(Token {
                    kind: TokenKind::Eof,
                    start: self.pos as u32,
                    end: self.pos as u32,
                    attributes: Vec::new(),
                });
                break;
            }

            let byte = self.input[self.pos];

            match self.state {
                State::Data => self.handle_data(byte),
                State::TagOpen => self.handle_tag_open(byte),
                State::TagName => self.handle_tag_name(byte),
                State::EndTagOpen => self.handle_end_tag_open(byte),
                State::SelfClosingStartTag => self.handle_self_closing_start_tag(byte),
                State::BeforeAttributeName => self.handle_before_attribute_name(byte),
                State::AttributeName => self.handle_attribute_name(byte),
                State::AfterAttributeName => self.handle_after_attribute_name(byte),
                State::BeforeAttributeValue => self.handle_before_attribute_value(byte),
                State::AttributeValueDoubleQuoted => self.handle_attr_value_double_quoted(byte),
                State::AttributeValueSingleQuoted => self.handle_attr_value_single_quoted(byte),
                State::AttributeValueUnquoted => self.handle_attr_value_unquoted(byte),
                State::AfterAttributeValue => self.handle_after_attribute_value(byte),
                State::Comment => self.handle_comment(byte),
                State::Doctype => self.handle_doctype(byte),
            }

            self.pos += 1;
        }

        std::mem::take(&mut self.tokens)
    }

    // ─── State Handlers ──────────────────────────────────────────────

    /// Data state: default state, accumulates text until we hit '<'
    fn handle_data(&mut self, byte: u8) {
        if byte == b'<' {
            // Flush any accumulated text before the '<'
            if self.pos > self.token_start {
                self.emit_text();
            }
            self.token_start = self.pos;
            self.state = State::TagOpen;
        }
        // Otherwise keep accumulating — pos advances, token_start stays put
    }

    /// TagOpen state: we just saw '<', figure out what kind of tag this is
    fn handle_tag_open(&mut self, byte: u8) {
        match byte {
            b'/' => {
                self.is_end_tag = true;
                self.state = State::EndTagOpen;
            }
            b'!' => {
                // Could be a comment (<!--) or doctype (<!DOCTYPE)
                // Peek ahead to decide
                if self.starts_with_at(self.pos, b"!--") {
                    // Skip past "!--" (we're currently on '!', advance past '--')
                    self.pos += 2; // will be incremented once more by the main loop
                    self.state = State::Comment;
                } else if self.starts_with_at_case_insensitive(self.pos, b"!doctype") {
                    self.state = State::Doctype;
                } else {
                    // Unknown <! construct — treat '<!' as text
                    self.state = State::Data;
                }
            }
            b'a'..=b'z' | b'A'..=b'Z' => {
                self.is_end_tag = false;
                self.tag_name_start = self.pos;
                self.current_attrs.clear();
                self.state = State::TagName;
            }
            _ => {
                // Not a valid tag start — treat the '<' as text
                self.state = State::Data;
            }
        }
    }

    /// EndTagOpen state: we saw '</', now read the tag name
    fn handle_end_tag_open(&mut self, byte: u8) {
        match byte {
            b'a'..=b'z' | b'A'..=b'Z' => {
                self.tag_name_start = self.pos;
                self.state = State::TagName;
            }
            b'>' => {
                // '</>' — malformed, ignore it and go back to Data
                self.state = State::Data;
                self.token_start = self.pos + 1;
                self.is_end_tag = false;
            }
            _ => {
                // Malformed end tag — treat as text
                self.state = State::Data;
            }
        }
    }

    /// TagName state: reading the tag name character by character
    fn handle_tag_name(&mut self, byte: u8) {
        match byte {
            b' ' | b'\t' | b'\n' | b'\r' => {
                self.tag_name_end = self.pos;
                self.state = State::BeforeAttributeName;
            }
            b'/' => {
                self.tag_name_end = self.pos;
                self.state = State::SelfClosingStartTag;
            }
            b'>' => {
                self.tag_name_end = self.pos;
                self.emit_tag();
                self.state = State::Data;
                self.token_start = self.pos + 1;
            }
            _ => {
                // Still reading the tag name — do nothing, pos will advance
            }
        }
    }

    /// SelfClosingStartTag state: we saw '/' inside a tag, expecting '>'
    fn handle_self_closing_start_tag(&mut self, byte: u8) {
        if byte == b'>' {
            self.is_end_tag = false; // self-closing tags are not end tags
            self.emit_self_closing_tag();
            self.state = State::Data;
            self.token_start = self.pos + 1;
        } else {
            // '/' not followed by '>' — go back to BeforeAttributeName
            // (e.g. `<div / class="x">` is technically invalid but we handle it)
            self.state = State::BeforeAttributeName;
        }
    }

    /// BeforeAttributeName state: whitespace between tag name and attributes, or between attributes
    fn handle_before_attribute_name(&mut self, byte: u8) {
        match byte {
            b' ' | b'\t' | b'\n' | b'\r' => {
                // Skip whitespace
            }
            b'/' => {
                self.state = State::SelfClosingStartTag;
            }
            b'>' => {
                self.emit_tag();
                self.state = State::Data;
                self.token_start = self.pos + 1;
            }
            _ => {
                // Start of an attribute name
                self.attr_name_start = self.pos;
                self.state = State::AttributeName;
            }
        }
    }

    /// AttributeName state: reading an attribute name
    fn handle_attribute_name(&mut self, byte: u8) {
        match byte {
            b'=' => {
                self.attr_name_end = self.pos;
                self.state = State::BeforeAttributeValue;
            }
            b' ' | b'\t' | b'\n' | b'\r' => {
                self.attr_name_end = self.pos;
                self.state = State::AfterAttributeName;
            }
            b'/' => {
                // Attribute without value (e.g. `<input disabled/>`)
                self.attr_name_end = self.pos;
                self.push_attribute_no_value();
                self.state = State::SelfClosingStartTag;
            }
            b'>' => {
                // Attribute without value at end of tag (e.g. `<input disabled>`)
                self.attr_name_end = self.pos;
                self.push_attribute_no_value();
                self.emit_tag();
                self.state = State::Data;
                self.token_start = self.pos + 1;
            }
            _ => {
                // Still reading attribute name
            }
        }
    }

    /// AfterAttributeName state: saw whitespace after attribute name, could be `=` or another attribute
    fn handle_after_attribute_name(&mut self, byte: u8) {
        match byte {
            b' ' | b'\t' | b'\n' | b'\r' => {
                // Skip whitespace
            }
            b'=' => {
                self.state = State::BeforeAttributeValue;
            }
            b'/' => {
                self.push_attribute_no_value();
                self.state = State::SelfClosingStartTag;
            }
            b'>' => {
                self.push_attribute_no_value();
                self.emit_tag();
                self.state = State::Data;
                self.token_start = self.pos + 1;
            }
            _ => {
                // Previous attribute had no value — push it, start new attribute
                self.push_attribute_no_value();
                self.attr_name_start = self.pos;
                self.state = State::AttributeName;
            }
        }
    }

    /// BeforeAttributeValue state: just saw '=', expecting the value
    fn handle_before_attribute_value(&mut self, byte: u8) {
        match byte {
            b' ' | b'\t' | b'\n' | b'\r' => {
                // Skip whitespace between '=' and value
            }
            b'"' => {
                self.attr_value_start = self.pos + 1; // value starts after the opening quote
                self.state = State::AttributeValueDoubleQuoted;
            }
            b'\'' => {
                self.attr_value_start = self.pos + 1;
                self.state = State::AttributeValueSingleQuoted;
            }
            b'>' => {
                // `attr=>` — malformed, push attribute with empty value
                self.attr_value_start = self.pos;
                self.attr_value_end = self.pos;
                self.push_attribute();
                self.emit_tag();
                self.state = State::Data;
                self.token_start = self.pos + 1;
            }
            _ => {
                // Unquoted attribute value
                self.attr_value_start = self.pos;
                self.state = State::AttributeValueUnquoted;
            }
        }
    }

    /// AttributeValueDoubleQuoted state: reading value inside "..."
    fn handle_attr_value_double_quoted(&mut self, byte: u8) {
        if byte == b'"' {
            self.attr_value_end = self.pos; // value ends before the closing quote
            self.push_attribute();
            self.state = State::AfterAttributeValue;
        }
        // Otherwise keep reading — value continues
    }

    /// AttributeValueSingleQuoted state: reading value inside '...'
    fn handle_attr_value_single_quoted(&mut self, byte: u8) {
        if byte == b'\'' {
            self.attr_value_end = self.pos;
            self.push_attribute();
            self.state = State::AfterAttributeValue;
        }
    }

    /// AttributeValueUnquoted state: reading a bare value (no quotes)
    fn handle_attr_value_unquoted(&mut self, byte: u8) {
        match byte {
            b' ' | b'\t' | b'\n' | b'\r' => {
                self.attr_value_end = self.pos;
                self.push_attribute();
                self.state = State::BeforeAttributeName;
            }
            b'>' => {
                self.attr_value_end = self.pos;
                self.push_attribute();
                self.emit_tag();
                self.state = State::Data;
                self.token_start = self.pos + 1;
            }
            b'/' => {
                // Could be end of unquoted value before self-closing
                self.attr_value_end = self.pos;
                self.push_attribute();
                self.state = State::SelfClosingStartTag;
            }
            _ => {
                // Keep reading value
            }
        }
    }

    /// AfterAttributeValue state: just finished a quoted attribute value
    fn handle_after_attribute_value(&mut self, byte: u8) {
        match byte {
            b' ' | b'\t' | b'\n' | b'\r' => {
                self.state = State::BeforeAttributeName;
            }
            b'/' => {
                self.state = State::SelfClosingStartTag;
            }
            b'>' => {
                self.emit_tag();
                self.state = State::Data;
                self.token_start = self.pos + 1;
            }
            _ => {
                // Missing whitespace between attributes — be lenient, start a new attr
                self.attr_name_start = self.pos;
                self.state = State::AttributeName;
            }
        }
    }

    /// Comment state: inside <!-- ... -->, scanning for the closing -->
    fn handle_comment(&mut self, byte: u8) {
        if byte == b'-' && self.starts_with_at(self.pos, b"-->") {
            // Found the closing '-->'
            // token_start points to '<', comment content is between '<!--' and '-->'
            let comment_content_start = self.token_start + 4; // skip past '<!--'
            let comment_content_end = self.pos; // ends before '-->'
            self.tokens.push(Token {
                kind: TokenKind::Comment,
                start: comment_content_start as u32,
                end: comment_content_end as u32,
                attributes: Vec::new(),
            });
            self.pos += 2; // skip past '-->' (main loop adds 1 more)
            self.state = State::Data;
            self.token_start = self.pos + 1;
        }
        // Otherwise keep scanning
    }

    /// Doctype state: inside <!DOCTYPE ...>, scanning for '>'
    fn handle_doctype(&mut self, byte: u8) {
        if byte == b'>' {
            self.tokens.push(Token {
                kind: TokenKind::Doctype,
                start: self.token_start as u32,
                end: (self.pos + 1) as u32,
                attributes: Vec::new(),
            });
            self.state = State::Data;
            self.token_start = self.pos + 1;
        }
    }

    // ─── Emit Helpers ────────────────────────────────────────────────

    /// Emit a Text token from token_start to current pos
    fn emit_text(&mut self) {
        self.tokens.push(Token {
            kind: TokenKind::Text,
            start: self.token_start as u32,
            end: self.pos as u32,
            attributes: Vec::new(),
        });
    }

    /// Emit a StartTag or EndTag token
    fn emit_tag(&mut self) {
        let kind = if self.is_end_tag {
            TokenKind::EndTag
        } else {
            TokenKind::StartTag
        };

        self.tokens.push(Token {
            kind,
            start: self.tag_name_start as u32,
            end: self.tag_name_end as u32,
            attributes: std::mem::take(&mut self.current_attrs),
        });

        self.is_end_tag = false;
    }

    /// Emit a SelfClosingTag token
    fn emit_self_closing_tag(&mut self) {
        self.tokens.push(Token {
            kind: TokenKind::SelfClosingTag,
            start: self.tag_name_start as u32,
            end: self.tag_name_end as u32,
            attributes: std::mem::take(&mut self.current_attrs),
        });
    }

    /// Push a completed attribute (with value) to current_attrs
    fn push_attribute(&mut self) {
        self.current_attrs.push(Attribute {
            name_start: self.attr_name_start as u32,
            name_end: self.attr_name_end as u32,
            value_start: self.attr_value_start as u32,
            value_end: self.attr_value_end as u32,
        });
    }

    /// Push an attribute that has no value (e.g. `disabled`, `checked`)
    fn push_attribute_no_value(&mut self) {
        self.current_attrs.push(Attribute {
            name_start: self.attr_name_start as u32,
            name_end: self.attr_name_end as u32,
            // No value — start == end signals empty
            value_start: 0,
            value_end: 0,
        });
    }

    // ─── Utility Helpers ─────────────────────────────────────────────

    /// Check if input starting at `pos` matches the given byte sequence
    fn starts_with_at(&self, pos: usize, needle: &[u8]) -> bool {
        if pos + needle.len() > self.input.len() {
            return false;
        }
        &self.input[pos..pos + needle.len()] == needle
    }

    /// Case-insensitive version of starts_with_at
    fn starts_with_at_case_insensitive(&self, pos: usize, needle: &[u8]) -> bool {
        if pos + needle.len() > self.input.len() {
            return false;
        }
        for (i, &b) in needle.iter().enumerate() {
            if !self.input[pos + i].eq_ignore_ascii_case(&b) {
                return false;
            }
        }
        true
    }
}

// ─── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: tokenize an HTML string and return the tokens
    fn tokenize(html: &str) -> Vec<Token> {
        let mut tokenizer = Tokenizer::new(html.as_bytes());
        tokenizer.tokenize()
    }

    /// Helper: extract the text slice a token points to
    fn slice<'a>(html: &'a str, token: &Token) -> &'a str {
        &html[token.start as usize..token.end as usize]
    }

    #[test]
    fn test_empty_input() {
        let tokens = tokenize("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_text_only() {
        let html = "Hello, world!";
        let tokens = tokenize(html);
        assert_eq!(tokens.len(), 2); // Text + Eof
        assert_eq!(tokens[0].kind, TokenKind::Text);
        assert_eq!(slice(html, &tokens[0]), "Hello, world!");
    }

    #[test]
    fn test_single_tag() {
        let html = "<div></div>";
        let tokens = tokenize(html);
        assert_eq!(tokens.len(), 3); // StartTag + EndTag + Eof
        assert_eq!(tokens[0].kind, TokenKind::StartTag);
        assert_eq!(slice(html, &tokens[0]), "div");
        assert_eq!(tokens[1].kind, TokenKind::EndTag);
        assert_eq!(slice(html, &tokens[1]), "div");
    }

    #[test]
    fn test_nested_tags() {
        let html = "<div><p>Hello</p></div>";
        let tokens = tokenize(html);
        // StartTag(div) + StartTag(p) + Text("Hello") + EndTag(p) + EndTag(div) + Eof
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].kind, TokenKind::StartTag);
        assert_eq!(slice(html, &tokens[0]), "div");
        assert_eq!(tokens[1].kind, TokenKind::StartTag);
        assert_eq!(slice(html, &tokens[1]), "p");
        assert_eq!(tokens[2].kind, TokenKind::Text);
        assert_eq!(slice(html, &tokens[2]), "Hello");
        assert_eq!(tokens[3].kind, TokenKind::EndTag);
        assert_eq!(slice(html, &tokens[3]), "p");
        assert_eq!(tokens[4].kind, TokenKind::EndTag);
        assert_eq!(slice(html, &tokens[4]), "div");
    }

    #[test]
    fn test_self_closing_tag() {
        let html = "<br/>";
        let tokens = tokenize(html);
        assert_eq!(tokens.len(), 2); // SelfClosingTag + Eof
        assert_eq!(tokens[0].kind, TokenKind::SelfClosingTag);
        assert_eq!(slice(html, &tokens[0]), "br");
    }

    #[test]
    fn test_attribute_double_quoted() {
        let html = r#"<div class="main" id="container"></div>"#;
        let tokens = tokenize(html);
        assert_eq!(tokens[0].kind, TokenKind::StartTag);
        assert_eq!(slice(html, &tokens[0]), "div");
        assert_eq!(tokens[0].attributes.len(), 2);

        let attr0 = &tokens[0].attributes[0];
        assert_eq!(
            &html[attr0.name_start as usize..attr0.name_end as usize],
            "class"
        );
        assert_eq!(
            &html[attr0.value_start as usize..attr0.value_end as usize],
            "main"
        );

        let attr1 = &tokens[0].attributes[1];
        assert_eq!(
            &html[attr1.name_start as usize..attr1.name_end as usize],
            "id"
        );
        assert_eq!(
            &html[attr1.value_start as usize..attr1.value_end as usize],
            "container"
        );
    }

    #[test]
    fn test_attribute_single_quoted() {
        let html = "<div class='main'></div>";
        let tokens = tokenize(html);
        let attr = &tokens[0].attributes[0];
        assert_eq!(
            &html[attr.name_start as usize..attr.name_end as usize],
            "class"
        );
        assert_eq!(
            &html[attr.value_start as usize..attr.value_end as usize],
            "main"
        );
    }

    #[test]
    fn test_attribute_unquoted() {
        let html = "<div class=main></div>";
        let tokens = tokenize(html);
        let attr = &tokens[0].attributes[0];
        assert_eq!(
            &html[attr.name_start as usize..attr.name_end as usize],
            "class"
        );
        assert_eq!(
            &html[attr.value_start as usize..attr.value_end as usize],
            "main"
        );
    }

    #[test]
    fn test_attribute_no_value() {
        let html = "<input disabled>";
        let tokens = tokenize(html);
        assert_eq!(tokens[0].kind, TokenKind::StartTag);
        assert_eq!(tokens[0].attributes.len(), 1);
        let attr = &tokens[0].attributes[0];
        assert_eq!(
            &html[attr.name_start as usize..attr.name_end as usize],
            "disabled"
        );
        // No value — both offsets are 0
        assert_eq!(attr.value_start, 0);
        assert_eq!(attr.value_end, 0);
    }

    #[test]
    fn test_comment() {
        let html = "<!-- hello world -->";
        let tokens = tokenize(html);
        assert_eq!(tokens.len(), 2); // Comment + Eof
        assert_eq!(tokens[0].kind, TokenKind::Comment);
        assert_eq!(slice(html, &tokens[0]).trim(), "hello world");
    }

    #[test]
    fn test_doctype() {
        let html = "<!DOCTYPE html><html></html>";
        let tokens = tokenize(html);
        assert_eq!(tokens[0].kind, TokenKind::Doctype);
        assert_eq!(tokens[1].kind, TokenKind::StartTag);
        assert_eq!(slice(html, &tokens[1]), "html");
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

        let tokens = tokenize(html);

        // Verify we get a reasonable number of tokens and the structure is correct
        assert!(
            tokens.len() > 10,
            "Expected many tokens, got {}",
            tokens.len()
        );

        // First token should be Doctype
        assert_eq!(tokens[0].kind, TokenKind::Doctype);

        // Last token should be Eof
        assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);

        // Should contain a comment
        let has_comment = tokens.iter().any(|t| t.kind == TokenKind::Comment);
        assert!(has_comment, "Expected a comment token");

        // Should contain a self-closing tag (br)
        let has_self_closing = tokens.iter().any(|t| t.kind == TokenKind::SelfClosingTag);
        assert!(has_self_closing, "Expected a self-closing tag");

        // Verify h1 has the class attribute
        let h1 = tokens
            .iter()
            .find(|t| t.kind == TokenKind::StartTag && slice(html, t) == "h1")
            .expect("Expected an h1 tag");
        assert_eq!(h1.attributes.len(), 1);
        let attr = &h1.attributes[0];
        assert_eq!(
            &html[attr.name_start as usize..attr.name_end as usize],
            "class"
        );
        assert_eq!(
            &html[attr.value_start as usize..attr.value_end as usize],
            "main"
        );
    }

    #[test]
    fn test_self_closing_with_attributes() {
        let html = r#"<img src="photo.jpg" alt="A photo"/>"#;
        let tokens = tokenize(html);
        assert_eq!(tokens[0].kind, TokenKind::SelfClosingTag);
        assert_eq!(slice(html, &tokens[0]), "img");
        assert_eq!(tokens[0].attributes.len(), 2);

        let attr0 = &tokens[0].attributes[0];
        assert_eq!(
            &html[attr0.name_start as usize..attr0.name_end as usize],
            "src"
        );
        assert_eq!(
            &html[attr0.value_start as usize..attr0.value_end as usize],
            "photo.jpg"
        );

        let attr1 = &tokens[0].attributes[1];
        assert_eq!(
            &html[attr1.name_start as usize..attr1.name_end as usize],
            "alt"
        );
        assert_eq!(
            &html[attr1.value_start as usize..attr1.value_end as usize],
            "A photo"
        );
    }

    #[test]
    fn test_text_between_sibling_tags() {
        let html = "<p>Hello</p>World<p>Again</p>";
        let tokens = tokenize(html);
        // StartTag(p) + Text(Hello) + EndTag(p) + Text(World) + StartTag(p) + Text(Again) + EndTag(p) + Eof
        assert_eq!(tokens.len(), 8);
        assert_eq!(tokens[0].kind, TokenKind::StartTag);
        assert_eq!(tokens[1].kind, TokenKind::Text);
        assert_eq!(slice(html, &tokens[1]), "Hello");
        assert_eq!(tokens[2].kind, TokenKind::EndTag);
        assert_eq!(tokens[3].kind, TokenKind::Text);
        assert_eq!(slice(html, &tokens[3]), "World");
        assert_eq!(tokens[4].kind, TokenKind::StartTag);
        assert_eq!(tokens[5].kind, TokenKind::Text);
        assert_eq!(slice(html, &tokens[5]), "Again");
    }

    #[test]
    fn test_uppercase_tags() {
        let html = "<DIV><P>Hello</P></DIV>";
        let tokens = tokenize(html);
        assert_eq!(tokens[0].kind, TokenKind::StartTag);
        assert_eq!(slice(html, &tokens[0]), "DIV");
        assert_eq!(tokens[1].kind, TokenKind::StartTag);
        assert_eq!(slice(html, &tokens[1]), "P");
    }

    #[test]
    fn test_multiple_boolean_attributes() {
        let html = "<input type=\"text\" disabled required readonly>";
        let tokens = tokenize(html);
        assert_eq!(tokens[0].kind, TokenKind::StartTag);
        assert_eq!(tokens[0].attributes.len(), 4);

        // type has a value
        let attr0 = &tokens[0].attributes[0];
        assert_eq!(
            &html[attr0.name_start as usize..attr0.name_end as usize],
            "type"
        );
        assert_eq!(
            &html[attr0.value_start as usize..attr0.value_end as usize],
            "text"
        );

        // disabled, required, readonly have no values
        for attr in &tokens[0].attributes[1..] {
            assert_eq!(attr.value_start, 0);
            assert_eq!(attr.value_end, 0);
        }
    }

    #[test]
    fn test_adjacent_tags_no_whitespace() {
        let html = "<a><b><c></c></b></a>";
        let tokens = tokenize(html);
        // 3 start tags + 3 end tags + Eof = 7
        assert_eq!(tokens.len(), 7);
        assert_eq!(slice(html, &tokens[0]), "a");
        assert_eq!(slice(html, &tokens[1]), "b");
        assert_eq!(slice(html, &tokens[2]), "c");
        assert_eq!(tokens[3].kind, TokenKind::EndTag);
        assert_eq!(tokens[4].kind, TokenKind::EndTag);
        assert_eq!(tokens[5].kind, TokenKind::EndTag);
    }
}
