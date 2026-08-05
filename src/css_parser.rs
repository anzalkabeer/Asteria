use crate::css_tokenizer::CssTokenizer;
use crate::css_tokens::{CssToken, CssTokenKind};

// ─── CSS Parser ──────────────────────────────────────────────────
//
// Parses a stream of CSS tokens into a structured Stylesheet — a list
// of style rules, each containing selectors and declarations.
//
// This is a minimal CSSOM (CSS Object Model) sufficient for style
// resolution. It supports:
//   - Tag selectors: h1, div, p
//   - Class selectors: .main, .container
//   - ID selectors: #header, #footer
//   - Universal selector: *
//   - Compound selectors: div.main, h1#title
//   - Descendant combinators: div p, .sidebar .link
//   - Grouped selectors: h1, h2, h3 { ... }
//
// Not supported (v1):
//   - @-rules (@media, @import, @keyframes)
//   - Pseudo-classes (:hover, :first-child)
//   - Pseudo-elements (::before, ::after)
//   - Child (>), sibling (+, ~) combinators
//   - Attribute selectors ([type="text"])

// ─── Data Structures ─────────────────────────────────────────────

/// Combinators connecting steps in a selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// Whitespace (descendant)
    Descendant,
    /// `>` (child)
    Child,
    /// `+` (next sibling)
    NextSibling,
    /// `~` (subsequent sibling)
    SubsequentSibling,
}

/// A single CSS declaration: a property-value pair.
/// e.g. "color: red" → Declaration { property: "color", value: "red" }
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub property: String,
    pub value: String,
}

/// A simple selector — the atomic unit of selector matching.
/// A compound selector is made up of multiple simple selectors
/// that all apply to the same element.
#[derive(Debug, Clone, PartialEq)]
pub enum SimpleSelector {
    /// Matches elements by tag name, e.g. "div", "p", "h1"
    Tag(String),
    /// Matches elements by class attribute, e.g. ".main"
    Class(String),
    /// Matches elements by id attribute, e.g. "#container"
    Id(String),
    /// Matches any element: *
    Universal,
    /// Matches element state or structural position, e.g. ":hover", ":first-child"
    PseudoClass(String),
}

/// A single step in a complex selector: a combinator and a compound selector.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectorStep {
    pub combinator: Combinator,
    pub compound: Vec<SimpleSelector>,
}

/// A selector is a chain of compound selector steps connected by combinators.
#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub steps: Vec<SelectorStep>,
    pub parts: Vec<Vec<SimpleSelector>>,
}

impl Selector {
    pub fn from_steps(steps: Vec<SelectorStep>) -> Self {
        let parts = steps.iter().map(|s| s.compound.clone()).collect();
        Selector { steps, parts }
    }
}

/// A CSS rule: one or more selectors sharing a declaration block.
/// e.g. "h1, h2 { color: red; font-size: 24px; }"
#[derive(Debug, Clone)]
pub struct StyleRule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
    pub position: usize,
}

/// A `@media` rule containing nested style rules.
#[derive(Debug, Clone)]
pub struct MediaRule {
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub rules: Vec<StyleRule>,
}

/// A stylesheet: a list of style rules and media rules parsed from CSS source.
#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    pub rules: Vec<StyleRule>,
    pub media_rules: Vec<MediaRule>,
}

impl Stylesheet {
    /// Convenience constructor: tokenize + parse a CSS byte slice in one step.
    pub fn parse(source: &[u8]) -> Self {
        let mut tokenizer = CssTokenizer::new(source);
        let tokens = tokenizer.tokenize();
        let mut parser = CssParser::new(&tokens, source);
        parser.parse()
    }
}

// ─── The Parser ──────────────────────────────────────────────────

pub struct CssParser<'a> {
    tokens: &'a [CssToken],
    source: &'a [u8],
    pos: usize,
    next_position: usize,
}

impl<'a> CssParser<'a> {
    pub fn new(tokens: &'a [CssToken], source: &'a [u8]) -> Self {
        CssParser {
            tokens,
            source,
            pos: 0,
            next_position: 0,
        }
    }

    pub fn parse(&mut self) -> Stylesheet {
        let mut rules = Vec::new();
        let mut media_rules = Vec::new();

        while !self.at_end() {
            self.skip_whitespace();

            if self.at_end() {
                break;
            }

            if self.current_kind() == CssTokenKind::AtKeyword {
                if self.current_slice().eq_ignore_ascii_case("@media") {
                    if let Some(media_rule) = self.parse_media_rule() {
                        media_rules.push(media_rule);
                    }
                    continue;
                }
                self.skip_at_rule();
                continue;
            }

            // Try to parse a style rule
            if let Some(rule) = self.parse_rule() {
                rules.push(rule);
            }
        }

        Stylesheet { rules, media_rules }
    }

    /// Parse a @media rule block: @media (min-width: 600px) { ... }
    fn parse_media_rule(&mut self) -> Option<MediaRule> {
        self.advance(); // skip @media
        let mut min_width = None;
        let mut max_width = None;

        let mut header_text = String::new();
        while !self.at_end() && self.current_kind() != CssTokenKind::OpenBrace {
            header_text.push_str(self.current_slice());
            self.advance();
        }

        let header_lower = header_text.trim().to_ascii_lowercase();
        let unsupported_media = header_lower.contains("print")
            || header_lower.contains("speech")
            || header_lower.contains("handheld")
            || header_lower.contains("projection")
            || header_lower.contains("tv")
            || header_lower.contains("not screen")
            || header_lower.contains("not all");

        // Parse min-width / max-width from header string
        if let Some(pos) = header_text.find("min-width") {
            let rest = &header_text[pos..];
            if let Some(val_start) = rest.find(':') {
                let num_str: String = rest[val_start + 1..]
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                if let Ok(v) = num_str.parse::<f32>() {
                    min_width = Some(v);
                }
            }
        }

        if let Some(pos) = header_text.find("max-width") {
            let rest = &header_text[pos..];
            if let Some(val_start) = rest.find(':') {
                let num_str: String = rest[val_start + 1..]
                    .trim_start()
                    .chars()
                    .take_while(|c| c.is_ascii_digit() || *c == '.')
                    .collect();
                if let Ok(v) = num_str.parse::<f32>() {
                    max_width = Some(v);
                }
            }
        }

        if self.current_kind() == CssTokenKind::OpenBrace {
            self.advance(); // skip '{'
            let mut rules = Vec::new();

            while !self.at_end() && self.current_kind() != CssTokenKind::CloseBrace {
                self.skip_whitespace();
                if self.current_kind() == CssTokenKind::CloseBrace {
                    break;
                }
                if let Some(rule) = self.parse_rule() {
                    rules.push(rule);
                } else {
                    self.advance();
                }
            }

            if self.current_kind() == CssTokenKind::CloseBrace {
                self.advance(); // skip '}'
            }

            if unsupported_media {
                None
            } else {
                Some(MediaRule {
                    min_width,
                    max_width,
                    rules,
                })
            }
        } else {
            None
        }
    }

    // ─── Rule Parsing ────────────────────────────────────────────

    /// Parse a single style rule: selectors { declarations }
    fn parse_rule(&mut self) -> Option<StyleRule> {
        let selectors = self.parse_selector_list();
        if selectors.is_empty() {
            return None;
        }

        // Expect '{'
        self.skip_whitespace();
        if self.current_kind() != CssTokenKind::OpenBrace {
            // Malformed rule — skip to next '}' or end
            self.skip_to_close_brace();
            return None;
        }
        self.advance(); // skip '{'

        let declarations = self.parse_declarations();

        // Expect '}'
        self.skip_whitespace();
        if self.current_kind() == CssTokenKind::CloseBrace {
            self.advance(); // skip '}'
        }

        let position = self.next_position;
        self.next_position += 1;

        Some(StyleRule {
            selectors,
            declarations,
            position,
        })
    }

    // ─── Selector Parsing ────────────────────────────────────────

    /// Parse a comma-separated list of selectors.
    /// e.g. "h1, h2, .title" → [Selector, Selector, Selector]
    fn parse_selector_list(&mut self) -> Vec<Selector> {
        let mut selectors = Vec::new();

        if let Some(sel) = self.parse_selector() {
            selectors.push(sel);
        } else {
            return selectors;
        }

        // Parse additional selectors separated by ','
        loop {
            self.skip_whitespace();
            if self.current_kind() == CssTokenKind::Comma {
                self.advance(); // skip ','
                self.skip_whitespace();
                if let Some(sel) = self.parse_selector() {
                    selectors.push(sel);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        selectors
    }

    /// Parse a single selector, supporting descendant, child (>), and sibling (+, ~) combinators.
    fn parse_selector(&mut self) -> Option<Selector> {
        let mut steps = Vec::new();

        let initial_compound = self.parse_compound_selector();
        if initial_compound.is_empty() {
            return None;
        }

        steps.push(SelectorStep {
            combinator: Combinator::Descendant,
            compound: initial_compound,
        });

        loop {
            self.skip_whitespace();

            let combinator = match self.current_kind() {
                CssTokenKind::Delim => match self.current_slice() {
                    ">" => {
                        self.advance();
                        self.skip_whitespace();
                        Combinator::Child
                    }
                    "+" => {
                        self.advance();
                        self.skip_whitespace();
                        Combinator::NextSibling
                    }
                    "~" => {
                        self.advance();
                        self.skip_whitespace();
                        Combinator::SubsequentSibling
                    }
                    _ => Combinator::Descendant,
                },
                CssTokenKind::OpenBrace
                | CssTokenKind::Comma
                | CssTokenKind::CloseBrace
                | CssTokenKind::Eof => break,
                _ => Combinator::Descendant,
            };

            let compound = self.parse_compound_selector();
            if compound.is_empty() {
                break;
            }

            steps.push(SelectorStep {
                combinator,
                compound,
            });
        }

        Some(Selector::from_steps(steps))
    }

    /// Parse a compound selector — one or more simple selectors that all
    /// apply to the same element, with no whitespace between them.
    /// e.g. "div.main#hero:first-child" → [Tag("div"), Class("main"), Id("hero"), PseudoClass("first-child")]
    fn parse_compound_selector(&mut self) -> Vec<SimpleSelector> {
        let mut parts = Vec::new();

        loop {
            match self.current_kind() {
                // Tag selector: an identifier like "div", "p", "h1"
                CssTokenKind::Ident => {
                    let name = self.current_slice().to_ascii_lowercase();
                    parts.push(SimpleSelector::Tag(name));
                    self.advance();
                }

                // Class selector: '.' followed by identifier
                CssTokenKind::Dot => {
                    self.advance(); // skip '.'
                    if self.current_kind() == CssTokenKind::Ident {
                        let name = self.current_slice().to_string();
                        parts.push(SimpleSelector::Class(name));
                        self.advance();
                    }
                }

                // ID selector: '#' hash token
                CssTokenKind::Hash => {
                    let text = self.current_slice();
                    // Hash token includes the '#', so strip it
                    let name = text[1..].to_string();
                    parts.push(SimpleSelector::Id(name));
                    self.advance();
                }

                // Universal selector: '*'
                CssTokenKind::Delim if self.current_slice() == "*" => {
                    parts.push(SimpleSelector::Universal);
                    self.advance();
                }

                // Pseudo-class selector: ':' followed by identifier
                CssTokenKind::Colon => {
                    self.advance(); // skip ':'
                    if self.current_kind() == CssTokenKind::Ident {
                        let name = self.current_slice().to_ascii_lowercase();
                        parts.push(SimpleSelector::PseudoClass(name));
                        self.advance();
                    }
                }

                _ => break,
            }
        }

        parts
    }

    // ─── Declaration Parsing ─────────────────────────────────────

    /// Parse declarations inside a rule block (between { and }).
    /// e.g. "color: red; font-size: 16px;" → [Declaration, Declaration]
    fn parse_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();

        loop {
            self.skip_whitespace();

            // Stop at '}' or EOF
            if self.current_kind() == CssTokenKind::CloseBrace || self.at_end() {
                break;
            }

            if let Some(decl) = self.parse_declaration() {
                declarations.push(decl);
            } else {
                // Skip to next ';' or '}' to recover from malformed declarations
                while !self.at_end()
                    && self.current_kind() != CssTokenKind::Semicolon
                    && self.current_kind() != CssTokenKind::CloseBrace
                {
                    self.advance();
                }
                if self.current_kind() == CssTokenKind::Semicolon {
                    self.advance();
                }
            }
        }

        declarations
    }

    /// Parse a single declaration: property ':' value ';'
    fn parse_declaration(&mut self) -> Option<Declaration> {
        // Property name must be an identifier
        if self.current_kind() != CssTokenKind::Ident {
            return None;
        }
        let property = self.current_slice().to_ascii_lowercase();
        self.advance();

        // Skip whitespace, expect ':'
        self.skip_whitespace();
        if self.current_kind() != CssTokenKind::Colon {
            return None;
        }
        self.advance(); // skip ':'

        // Collect value tokens until ';' or '}'
        self.skip_whitespace();
        let mut value_parts: Vec<String> = Vec::new();

        while !self.at_end()
            && self.current_kind() != CssTokenKind::Semicolon
            && self.current_kind() != CssTokenKind::CloseBrace
        {
            if self.current_kind() == CssTokenKind::Whitespace {
                // Preserve a single space for multi-word values like "Helvetica Neue"
                if !value_parts.is_empty() {
                    value_parts.push(" ".to_string());
                }
                self.skip_whitespace();
            } else {
                value_parts.push(self.current_slice().to_string());
                self.advance();
            }
        }

        // Skip trailing ';'
        if self.current_kind() == CssTokenKind::Semicolon {
            self.advance();
        }

        // Trim trailing whitespace from value
        while value_parts.last().map(|s| s.as_str()) == Some(" ") {
            value_parts.pop();
        }

        let value = value_parts.join("");

        if value.is_empty() {
            return None;
        }

        Some(Declaration { property, value })
    }

    // ─── Navigation Helpers ──────────────────────────────────────

    /// Are we at the end of the token stream?
    fn at_end(&self) -> bool {
        self.pos >= self.tokens.len() || self.tokens[self.pos].kind == CssTokenKind::Eof
    }

    /// Get the kind of the current token.
    fn current_kind(&self) -> CssTokenKind {
        if self.pos < self.tokens.len() {
            self.tokens[self.pos].kind
        } else {
            CssTokenKind::Eof
        }
    }

    /// Get the source slice of the current token.
    fn current_slice(&self) -> &str {
        let token = &self.tokens[self.pos];
        std::str::from_utf8(&self.source[token.start as usize..token.end as usize]).unwrap_or("")
    }

    /// Advance to the next token.
    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    /// Skip whitespace tokens.
    fn skip_whitespace(&mut self) {
        while self.pos < self.tokens.len() && self.tokens[self.pos].kind == CssTokenKind::Whitespace
        {
            self.pos += 1;
        }
    }

    /// Skip an @-rule by consuming tokens until matching '}' or end.
    fn skip_at_rule(&mut self) {
        self.advance(); // skip @keyword
        let mut brace_depth = 0;
        while !self.at_end() {
            match self.current_kind() {
                CssTokenKind::OpenBrace => {
                    brace_depth += 1;
                    self.advance();
                }
                CssTokenKind::CloseBrace => {
                    brace_depth -= 1;
                    self.advance();
                    if brace_depth <= 0 {
                        break;
                    }
                }
                CssTokenKind::Semicolon if brace_depth == 0 => {
                    // Simple @-rules like @import end with ';'
                    self.advance();
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    /// Skip tokens until we find a '}' (error recovery).
    fn skip_to_close_brace(&mut self) {
        while !self.at_end() && self.current_kind() != CssTokenKind::CloseBrace {
            self.advance();
        }
        if self.current_kind() == CssTokenKind::CloseBrace {
            self.advance();
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_stylesheet() {
        let stylesheet = Stylesheet::parse(b"");
        assert!(stylesheet.rules.is_empty());
    }

    #[test]
    fn test_single_rule() {
        let stylesheet = Stylesheet::parse(b"h1 { color: red; }");

        assert_eq!(stylesheet.rules.len(), 1);

        let rule = &stylesheet.rules[0];
        assert_eq!(rule.selectors.len(), 1);
        assert_eq!(
            rule.selectors[0].parts,
            vec![vec![SimpleSelector::Tag("h1".to_string())]]
        );
        assert_eq!(rule.declarations.len(), 1);
        assert_eq!(rule.declarations[0].property, "color");
        assert_eq!(rule.declarations[0].value, "red");
    }

    #[test]
    fn test_multiple_declarations() {
        let stylesheet = Stylesheet::parse(b"p { color: blue; font-size: 16px; margin: 10px; }");

        let rule = &stylesheet.rules[0];
        assert_eq!(rule.declarations.len(), 3);
        assert_eq!(rule.declarations[0].property, "color");
        assert_eq!(rule.declarations[0].value, "blue");
        assert_eq!(rule.declarations[1].property, "font-size");
        assert_eq!(rule.declarations[1].value, "16px");
        assert_eq!(rule.declarations[2].property, "margin");
        assert_eq!(rule.declarations[2].value, "10px");
    }

    #[test]
    fn test_multiple_rules() {
        let stylesheet = Stylesheet::parse(b"h1 { color: red; } p { color: blue; }");

        assert_eq!(stylesheet.rules.len(), 2);
        assert_eq!(stylesheet.rules[0].declarations[0].value, "red");
        assert_eq!(stylesheet.rules[1].declarations[0].value, "blue");
    }

    #[test]
    fn test_class_selector() {
        let stylesheet = Stylesheet::parse(b".main { background: white; }");

        let sel = &stylesheet.rules[0].selectors[0];
        assert_eq!(
            sel.parts,
            vec![vec![SimpleSelector::Class("main".to_string())]]
        );
    }

    #[test]
    fn test_id_selector() {
        let stylesheet = Stylesheet::parse(b"#container { width: 960px; }");

        let sel = &stylesheet.rules[0].selectors[0];
        assert_eq!(
            sel.parts,
            vec![vec![SimpleSelector::Id("container".to_string())]]
        );
    }

    #[test]
    fn test_universal_selector() {
        let stylesheet = Stylesheet::parse(b"* { margin: 0; }");

        let sel = &stylesheet.rules[0].selectors[0];
        assert_eq!(sel.parts, vec![vec![SimpleSelector::Universal]]);
    }

    #[test]
    fn test_compound_selector() {
        let stylesheet = Stylesheet::parse(b"div.main { color: red; }");

        let sel = &stylesheet.rules[0].selectors[0];
        assert_eq!(
            sel.parts,
            vec![vec![
                SimpleSelector::Tag("div".to_string()),
                SimpleSelector::Class("main".to_string()),
            ]]
        );
    }

    #[test]
    fn test_descendant_selector() {
        let stylesheet = Stylesheet::parse(b"div p { color: red; }");

        let sel = &stylesheet.rules[0].selectors[0];
        assert_eq!(sel.parts.len(), 2);
        assert_eq!(sel.parts[0], vec![SimpleSelector::Tag("div".to_string())]);
        assert_eq!(sel.parts[1], vec![SimpleSelector::Tag("p".to_string())]);
    }

    #[test]
    fn test_grouped_selectors() {
        let stylesheet = Stylesheet::parse(b"h1, h2, h3 { color: red; }");

        let rule = &stylesheet.rules[0];
        assert_eq!(rule.selectors.len(), 3);
        assert_eq!(
            rule.selectors[0].parts,
            vec![vec![SimpleSelector::Tag("h1".to_string())]]
        );
        assert_eq!(
            rule.selectors[1].parts,
            vec![vec![SimpleSelector::Tag("h2".to_string())]]
        );
        assert_eq!(
            rule.selectors[2].parts,
            vec![vec![SimpleSelector::Tag("h3".to_string())]]
        );
    }

    #[test]
    fn test_at_rule_skipped() {
        let stylesheet =
            Stylesheet::parse(b"@media screen { body { color: red; } } h1 { color: blue; }");

        // The @media rule should be skipped, only h1 rule remains
        assert_eq!(stylesheet.rules.len(), 1);
        assert_eq!(stylesheet.rules[0].declarations[0].value, "blue");
    }

    #[test]
    fn test_comments_in_stylesheet() {
        let stylesheet = Stylesheet::parse(b"/* heading */ h1 { /* text color */ color: red; }");

        assert_eq!(stylesheet.rules.len(), 1);
        assert_eq!(stylesheet.rules[0].declarations[0].property, "color");
        assert_eq!(stylesheet.rules[0].declarations[0].value, "red");
    }

    #[test]
    fn test_hash_color_value() {
        let stylesheet = Stylesheet::parse(b"h1 { color: #ff0000; }");

        assert_eq!(stylesheet.rules[0].declarations[0].value, "#ff0000");
    }

    #[test]
    fn test_child_and_sibling_combinators() {
        let stylesheet = Stylesheet::parse(b"div > p + span ~ a { color: red; }");
        let sel = &stylesheet.rules[0].selectors[0];
        assert_eq!(sel.steps.len(), 4);
        assert_eq!(sel.steps[0].combinator, Combinator::Descendant);
        assert_eq!(sel.steps[1].combinator, Combinator::Child);
        assert_eq!(sel.steps[2].combinator, Combinator::NextSibling);
        assert_eq!(sel.steps[3].combinator, Combinator::SubsequentSibling);
    }

    #[test]
    fn test_pseudo_classes() {
        let stylesheet = Stylesheet::parse(b"p:first-child:hover { color: green; }");
        let sel = &stylesheet.rules[0].selectors[0];
        assert_eq!(
            sel.parts[0],
            vec![
                SimpleSelector::Tag("p".to_string()),
                SimpleSelector::PseudoClass("first-child".to_string()),
                SimpleSelector::PseudoClass("hover".to_string()),
            ]
        );
    }

    #[test]
    fn test_media_rule_parsing() {
        let stylesheet = Stylesheet::parse(
            b"@media (min-width: 600px) and (max-width: 1200px) { h1 { color: purple; } }",
        );
        assert_eq!(stylesheet.media_rules.len(), 1);
        assert_eq!(stylesheet.media_rules[0].min_width, Some(600.0));
        assert_eq!(stylesheet.media_rules[0].max_width, Some(1200.0));
        assert_eq!(stylesheet.media_rules[0].rules.len(), 1);
    }
}
