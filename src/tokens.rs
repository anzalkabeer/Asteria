#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TokenKind {
    Doctype,
    StartTag,
    EndTag,
    SelfClosingTag,
    Text,
    Comment,
    Eof, // state will change until state = EOF , then the loop will be stopped and the tokenizer will return the token with kind = EOF
}

#[derive(Debug, Clone)]
pub struct Attribute {
    pub name_start: u32,
    pub name_end: u32,
    pub value_start: u32,
    pub value_end: u32,
}

#[derive(Debug)]
pub struct Token {
    pub kind: TokenKind,
    pub start: u32,
    pub end: u32,
    pub attributes: Vec<Attribute>, // only populated for tags
}
