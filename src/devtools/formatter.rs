// ─── ASTERIA Observability Framework: Formatter ─────────────────
//
// Converts internal snapshots and metrics into terminal ASCII reports.
// Does NOT run if telemetry is disabled.

use crate::devtools::snapshot::EngineSnapshot;
use crate::devtools::metrics::{EnergyDiagnostics, EnergyImpact, MEMORY_ALLOCATED, GPU_VRAM_USED};
use std::sync::atomic::Ordering;

pub fn format_memory_inspector() -> String {
    let allocs = MEMORY_ALLOCATED.load(Ordering::Relaxed) as f64 / 1024.0 / 1024.0;
    let vram = GPU_VRAM_USED.load(Ordering::Relaxed) as f64 / 1024.0 / 1024.0;

    let alloc_bar = create_progress_bar(allocs, 10.0);
    let vram_bar = create_progress_bar(vram, 20.0);

    format!(
        "── Memory Inspector ──────────────────────────────────\n\
         RAM Allocations  : [{}] {:.2} MB\n\
         GPU VRAM Used    : [{}] {:.2} MB\n\
         ──────────────────────────────────────────────────────",
        alloc_bar, allocs, vram_bar, vram
    )
}

pub fn format_energy_diagnostics(diagnostics: &EnergyDiagnostics) -> String {
    let impact_str = match diagnostics.impact {
        EnergyImpact::Low => "Low (Battery Friendly)",
        EnergyImpact::Medium => "Medium",
        EnergyImpact::High => "High (Battery Drain Risk)",
    };

    let copied_mb = diagnostics.bytes_copied as f64 / 1024.0 / 1024.0;

    format!(
        "── Energy Diagnostics ────────────────────────────────\n\
         Battery Risk     : {}\n\
         Thread Wakeups   : {}\n\
         Memory Copied    : {:.2} MB\n\
         GPU Uploads      : {}\n\
         Total Allocs     : {}\n\
         ──────────────────────────────────────────────────────",
        impact_str,
        diagnostics.thread_wakeups,
        copied_mb,
        diagnostics.gpu_uploads,
        diagnostics.allocations
    )
}

pub fn format_segment_inspector(snapshot: &EngineSnapshot) -> String {
    if let Some(segments) = snapshot.segments {
        let mut output = format!(
            "── Segment Inspector ({} tiles, {:.1}px each) ─────────\n",
            segments.len(),
            segments.segment_height
        );
        output.push_str("  ┌─────────┬──────────────────────────┬────────┬──────────┐\n");
        output.push_str("  │ Tile ID │ Y-Range                  │ Status │ Gen      │\n");
        output.push_str("  ├─────────┼──────────────────────────┼────────┼──────────┤\n");
        
        for seg in &segments.segments {
            let status = if seg.dirty { "DIRTY " } else { "CLEAN " };
            let row = format!(
                "  │ {:<7} │ {:>5.0} .. {:<5.0}        │ {} │ {:<8} │\n",
                seg.id,
                seg.rect.y,
                seg.rect.y + seg.rect.height,
                status,
                seg.generation
            );
            output.push_str(&row);
        }
        output.push_str("  └─────────┴──────────────────────────┴────────┴──────────┘\n");
        output
    } else {
        "── Segment Inspector: No Segment Data ────────────────".to_string()
    }
}

fn create_progress_bar(value: f64, max: f64) -> String {
    let percentage = (value / max).clamp(0.0, 1.0);
    let filled_blocks = (percentage * 20.0).round() as usize;
    let empty_blocks = 20 - filled_blocks;
    format!("{}{}", "█".repeat(filled_blocks), "░".repeat(empty_blocks))
}
