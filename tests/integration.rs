use kact::config::Config;
use kact::core::types::Vector2D;
use kact::core::{AppState, Direction, MotionEngine};

#[test]
fn test_config_loading() {
  let config = Config::default();
  assert_eq!(config.motion.curve_type, "sigmoid");
  assert_eq!(config.motion.max_speed, 2000.0);
}

#[test]
fn test_motion_engine_basic() {
  let config = Config::default();
  let engine = MotionEngine::new(config.motion);

  let mut state = AppState::new();
  state.active = true;
  state.input.press_direction(Direction::Right);

  let delta_time = 1.0 / 60.0; // 60 FPS
  let (velocity, delta_pos) = engine.tick(&state, delta_time);

  assert!(velocity.x > 0.0);
  assert!(delta_pos.x > 0.0);
}

#[test]
fn test_vector2d_operations() {
  let v1 = Vector2D::new(3.0, 4.0);
  assert_eq!(v1.magnitude(), 5.0);

  let v2 = v1.normalize();
  assert!((v2.magnitude() - 1.0).abs() < 0.001);

  let v3 = v1.scale(2.0);
  assert_eq!(v3.x, 6.0);
  assert_eq!(v3.y, 8.0);
}
