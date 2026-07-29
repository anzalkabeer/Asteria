// ─── CSS Token Types ─────────────────────────────────────────────
// Each token stores offsets into the original CSS source buffer — zero-copy,
// just like the HTML tokenizer.
//
// The CSS tokenizer produces a Vec<CssToken> that the CSS parser consumes.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CssTokenKind {
    /// An identifier: tag names, property names, keyword values
    /// e.g. "color", "div", "red", "bold"
    Ident,

    /// A hash token: '#' followed by an identifier
    /// e.g. "#container", "#f0f0f0"
    /// start..end includes the '#' character
    Hash,

    /// A quoted string: "..." or '...'
    /// start..end is the content INSIDE the quotes (not including quotes)
    String,

    /// A plain number: "16", "1.5", "0"
    Number,

    /// A number with a unit suffix: "16px", "1.5em", "100vh"
    /// start..end covers the entire dimension including the unit
    Dimension,

    /// A number followed by '%': "50%", "100%"
    Percentage,

    /// The ':' character (separates property from value)
    Colon,

    /// The ';' character (ends a declaration)
    Semicolon,

    /// The ',' character (separates selectors in a group)
    Comma,

    /// The '{' character (opens a declaration block)
    OpenBrace,

    /// The '}' character (closes a declaration block)
    CloseBrace,

    /// The '.' character (class selector prefix)
    Dot,

    /// A run of whitespace (spaces, tabs, newlines)
    /// Collapsed into a single token regardless of length
    Whitespace,

    /// A single character that doesn't match any other token type
    /// e.g. '*' (universal selector), '>' (child combinator)
    Delim,

    /// An '@' followed by an identifier: @media, @import, @keyframes
    /// start..end covers from '@' through the keyword
    AtKeyword,

    /// An identifier immediately followed by '('
    /// e.g. "rgb(", "url("
    /// start..end covers the identifier part (not the parenthesis)
    Function,

    /// The '(' character
    OpenParen,

    /// The ')' character
    CloseParen,

    /// End of input
    Eof,
}

/// A single CSS token — points back into the source buffer via offsets.
/// No string allocations needed to represent the token.
#[derive(Debug)]
pub struct CssToken {
    pub kind: CssTokenKind,
    pub start: u32,
    pub end: u32,
}
