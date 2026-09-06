use super::state::{AppState, Mode};
use super::types::Vector2D;
use crate::config::{ModeConfig, MotionConfig};

pub struct MotionEngine {
  config: MotionConfig,
  modes: ModeConfig,
}

impl MotionEngine {
  pub fn new(config: MotionConfig) -> Self {
    Self::with_modes(config, ModeConfig::default())
  }

  pub fn with_modes(config: MotionConfig, modes: ModeConfig) -> Self {
    Self { config, modes }
  }

  pub fn update_config(&mut self, config: MotionConfig) {
    self.config = config;
  }

  pub fn update_modes(&mut self, modes: ModeConfig) {
    self.modes = modes;
  }

  pub fn tick(&self, state: &AppState, delta_time: f64) -> (Vector2D, Vector2D) {
    if !state.active || state.emergency_stop || !delta_time.is_finite() || delta_time <= 0.0 {
      return (Vector2D::zero(), Vector2D::zero());
    }
    let input = state.input.get_input_vector();
    if input.magnitude() == 0.0 {
      // Integrate exponential decay, keeping stopping distance independent of frame rate.
      if self.config.friction <= 0.0 {
        return (Vector2D::zero(), Vector2D::zero());
      }
      let rate = -self.config.friction.ln() * 60.0;
      let decay = (-rate * delta_time).exp();
      let velocity = state.velocity.scale(decay);
      let delta = state.velocity.scale((1.0 - decay) / rate);
      return (
        if velocity.magnitude() < 0.01 {
          Vector2D::zero()
        } else {
          velocity
        },
        delta,
      );
    }
    let multiplier = match state.input.mode {
      Mode::Normal => self.modes.normal_multiplier,
      Mode::Precise => self.modes.precise_multiplier,
      Mode::Fast => self.modes.fast_multiplier,
    };
    let speed = self.config.max_speed * multiplier;
    let target = input.scale(speed);
    if self.config.acceleration >= 1.0 {
      return (target, target.scale(delta_time));
    }
    let rate = -(1.0 - self.config.acceleration).ln() * 60.0;
    let difference = Vector2D::new(state.velocity.x - target.x, state.velocity.y - target.y);
    let distance = difference.magnitude();
    if distance < f64::EPSILON {
      return (target, target.scale(delta_time));
    }
    let (remaining, integrated) = match self.config.curve_type.as_str() {
      "linear" => {
        let acceleration = speed * rate;
        let time = delta_time.min(distance / acceleration);
        let remaining = (distance - acceleration * delta_time).max(0.0) / distance;
        (remaining, time - acceleration * time * time / (2.0 * distance))
      }
      "sigmoid" => {
        // Logistic decay of distance to the target has an exact time-based integral.
        let normalized = distance / (distance + speed);
        let end = normalized * (-rate * delta_time).exp();
        let remaining = speed * end / ((1.0 - end) * distance);
        let integrated = speed * ((1.0 - end) / (1.0 - normalized)).ln() / (rate * distance);
        (remaining, integrated)
      }
      _ => {
        let decay = (-rate * delta_time).exp();
        (decay, (1.0 - decay) / rate)
      }
    };
    let velocity = target.add(&difference.scale(remaining));
    let delta = target.scale(delta_time).add(&difference.scale(integrated));
    (velocity, delta)
  }
}
