use asteria::devtools::config::{
    AOF_ENABLED, AofConfig, EXPORT_ENABLED, METRICS_ENABLED, SNAPSHOT_ENABLED, TRACE_ENABLED,
};
use asteria::devtools::export::export_chrome_trace;
use asteria::devtools::trace::{TraceEventKind, record_event, trace_recorder};
use std::fs;
use std::sync::atomic::Ordering;
use std::sync::{Mutex, OnceLock};

static TEST_MUTEX: OnceLock<Mutex<()>> = OnceLock::new();

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_MUTEX.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn test_observability_tracing_disabled_by_default() {
    let _guard = test_lock();
    let mut recorder = trace_recorder().lock().unwrap();
    recorder.clear();
    drop(recorder);

    AOF_ENABLED.store(false, Ordering::SeqCst);
    TRACE_ENABLED.store(false, Ordering::SeqCst);
    SNAPSHOT_ENABLED.store(false, Ordering::SeqCst);
    METRICS_ENABLED.store(false, Ordering::SeqCst);
    EXPORT_ENABLED.store(false, Ordering::SeqCst);
    record_event(TraceEventKind::ParseStart);

    let recorder = trace_recorder().lock().unwrap();
    assert_eq!(recorder.events.len(), 0);
}

#[test]
fn test_observability_trace_recording_and_export() {
    let _guard = test_lock();
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

    let _ = fs::remove_file(test_file);
}

#[test]
fn test_mixed_observability_config_respects_subsystem_flags() {
    let _guard = test_lock();
    let config = AofConfig {
        enabled: true,
        enable_tracing: false,
        enable_snapshots: true,
        enable_metrics: false,
        enable_export: true,
    };
    config.apply();

    assert!(AOF_ENABLED.load(Ordering::SeqCst));
    assert!(!TRACE_ENABLED.load(Ordering::SeqCst));
    assert!(SNAPSHOT_ENABLED.load(Ordering::SeqCst));
    assert!(!METRICS_ENABLED.load(Ordering::SeqCst));
    assert!(EXPORT_ENABLED.load(Ordering::SeqCst));
}

#[test]
fn test_trace_recorder_assigns_distinct_thread_ids() {
    let _guard = test_lock();
    AofConfig::full_inspection().apply();

    let mut recorder = trace_recorder().lock().unwrap();
    recorder.clear();
    drop(recorder);

    let mut handles = Vec::new();
    for _ in 0..2 {
        handles.push(std::thread::spawn(|| {
            record_event(TraceEventKind::ParseStart);
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let recorder = trace_recorder().lock().unwrap();
    let thread_ids: Vec<u32> = recorder
        .events
        .iter()
        .map(|event| event.thread_id)
        .collect();
    assert_eq!(thread_ids.len(), 2);
    assert_ne!(thread_ids[0], thread_ids[1]);
}
