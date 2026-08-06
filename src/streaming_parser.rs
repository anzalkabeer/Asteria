use crate::dom::{Dom, NodeId};
use crate::parser::Parser;
use crate::tokenizer::Tokenizer;

pub struct StreamingHtmlProcessor {
    buffer: Vec<u8>,
    tokenizer: Tokenizer,
    parser: Parser,
    pub dom: Dom,
}

impl StreamingHtmlProcessor {
    pub fn new() -> Self {
        StreamingHtmlProcessor {
            buffer: Vec::new(),
            tokenizer: Tokenizer::new(),
            parser: Parser::new(),
            dom: Dom::new(),
        }
    }

    /// Process a new chunk of network bytes and return the IDs of newly dirtied subtrees
    pub fn receive_network_chunk(&mut self, chunk: &[u8], is_final: bool) -> Vec<NodeId> {
        self.buffer.extend_from_slice(chunk);
        let tokens = self.tokenizer.process_chunk(&self.buffer, is_final);
        self.parser.push_tokens(&mut self.dom, &tokens, &self.buffer)
    }

    /// Finalize parsing and extract the completed DOM
    pub fn finish(self) -> Dom {
        self.dom
    }
}
