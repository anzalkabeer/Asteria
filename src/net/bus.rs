use std::sync::mpsc;

/// Events emitted during an asynchronous streaming fetch.
#[derive(Debug, Clone)]
pub enum ResourceBusEvent {
    /// Header metadata received.
    HeaderReceived {
        url: String,
        status_code: u16,
        content_length: Option<usize>,
        content_type: Option<String>,
    },
    /// A chunk of the response body was received.
    ChunkReceived {
        url: String,
        chunk_data: Vec<u8>,
        offset: usize,
        is_final: bool,
    },
    /// Fetch failed.
    FetchError { url: String, message: String },
}

/// A bus for streaming network responses directly into the parsing pipeline.
pub struct StreamingResourceBus {
    sender: mpsc::Sender<ResourceBusEvent>,
    receiver: mpsc::Receiver<ResourceBusEvent>,
}

impl StreamingResourceBus {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self { sender, receiver }
    }

    /// Cloneable sender for worker threads.
    pub fn sender(&self) -> mpsc::Sender<ResourceBusEvent> {
        self.sender.clone()
    }

    /// Try to receive the next available event without blocking.
    pub fn try_recv(&self) -> Option<ResourceBusEvent> {
        self.receiver.try_recv().ok()
    }
}

impl Default for StreamingResourceBus {
    fn default() -> Self {
        Self::new()
    }
}
