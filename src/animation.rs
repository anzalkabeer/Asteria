// ─── Keyframe Animation Engine ──────────────────────────────────────────────

use crate::values::{AnimationTimingFunction, Color};

pub struct AnimationManager {
    // state for active animations
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl AnimationManager {
    pub fn new() -> Self {
        AnimationManager {}
    }

    // Stub for window event integration
    pub fn tick(&mut self, _dt: f32) {
        // Handle animation progression
    }
}

pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

pub fn lerp_color(start: Color, end: Color, t: f32) -> Color {
    Color::new(
        lerp(start.r as f32, end.r as f32, t) as u8,
        lerp(start.g as f32, end.g as f32, t) as u8,
        lerp(start.b as f32, end.b as f32, t) as u8,
        lerp(start.a as f32, end.a as f32, t) as u8,
    )
}

pub fn ease(t: f32, function: &AnimationTimingFunction) -> f32 {
    match function {
        AnimationTimingFunction::Linear => t,
        AnimationTimingFunction::Ease => {
            // Cubic bezier approximation for ease
            t * t * (3.0 - 2.0 * t)
        }
        AnimationTimingFunction::EaseIn => t * t,
        AnimationTimingFunction::EaseOut => t * (2.0 - t),
        AnimationTimingFunction::EaseInOut => {
            if t < 0.5 {
                2.0 * t * t
            } else {
                -1.0 + (4.0 - 2.0 * t) * t
            }
        }
    }
}
