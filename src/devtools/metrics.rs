// ─── ASTERIA Observability Framework: Metrics & Diagnostics ─────
//
// Records numerical statistics (DOM nodes, layouts, VRAM).
// Estimates battery and energy impact of high-cost operations.

use crate::aof_guard;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyImpact {
    Low,
    Medium,
    High,
}

impl Default for EnergyImpact {
    fn default() -> Self {
        Self::Low
    }
}

#[derive(Debug, Clone, Default)]
pub struct EnergyDiagnostics {
    pub thread_wakeups: usize,
    pub bytes_copied: usize,
    pub gpu_uploads: usize,
    pub allocations: usize,
    pub impact: EnergyImpact,
}

impl EnergyDiagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.thread_wakeups = 0;
        self.bytes_copied = 0;
        self.gpu_uploads = 0;
        self.allocations = 0;
    }

    pub fn analyze_impact(&self) -> EnergyImpact {
        if self.allocations > 100 || self.gpu_uploads > 3 || self.thread_wakeups > 8 {
            EnergyImpact::High
        } else if self.allocations > 0 || self.gpu_uploads > 0 || self.thread_wakeups > 2 {
            EnergyImpact::Medium
        } else {
            EnergyImpact::Low
        }
    }
}

pub static MEMORY_ALLOCATED: AtomicUsize = AtomicUsize::new(0);
pub static GPU_VRAM_USED: AtomicUsize = AtomicUsize::new(0);

pub fn record_allocation(bytes: usize) {
    aof_guard!();
    MEMORY_ALLOCATED.fetch_add(bytes, Ordering::Relaxed);
}

pub fn record_gpu_upload(bytes: usize) {
    aof_guard!();
    GPU_VRAM_USED.fetch_add(bytes, Ordering::Relaxed);
}

pub fn reset_frame_metrics() {
    aof_guard!();
    MEMORY_ALLOCATED.store(0, Ordering::Relaxed);
    GPU_VRAM_USED.store(0, Ordering::Relaxed);
}
