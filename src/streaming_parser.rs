use crate::dom::{Dom, NodeId};
use crate::parser::Parser;
use crate::tokenizer::Tokenizer;

pub struct StreamingHtmlProcessor {
    buffer: Vec<u8>,
    tokenizer: Tokenizer,
    parser: Parser,
    pub dom: Dom,
}

impl Default for StreamingHtmlProcessor {
    fn default() -> Self {
        Self::new()
    }
}f

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
    /// Maximum document size: 64MB
    const MAX_DOCUMENT_SIZE: usize = 64 * 1024 * 1024;

    pub fn receive_network_chunk(&mut self, chunk: &[u8], is_final: bool) -> Vec<NodeId> {
        if self.buffer.len() + chunk.len() > Self::MAX_DOCUMENT_SIZE {
            eprintln!(
                "Warning: Document exceeds {}MB limit, truncating",
                Self::MAX_DOCUMENT_SIZE / (1024 * 1024)
            );
            return Vec::new();
        }
        self.buffer.extend_from_slice(chunk);
        let tokens = self.tokenizer.process_chunk(&self.buffer, is_final);
        self.parser
            .push_tokens(&mut self.dom, &tokens, &self.buffer)
    }

    /// Finalize parsing and extract the completed DOM
    pub fn finish(self) -> Dom {
        self.dom
    }
}
