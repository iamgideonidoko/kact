use super::state::{AppState, Mode};
use super::types::Vector2D;
use crate::config::MotionConfig;

pub struct MotionEngine {
  config: MotionConfig,
}

impl MotionEngine {
  pub fn new(config: MotionConfig) -> Self {
    Self { config }
  }

  pub fn update_config(&mut self, config: MotionConfig) {
    self.config = config;
  }

  /// Pure function: (State, DeltaTime) -> (NewVelocity, DeltaPosition)
  pub fn tick(&self, state: &AppState, delta_time: f64) -> (Vector2D, Vector2D) {
    if !state.active || state.emergency_stop {
      return (Vector2D::zero(), Vector2D::zero());
    }

    let input_vector = state.input.get_input_vector();
    let mode_multiplier = self.get_mode_multiplier(state.input.mode);

    // Calculate target velocity based on input
    let target_velocity = if input_vector.magnitude() > 0.0 {
      let curve_factor = self.apply_curve(input_vector.magnitude());
      input_vector.scale(self.config.max_speed * curve_factor * mode_multiplier)
    } else {
      Vector2D::zero()
    };

    // Interpolate towards target velocity (acceleration)
    let new_velocity = self.lerp_velocity(&state.velocity, &target_velocity, delta_time);

    // Apply friction when no input
    let new_velocity = if input_vector.magnitude() == 0.0 {
      new_velocity.scale(self.config.friction)
    } else {
      new_velocity
    };

    // Calculate position delta
    let delta_position = new_velocity.scale(delta_time);

    (new_velocity, delta_position)
  }

  fn get_mode_multiplier(&self, mode: Mode) -> f64 {
    // Note: These should come from config.modes in a real implementation
    // For now, using hardcoded values
    match mode {
      Mode::Normal => 1.0,
      Mode::Precise => 0.3,
      Mode::Fast => 2.5,
    }
  }

  fn apply_curve(&self, input_magnitude: f64) -> f64 {
    match self.config.curve_type.as_str() {
      "sigmoid" => self.sigmoid_curve(input_magnitude),
      "exponential" => self.exponential_curve(input_magnitude),
      "linear" => input_magnitude,
      _ => input_magnitude,
    }
  }

  fn sigmoid_curve(&self, x: f64) -> f64 {
    // Sigmoid: 1 / (1 + e^(-k(x - 0.5)))
    // Maps [0, 1] to smooth S-curve
    let k = 10.0 * self.config.acceleration;
    1.0 / (1.0 + (-k * (x - 0.5)).exp())
  }

  fn exponential_curve(&self, x: f64) -> f64 {
    // Exponential: x^p where p controls steepness
    let power = 1.0 + (1.0 - self.config.acceleration) * 2.0;
    x.powf(power)
  }

  fn lerp_velocity(&self, current: &Vector2D, target: &Vector2D, delta_time: f64) -> Vector2D {
    // Smooth interpolation with acceleration factor
    let t = 1.0 - (1.0 - self.config.acceleration).powf(delta_time * 60.0);
    Vector2D::new(
      current.x + (target.x - current.x) * t,
      current.y + (target.y - current.y) * t,
    )
  }
}
