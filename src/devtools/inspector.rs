// ─── ASTERIA Observability Framework: Inspector ─────────────────
//
// The central coordinator for the Observability Framework.
// Collects traces, snapshots, and metrics, then outputs them
// via the requested exporters and formatters.

use crate::devtools::config::{AOF_ENABLED, AofConfig, EXPORT_ENABLED};
use crate::devtools::export::export_chrome_trace;
use crate::devtools::formatter::{
    format_energy_diagnostics, format_memory_inspector, format_segment_inspector,
};
use crate::devtools::metrics::EnergyDiagnostics;
use crate::devtools::snapshot::EngineSnapshot;
use std::sync::atomic::Ordering;

pub struct AofInspector;

impl AofInspector {
    /// Initialize the framework with the given configuration
    pub fn init(config: AofConfig) {
        config.apply();
        if config.enabled {
            println!("── ASTERIA Observability Framework (AOF) Initialized ──");
            if config.enable_tracing {
                println!("  [x] Event Tracing Enabled");
            }
            if config.enable_snapshots {
                println!("  [x] Snapshots Enabled");
            }
            if config.enable_metrics {
                println!("  [x] Metrics & Energy Diagnostics Enabled");
            }
            if config.enable_export {
                println!("  [x] Chrome Trace JSON Export Enabled");
            }
            println!("───────────────────────────────────────────────────────\n");
        }
    }

    /// Run the final inspection output step (Terminal UI & Trace JSON Export)
    pub fn inspect(
        snapshot: EngineSnapshot,
        energy_diag: &EnergyDiagnostics,
        trace_filename: &str,
    ) {
        if !AOF_ENABLED.load(Ordering::Relaxed) {
            return;
        }

        println!("\n{}", format_memory_inspector());
        println!("\n{}", format_energy_diagnostics(energy_diag));
        println!("\n{}", format_segment_inspector(&snapshot));

        // Export Chrome Trace only when export is enabled.
        if EXPORT_ENABLED.load(Ordering::Relaxed) {
            match export_chrome_trace(trace_filename) {
                Ok(_) => {
                    let recorder = crate::devtools::trace::trace_recorder().lock().unwrap();
                    if !recorder.events.is_empty() {
                        println!("\n[AOF] Chrome Trace exported to '{}'.", trace_filename);
                        println!(
                            "      Drag and drop into chrome://tracing or https://ui.perfetto.dev/ to view."
                        );
                    }
                }
                Err(e) => println!("[AOF] Failed to export trace: {}", e),
            }
        }
    }
}
