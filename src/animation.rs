// ─── Keyframe & Transition Animation Engine ──────────────────────────────────────────────

use crate::dom::NodeId;
use crate::properties::PropertyId;
use crate::values::{AnimationTimingFunction, Color};

/// An actively progressing transition on a CSS property of a DOM node.
#[derive(Debug, Clone)]
pub struct ActiveTransition {
    pub node_id: NodeId,
    pub property: PropertyId,
    pub start_value: f32,
    pub end_value: f32,
    pub duration: f32,
    pub elapsed: f32,
    pub timing_function: AnimationTimingFunction,
}

impl ActiveTransition {
    pub fn new(
        node_id: NodeId,
        property: PropertyId,
        start_value: f32,
        end_value: f32,
        duration: f32,
        timing_function: AnimationTimingFunction,
    ) -> Self {
        Self {
            node_id,
            property,
            start_value,
            end_value,
            duration: duration.max(0.0001),
            elapsed: 0.0,
            timing_function,
        }
    }

    /// Advance transition by dt (in seconds) and compute interpolated value.
    pub fn step(&mut self, dt: f32) -> (f32, bool) {
        self.elapsed += dt;
        let progress = (self.elapsed / self.duration).clamp(0.0, 1.0);
        let eased = ease(progress, &self.timing_function);
        let current_val = lerp(self.start_value, self.end_value, eased);
        let is_finished = self.elapsed >= self.duration;
        (current_val, is_finished)
    }
}

/// An actively running keyframe animation.
#[derive(Debug, Clone)]
pub struct ActiveAnimation {
    pub node_id: NodeId,
    pub name: String,
    pub duration: f32,
    pub elapsed: f32,
    pub iteration_count: f32,
    pub timing_function: AnimationTimingFunction,
}

/// Central animation manager maintaining active transitions and keyframe timelines.
#[derive(Debug, Default)]
pub struct AnimationManager {
    pub transitions: Vec<ActiveTransition>,
    pub animations: Vec<ActiveAnimation>,
}

impl AnimationManager {
    pub fn new() -> Self {
        Self {
            transitions: Vec::new(),
            animations: Vec::new(),
        }
    }

    /// Register a new property transition. Replaces any existing transition on the same node & property.
    pub fn start_transition(&mut self, transition: ActiveTransition) {
        self.transitions
            .retain(|t| !(t.node_id == transition.node_id && t.property == transition.property));
        self.transitions.push(transition);
    }

    /// Progress all active animations and transitions by `dt` seconds.
    /// Returns a list of `(NodeId, PropertyId, f32)` representing updated property values.
    pub fn tick(&mut self, dt: f32) -> Vec<(NodeId, PropertyId, f32)> {
        let mut updates = Vec::new();

        self.transitions.retain_mut(|trans| {
            let (val, is_finished) = trans.step(dt);
            updates.push((trans.node_id, trans.property, val));
            !is_finished
        });

        updates
    }

    /// Number of active transitions running.
    pub fn active_transition_count(&self) -> usize {
        self.transitions.len()
    }

    /// Clears all active animations and transitions.
    pub fn clear(&mut self) {
        self.transitions.clear();
        self.animations.clear();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 100.0, 0.5), 50.0);
        assert_eq!(lerp(10.0, 20.0, 0.0), 10.0);
        assert_eq!(lerp(10.0, 20.0, 1.0), 20.0);
    }

    #[test]
    fn test_active_transition_progression() {
        let mut manager = AnimationManager::new();
        let trans = ActiveTransition::new(
            NodeId(1),
            PropertyId::Width,
            100.0,
            200.0,
            1.0,
            AnimationTimingFunction::Linear,
        );
        manager.start_transition(trans);
        assert_eq!(manager.active_transition_count(), 1);

        // Step half a second
        let updates = manager.tick(0.5);
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].0, NodeId(1));
        assert_eq!(updates[0].1, PropertyId::Width);
        assert!((updates[0].2 - 150.0).abs() < 1e-4);
        assert_eq!(manager.active_transition_count(), 1);

        // Step remaining time to complete
        let updates2 = manager.tick(0.5);
        assert_eq!(updates2.len(), 1);
        assert!((updates2[0].2 - 200.0).abs() < 1e-4);
        assert_eq!(manager.active_transition_count(), 0);
    }
}
