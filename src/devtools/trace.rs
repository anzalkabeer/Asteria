// ─── ASTERIA Observability Framework: Trace Events ──────────────
//
// Records timeline events without any formatting.
// Used to reconstruct the exact execution flow of the engine.

use crate::aof_guard;
use crate::devtools::config::TRACE_ENABLED;
use std::cell::Cell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::Instant;

thread_local! {
    static THREAD_ID: Cell<u32> = const { Cell::new(0) };
}

static NEXT_THREAD_ID: AtomicU32 = AtomicU32::new(1);

/// Global trace recorder
pub fn trace_recorder() -> &'static Mutex<TraceRecorder> {
    static RECORDER: OnceLock<Mutex<TraceRecorder>> = OnceLock::new();
    RECORDER.get_or_init(|| Mutex::new(TraceRecorder::new()))
}

#[derive(Debug, Clone)]
pub enum TraceEventKind {
    FrameBegin {
        frame_id: u64,
    },
    FrameEnd {
        frame_id: u64,
        duration_ms: f64,
    },
    ParseStart,
    ParseEnd {
        node_count: usize,
        duration_ms: f64,
    },
    StyleStart,
    StyleEnd {
        styled_count: usize,
        duration_ms: f64,
    },
    LayoutStart,
    LayoutEnd {
        box_count: usize,
        duration_ms: f64,
    },
    PaintStart,
    PaintEnd {
        command_count: usize,
        duration_ms: f64,
    },
    SceneStart,
    SceneEnd {
        node_count: usize,
        duration_ms: f64,
    },
    SegmentInvalidated {
        segment_id: u16,
        reason: String,
    },
    ImageDecoded {
        image_id: String,
        duration_ms: f64,
        bytes: usize,
    },
    GpuUpload {
        bytes: usize,
        duration_ms: f64,
    },
}

#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub timestamp_us: u64,
    pub thread_id: u32,
    pub kind: TraceEventKind,
}

pub struct TraceRecorder {
    pub events: Vec<TraceEvent>,
    pub session_start: Instant,
    pub max_events: usize,
    pub dropped_events: usize,
}

impl Default for TraceRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceRecorder {
    pub fn new() -> Self {
        TraceRecorder {
            events: Vec::with_capacity(1000),
            session_start: Instant::now(),
            max_events: 2048,
            dropped_events: 0,
        }
    }

    pub fn record(&mut self, kind: TraceEventKind) {
        if !TRACE_ENABLED.load(Ordering::Relaxed) {
            return;
        }

        let thread_id = THREAD_ID.with(|cell| {
            let current = cell.get();
            if current == 0 {
                let assigned = NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed);
                cell.set(assigned);
                assigned
            } else {
                current
            }
        });

        if self.events.len() >= self.max_events {
            self.events.remove(0);
            self.dropped_events += 1;
        }

        self.events.push(TraceEvent {
            timestamp_us: self.session_start.elapsed().as_micros() as u64,
            thread_id,
            kind,
        });
    }

    pub fn clear(&mut self) {
        self.events.clear();
        self.dropped_events = 0;
        self.session_start = Instant::now();
    }
}

/// Helper function to push an event to the global recorder
pub fn record_event(kind: TraceEventKind) {
    aof_guard!();
    if let Ok(mut recorder) = trace_recorder().lock() {
        recorder.record(kind);
    }
}
