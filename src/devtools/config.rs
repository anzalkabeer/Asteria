// ─── ASTERIA Observability Framework: Configuration ────────────
//
// Runtime switches for telemetry. When tracing is disabled, the engine
// bypasses all metric collection and trace generation to ensure
// zero-overhead operation.

use std::sync::atomic::{AtomicBool, Ordering};

/// Global flag for the ASTERIA Observability Framework.
/// If this is false, NO events, metrics, or snapshots are collected.
pub static AOF_ENABLED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AofConfig {
    /// Master switch for all observability
    pub enabled: bool,
    /// Record timeline events (Start/End phases)
    pub enable_tracing: bool,
    /// Collect memory and layout snapshots
    pub enable_snapshots: bool,
    /// Collect energy and allocation metrics
    pub enable_metrics: bool,
    /// Export traces to JSON
    pub enable_export: bool,
}

impl Default for AofConfig {
    fn default() -> Self {
        AofConfig {
            enabled: false,
            enable_tracing: false,
            enable_snapshots: false,
            enable_metrics: false,
            enable_export: false,
        }
    }
}

impl AofConfig {
    /// Create a configuration with everything enabled for full inspection
    pub fn full_inspection() -> Self {
        AofConfig {
            enabled: true,
            enable_tracing: true,
            enable_snapshots: true,
            enable_metrics: true,
            enable_export: true,
        }
    }

    /// Apply the configuration to the global state
    pub fn apply(&self) {
        AOF_ENABLED.store(self.enabled, Ordering::SeqCst);
    }
}

/// Helper macro to early-exit from telemetry functions if AOF is disabled.
/// This guarantees zero-overhead when the framework is off.
#[macro_export]
macro_rules! aof_guard {
    () => {
        if !$crate::devtools::config::AOF_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }
    };
}
