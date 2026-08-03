use asteria::devtools::config::AofConfig;
use asteria::devtools::metrics::{
    EnergyDiagnostics, EnergyImpact, MEMORY_ALLOCATED, record_allocation, reset_frame_metrics,
};
use asteria::devtools::snapshot::EngineSnapshot;
use std::sync::atomic::Ordering;

#[test]
fn test_energy_diagnostics_impact_levels() {
    let mut diag = EnergyDiagnostics::new();

    // Low Impact
    assert_eq!(diag.analyze_impact(), EnergyImpact::Low);

    // Medium Impact
    diag.allocations = 5;
    assert_eq!(diag.analyze_impact(), EnergyImpact::Medium);

    // High Impact
    diag.thread_wakeups = 10;
    assert_eq!(diag.analyze_impact(), EnergyImpact::High);
}

#[test]
fn test_memory_allocation_tracking() {
    AofConfig::full_inspection().apply();
    reset_frame_metrics();

    record_allocation(1024);
    record_allocation(512);

    let total = MEMORY_ALLOCATED.load(Ordering::Relaxed);
    assert_eq!(total, 1536);
}

#[test]
fn test_snapshot_creation() {
    let snapshot = EngineSnapshot::new();
    assert!(snapshot.dom.is_none());
    assert!(snapshot.layout.is_none());
}
