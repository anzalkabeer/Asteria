use crate::css_tokens::{CssToken, CssTokenKind};

// ─── CSS Tokenizer ───────────────────────────────────────────────
//
// Consumes a CSS byte slice and produces a vector of zero-copy tokens.
// Unlike the HTML tokenizer (which is a full state machine), the CSS
// tokenizer is simpler — CSS has a more regular grammar.
//
// Key behaviors:
//   - Skips comments (/* ... */)
//   - Recognizes identifiers, numbers, dimensions, percentages
//   - Handles quoted strings (double and single)
//   - Recognizes hash tokens (#id, #color)
//   - Emits punctuation tokens (:, ;, {, }, ., ,, (, ))
//   - Collapses whitespace runs into single Whitespace tokens
//   - Falls back to Delim for unrecognized single characters

pub struct CssTokenizer<'a> {
    input: &'a [u8],
    pos: usize,
    tokens: Vec<CssToken>,
}

impl<'a> CssTokenizer<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        CssTokenizer {
            input,
            pos: 0,
            tokens: Vec::new(),
        }
    }

    /// Run the tokenizer and return all produced tokens.
    pub fn tokenize(&mut self) -> Vec<CssToken> {
        while self.pos < self.input.len() {
            let byte = self.input[self.pos];

            match byte {
                // ─── Whitespace ──────────────────────────────
                b' ' | b'\t' | b'\n' | b'\r' => {
                    self.consume_whitespace();
                }

                // ─── Comments ────────────────────────────────
                b'/' if self.peek_at(1) == Some(b'*') => {
                    self.skip_comment();
                }

                // ─── Strings ─────────────────────────────────
                b'"' | b'\'' => {
                    self.consume_string(byte);
                }

                // ─── Hash (ID selectors and hex colors) ──────
                b'#' => {
                    let start = self.pos as u32;
                    self.pos += 1; // skip '#'
                    // Consume the identifier/hex part after '#'
                    while self.pos < self.input.len() && is_ident_char(self.input[self.pos]) {
                        self.pos += 1;
                    }
                    self.tokens.push(CssToken {
                        kind: CssTokenKind::Hash,
                        start,
                        end: self.pos as u32,
                    });
                }

                // ─── At-keywords (@media, @import, etc.) ─────
                b'@' => {
                    let start = self.pos as u32;
                    self.pos += 1; // skip '@'
                    while self.pos < self.input.len() && is_ident_char(self.input[self.pos]) {
                        self.pos += 1;
                    }
                    self.tokens.push(CssToken {
                        kind: CssTokenKind::AtKeyword,
                        start,
                        end: self.pos as u32,
                    });
                }

                // ─── Numbers, Dimensions, Percentages ────────
                b'0'..=b'9' => {
                    self.consume_numeric();
                }

                // ─── Identifiers and Functions ───────────────
                _ if is_ident_start(byte) => {
                    self.consume_ident_or_function();
                }

                // Negative numbers or identifiers starting with '-'
                b'-' if self.pos + 1 < self.input.len()
                    && (self.input[self.pos + 1].is_ascii_digit()
                        || is_ident_start(self.input[self.pos + 1])) =>
                {
                    if self.input[self.pos + 1].is_ascii_digit() {
                        self.consume_numeric();
                    } else {
                        self.consume_ident_or_function();
                    }
                }

                // ─── Single-character tokens ─────────────────
                b':' => {
                    self.emit_single(CssTokenKind::Colon);
                }
                b';' => {
                    self.emit_single(CssTokenKind::Semicolon);
                }
                b',' => {
                    self.emit_single(CssTokenKind::Comma);
                }
                b'{' => {
                    self.emit_single(CssTokenKind::OpenBrace);
                }
                b'}' => {
                    self.emit_single(CssTokenKind::CloseBrace);
                }
                b'(' => {
                    self.emit_single(CssTokenKind::OpenParen);
                }
                b')' => {
                    self.emit_single(CssTokenKind::CloseParen);
                }
                b'.' => {
                    self.emit_single(CssTokenKind::Dot);
                }

                // ─── Anything else → Delim ───────────────────
                _ => {
                    self.emit_single(CssTokenKind::Delim);
                }
            }
        }

        // Emit EOF
        self.tokens.push(CssToken {
            kind: CssTokenKind::Eof,
            start: self.pos as u32,
            end: self.pos as u32,
        });

        std::mem::take(&mut self.tokens)
    }

    // ─── Consumers ───────────────────────────────────────────────

    /// Consume a run of whitespace characters into a single Whitespace token.
    fn consume_whitespace(&mut self) {
        let start = self.pos as u32;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
        self.tokens.push(CssToken {
            kind: CssTokenKind::Whitespace,
            start,
            end: self.pos as u32,
        });
    }

    /// Skip a CSS comment (/* ... */). Does not emit a token.
    fn skip_comment(&mut self) {
        self.pos += 2; // skip '/*'
        while self.pos + 1 < self.input.len() {
            if self.input[self.pos] == b'*' && self.input[self.pos + 1] == b'/' {
                self.pos += 2; // skip '*/'
                return;
            }
            self.pos += 1;
        }
        // Unterminated comment — consume rest of input
        self.pos = self.input.len();
    }

    /// Consume a quoted string (double or single quoted).
    /// Emits a String token whose start..end is the content INSIDE the quotes.
    fn consume_string(&mut self, quote: u8) {
        self.pos += 1; // skip opening quote
        let start = self.pos as u32;
        while self.pos < self.input.len() {
            let byte = self.input[self.pos];
            if byte == quote {
                let end = self.pos as u32;
                self.pos += 1; // skip closing quote
                self.tokens.push(CssToken {
                    kind: CssTokenKind::String,
                    start,
                    end,
                });
                return;
            }
            if byte == b'\\' && self.pos + 1 < self.input.len() {
                self.pos += 1; // skip escaped character
            }
            self.pos += 1;
        }
        // Unterminated string — emit what we have
        self.tokens.push(CssToken {
            kind: CssTokenKind::String,
            start,
            end: self.pos as u32,
        });
    }

    /// Consume a numeric token. Could be a Number, Dimension, or Percentage
    /// depending on what follows the digits.
    fn consume_numeric(&mut self) {
        let start = self.pos as u32;

        // Optional leading '-'
        if self.pos < self.input.len() && self.input[self.pos] == b'-' {
            self.pos += 1;
        }

        // Integer part
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }

        // Decimal part
        if self.pos + 1 < self.input.len()
            && self.input[self.pos] == b'.'
            && self.input[self.pos + 1].is_ascii_digit()
        {
            self.pos += 1; // skip '.'
            while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
        }

        // Check what follows: unit suffix → Dimension, '%' → Percentage, else → Number
        if self.pos < self.input.len() && self.input[self.pos] == b'%' {
            self.pos += 1;
            self.tokens.push(CssToken {
                kind: CssTokenKind::Percentage,
                start,
                end: self.pos as u32,
            });
        } else if self.pos < self.input.len() && is_ident_start(self.input[self.pos]) {
            // Consume the unit suffix (px, em, rem, vh, vw, etc.)
            while self.pos < self.input.len() && is_ident_char(self.input[self.pos]) {
                self.pos += 1;
            }
            self.tokens.push(CssToken {
                kind: CssTokenKind::Dimension,
                start,
                end: self.pos as u32,
            });
        } else {
            self.tokens.push(CssToken {
                kind: CssTokenKind::Number,
                start,
                end: self.pos as u32,
            });
        }
    }

    /// Consume an identifier. If followed by '(', emit as Function; otherwise Ident.
    fn consume_ident_or_function(&mut self) {
        let start = self.pos as u32;

        // Consume the identifier characters (including leading '-' for custom properties)
        if self.pos < self.input.len() && self.input[self.pos] == b'-' {
            self.pos += 1;
        }
        while self.pos < self.input.len() && is_ident_char(self.input[self.pos]) {
            self.pos += 1;
        }

        // If immediately followed by '(', this is a function token
        if self.pos < self.input.len() && self.input[self.pos] == b'(' {
            self.tokens.push(CssToken {
                kind: CssTokenKind::Function,
                start,
                end: self.pos as u32, // end before the '('
            });
            // The '(' will be emitted as OpenParen on the next iteration
        } else {
            self.tokens.push(CssToken {
                kind: CssTokenKind::Ident,
                start,
                end: self.pos as u32,
            });
        }
    }

    /// Emit a single-character token and advance position.
    fn emit_single(&mut self, kind: CssTokenKind) {
        self.tokens.push(CssToken {
            kind,
            start: self.pos as u32,
            end: (self.pos + 1) as u32,
        });
        self.pos += 1;
    }

    // ─── Utility ─────────────────────────────────────────────────

    /// Peek at a byte relative to current position without advancing.
    fn peek_at(&self, offset: usize) -> Option<u8> {
        let idx = self.pos + offset;
        if idx < self.input.len() {
            Some(self.input[idx])
        } else {
            None
        }
    }
}

// ─── Character Classification ────────────────────────────────────

/// Can this byte start a CSS identifier?
/// Identifiers start with a letter, underscore, or '-' (for custom properties).
fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

/// Can this byte continue a CSS identifier?
/// Identifiers can contain letters, digits, underscores, and hyphens.
fn is_ident_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-'
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css_tokens::CssTokenKind;

    /// Helper: tokenize a CSS string and return the tokens
    fn tokenize(css: &str) -> Vec<CssToken> {
        let mut tokenizer = CssTokenizer::new(css.as_bytes());
        tokenizer.tokenize()
    }

    /// Helper: extract the text slice a token points to
    fn slice<'a>(css: &'a str, token: &CssToken) -> &'a str {
        &css[token.start as usize..token.end as usize]
    }

    #[test]
    fn test_empty_input() {
        let tokens = tokenize("");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, CssTokenKind::Eof);
    }

    #[test]
    fn test_simple_rule() {
        let css = "h1 { color: red; }";
        let tokens = tokenize(css);

        // Filter out whitespace and Eof for easier assertion
        let significant: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind != CssTokenKind::Whitespace && t.kind != CssTokenKind::Eof)
            .collect();

        // h1 { color : red ; }
        assert_eq!(significant.len(), 7);
        assert_eq!(significant[0].kind, CssTokenKind::Ident); // h1
        assert_eq!(slice(css, significant[0]), "h1");
        assert_eq!(significant[1].kind, CssTokenKind::OpenBrace); // {
        assert_eq!(significant[2].kind, CssTokenKind::Ident); // color
        assert_eq!(slice(css, significant[2]), "color");
        assert_eq!(significant[3].kind, CssTokenKind::Colon); // :
        assert_eq!(significant[4].kind, CssTokenKind::Ident); // red
        assert_eq!(slice(css, significant[4]), "red");
        assert_eq!(significant[5].kind, CssTokenKind::Semicolon); // ;
        assert_eq!(significant[6].kind, CssTokenKind::CloseBrace); // }
    }

    #[test]
    fn test_class_and_id_selectors() {
        let css = ".main #container { }";
        let tokens = tokenize(css);

        let significant: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind != CssTokenKind::Whitespace && t.kind != CssTokenKind::Eof)
            .collect();

        assert_eq!(significant[0].kind, CssTokenKind::Dot);
        assert_eq!(significant[1].kind, CssTokenKind::Ident);
        assert_eq!(slice(css, significant[1]), "main");
        assert_eq!(significant[2].kind, CssTokenKind::Hash);
        assert_eq!(slice(css, significant[2]), "#container");
    }

    #[test]
    fn test_numbers_and_dimensions() {
        let css = "font-size: 16px; margin: 1.5em; width: 50%;";
        let tokens = tokenize(css);

        let significant: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind != CssTokenKind::Whitespace && t.kind != CssTokenKind::Eof)
            .collect();

        // Tokens: font-size : 16px ; margin : 1.5em ; width : 50% ;
        //         [0]        [1] [2]  [3]  [4]    [5] [6]   [7] [8]  [9] [10] [11]

        // font-size: 16px;
        assert_eq!(significant[0].kind, CssTokenKind::Ident);
        assert_eq!(slice(css, significant[0]), "font-size");
        assert_eq!(significant[2].kind, CssTokenKind::Dimension);
        assert_eq!(slice(css, significant[2]), "16px");

        // margin: 1.5em;
        assert_eq!(significant[4].kind, CssTokenKind::Ident);
        assert_eq!(slice(css, significant[4]), "margin");
        assert_eq!(significant[6].kind, CssTokenKind::Dimension);
        assert_eq!(slice(css, significant[6]), "1.5em");

        // width: 50%;
        assert_eq!(significant[8].kind, CssTokenKind::Ident);
        assert_eq!(slice(css, significant[8]), "width");
        assert_eq!(significant[10].kind, CssTokenKind::Percentage);
        assert_eq!(slice(css, significant[10]), "50%");
    }

    #[test]
    fn test_string_tokens() {
        let css = r#"font-family: "Helvetica Neue";"#;
        let tokens = tokenize(css);

        let string_token = tokens
            .iter()
            .find(|t| t.kind == CssTokenKind::String)
            .expect("Expected a String token");

        assert_eq!(slice(css, string_token), "Helvetica Neue");
    }

    #[test]
    fn test_comments_are_skipped() {
        let css = "/* this is a comment */ h1 { color: red; }";
        let tokens = tokenize(css);

        // No comment token should be emitted
        assert!(!tokens.iter().any(|t| slice(css, t).contains("comment")));

        // First significant token should be 'h1'
        let first_ident = tokens
            .iter()
            .find(|t| t.kind == CssTokenKind::Ident)
            .unwrap();
        assert_eq!(slice(css, first_ident), "h1");
    }

    #[test]
    fn test_at_keyword() {
        let css = "@media screen { }";
        let tokens = tokenize(css);

        let at_token = tokens
            .iter()
            .find(|t| t.kind == CssTokenKind::AtKeyword)
            .expect("Expected an AtKeyword token");

        assert_eq!(slice(css, at_token), "@media");
    }

    #[test]
    fn test_function_token() {
        let css = "color: rgb(255, 0, 0);";
        let tokens = tokenize(css);

        let func_token = tokens
            .iter()
            .find(|t| t.kind == CssTokenKind::Function)
            .expect("Expected a Function token");

        assert_eq!(slice(css, func_token), "rgb");
    }

    #[test]
    fn test_universal_selector() {
        let css = "* { margin: 0; }";
        let tokens = tokenize(css);

        assert_eq!(tokens[0].kind, CssTokenKind::Delim);
        assert_eq!(slice(css, &tokens[0]), "*");
    }

    #[test]
    fn test_compound_selector() {
        let css = "div.main { }";
        let tokens = tokenize(css);

        let significant: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind != CssTokenKind::Whitespace && t.kind != CssTokenKind::Eof)
            .collect();

        // "div" "." "main" "{" "}"
        assert_eq!(significant[0].kind, CssTokenKind::Ident);
        assert_eq!(slice(css, significant[0]), "div");
        assert_eq!(significant[1].kind, CssTokenKind::Dot);
        assert_eq!(significant[2].kind, CssTokenKind::Ident);
        assert_eq!(slice(css, significant[2]), "main");
    }

    #[test]
    fn test_multiple_rules() {
        let css = "h1 { color: red; } p { color: blue; }";
        let tokens = tokenize(css);

        // Count open/close braces
        let open_braces = tokens
            .iter()
            .filter(|t| t.kind == CssTokenKind::OpenBrace)
            .count();
        let close_braces = tokens
            .iter()
            .filter(|t| t.kind == CssTokenKind::CloseBrace)
            .count();

        assert_eq!(open_braces, 2);
        assert_eq!(close_braces, 2);
    }

    #[test]
    fn test_hash_color() {
        let css = "color: #ff0000;";
        let tokens = tokenize(css);

        let hash_token = tokens
            .iter()
            .find(|t| t.kind == CssTokenKind::Hash)
            .expect("Expected a Hash token");

        assert_eq!(slice(css, hash_token), "#ff0000");
    }

    #[test]
    fn test_negative_number() {
        let css = "margin: -10px;";
        let tokens = tokenize(css);

        let dim_token = tokens
            .iter()
            .find(|t| t.kind == CssTokenKind::Dimension)
            .expect("Expected a Dimension token");

        assert_eq!(slice(css, dim_token), "-10px");
    }

    #[test]
    fn test_whitespace_collapsed() {
        let css = "h1   \t  \n  { }";
        let tokens = tokenize(css);

        // The whitespace between h1 and { should be a single Whitespace token
        assert_eq!(tokens[0].kind, CssTokenKind::Ident);
        assert_eq!(tokens[1].kind, CssTokenKind::Whitespace);
        assert_eq!(tokens[2].kind, CssTokenKind::OpenBrace);
    }
}
