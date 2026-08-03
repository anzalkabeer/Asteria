use asteria::devtools::config::{AOF_ENABLED, AofConfig};
use asteria::devtools::export::export_chrome_trace;
use asteria::devtools::trace::{TraceEventKind, record_event, trace_recorder};
use std::fs;
use std::sync::atomic::Ordering;

#[test]
fn test_observability_tracing_disabled_by_default() {
    let mut recorder = trace_recorder().lock().unwrap();
    recorder.clear();
    drop(recorder);

    // Should do nothing because AOF_ENABLED is false
    AOF_ENABLED.store(false, Ordering::SeqCst);
    record_event(TraceEventKind::ParseStart);

    let recorder = trace_recorder().lock().unwrap();
    assert_eq!(recorder.events.len(), 0);
}

#[test]
fn test_observability_trace_recording_and_export() {
    AofConfig::full_inspection().apply();

    let mut recorder = trace_recorder().lock().unwrap();
    recorder.clear();
    drop(recorder);

    record_event(TraceEventKind::FrameBegin { frame_id: 100 });
    record_event(TraceEventKind::LayoutStart);
    record_event(TraceEventKind::LayoutEnd {
        box_count: 50,
        duration_ms: 2.5,
    });
    record_event(TraceEventKind::FrameEnd {
        frame_id: 100,
        duration_ms: 3.0,
    });

    let recorder = trace_recorder().lock().unwrap();
    assert_eq!(recorder.events.len(), 4);
    drop(recorder);

    let test_file = "test_trace.json";
    export_chrome_trace(test_file).expect("Failed to export trace");

    let json_content = fs::read_to_string(test_file).expect("Failed to read JSON");
    assert!(json_content.contains("\"name\": \"Frame\", \"cat\": \"engine\", \"ph\": \"B\""));
    assert!(json_content.contains("\"name\": \"Layout\", \"cat\": \"engine\", \"ph\": \"B\""));

    // Cleanup
    let _ = fs::remove_file(test_file);
}
