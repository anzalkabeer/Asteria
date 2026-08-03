// ─── ASTERIA Observability Framework: Exporters ─────────────────
//
// Exports AOF traces to Chrome Trace Event Format (JSON), which can be
// loaded into `chrome://tracing` or `https://ui.perfetto.dev/`.

use crate::devtools::trace::{TraceEventKind, trace_recorder};
use std::fs::File;
use std::io::Write;

pub fn export_chrome_trace(filename: &str) -> std::io::Result<()> {
    let recorder = trace_recorder().lock().unwrap();
    if recorder.events.is_empty() {
        return Ok(()); // Nothing to export
    }

    let mut file = File::create(filename)?;
    writeln!(file, "[")?;

    for (i, event) in recorder.events.iter().enumerate() {
        let (name, ph) = match &event.kind {
            TraceEventKind::FrameBegin { .. } => ("Frame", "B"),
            TraceEventKind::FrameEnd { .. } => ("Frame", "E"),
            TraceEventKind::ParseStart => ("Parse", "B"),
            TraceEventKind::ParseEnd { .. } => ("Parse", "E"),
            TraceEventKind::StyleStart => ("Style", "B"),
            TraceEventKind::StyleEnd { .. } => ("Style", "E"),
            TraceEventKind::LayoutStart => ("Layout", "B"),
            TraceEventKind::LayoutEnd { .. } => ("Layout", "E"),
            TraceEventKind::PaintStart => ("Paint", "B"),
            TraceEventKind::PaintEnd { .. } => ("Paint", "E"),
            TraceEventKind::SceneStart => ("SceneGraph", "B"),
            TraceEventKind::SceneEnd { .. } => ("SceneGraph", "E"),
            TraceEventKind::SegmentInvalidated { .. } => ("SegmentInvalidation", "i"), // Instant event
            TraceEventKind::ImageDecoded { .. } => ("ImageDecode", "i"),
            TraceEventKind::GpuUpload { .. } => ("GpuUpload", "i"),
        };

        // Standard Chrome Trace format fields
        // name: event name
        // cat: category (we use 'engine')
        // ph: phase (B=begin, E=end, i=instant)
        // ts: timestamp in microseconds
        // pid: process ID (mocked as 1)
        // tid: thread ID
        let json_line = format!(
            r#"  {{"name": "{}", "cat": "engine", "ph": "{}", "ts": {}, "pid": 1, "tid": {}}}"#,
            name, ph, event.timestamp_us, event.thread_id
        );

        if i < recorder.events.len() - 1 {
            writeln!(file, "{},", json_line)?;
        } else {
            writeln!(file, "{}", json_line)?;
        }
    }

    writeln!(file, "]")?;
    Ok(())
}
