use std::time::Instant;

#[derive(Debug, Clone, Copy)]
pub struct FrameBudget {
    pub target_ms: f64,
    pub input_ms: f64,
    pub style_ms: f64,
    pub layout_ms: f64,
    pub paint_ms: f64,
    pub gpu_upload_ms: f64,
    pub present_ms: f64,
}

impl FrameBudget {
    /// Pre-configured budget for 60Hz displays (16.67ms)
    pub fn new_60hz() -> Self {
        Self::new(16.67)
    }

    /// Define a custom target millisecond budget per frame
    pub fn new(target_ms: f64) -> Self {
        Self {
            target_ms,
            input_ms: 0.0,
            style_ms: 0.0,
            layout_ms: 0.0,
            paint_ms: 0.0,
            gpu_upload_ms: 0.0,
            present_ms: 0.0,
        }
    }

    /// The total ms spent in tracked stages so far in the current frame
    pub fn total_used(&self) -> f64 {
        self.input_ms 
            + self.style_ms 
            + self.layout_ms 
            + self.paint_ms 
            + self.gpu_upload_ms 
            + self.present_ms
    }

    /// Unused idle time remaining for the frame
    pub fn remaining(&self) -> f64 {
        self.target_ms - self.total_used()
    }

    /// Returns true if the frame processing exceeded the target budget
    pub fn is_over_budget(&self) -> bool {
        self.total_used() > self.target_ms
    }

    /// Record processing duration to the specified engine stage
    pub fn record_stage(&mut self, stage_name: &str, duration_ms: f64) {
        match stage_name {
            "input" => self.input_ms += duration_ms,
            "style" => self.style_ms += duration_ms,
            "layout" => self.layout_ms += duration_ms,
            "paint" => self.paint_ms += duration_ms,
            "gpu_upload" => self.gpu_upload_ms += duration_ms,
            "present" => self.present_ms += duration_ms,
            _ => {} 
        }
    }

    /// Zero out all timing data at the start of a new frame
    pub fn reset(&mut self) {
        self.input_ms = 0.0;
        self.style_ms = 0.0;
        self.layout_ms = 0.0;
        self.paint_ms = 0.0;
        self.gpu_upload_ms = 0.0;
        self.present_ms = 0.0;
    }
}

/// Simple convenience wrapper for measuring elapsed frame/stage time
#[derive(Debug, Clone, Copy)]
pub struct FrameTimer {
    start: Instant,
}

impl FrameTimer {
    /// Start measuring from right now
    pub fn start() -> Self {
        Self {
            start: Instant::now(),
        }
    }

    /// Calculate floating-point milliseconds elapsed since timer started
    pub fn elapsed_ms(&self) -> f64 {
        self.start.elapsed().as_secs_f64() * 1000.0
    }
}
