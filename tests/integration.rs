use kact::config::Config;
use kact::core::types::Vector2D;
use kact::core::{AppState, Direction, InputState, Mode, MotionEngine};

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

#[test]
fn strict_config_defaults_and_validation() {
  let defaults: Config = toml::from_str("").unwrap();
  defaults.validate().unwrap();
  assert!(!defaults.keybindings.enabled);
  assert!(defaults.keybindings.global.is_empty());
  assert!(toml::from_str::<Config>("[motion]\nmax_speeed = 20").is_err());
  for text in [
    "[motion]\ntarget_fps = 0",
    "[motion]\nmax_speed = nan",
    "[navigation]\nalphabet = 'aa'",
    "[appearance]\nbackground = '#GGFFFF'",
    "[modes]\nfast_multiplier = -1",
  ] {
    assert!(toml::from_str::<Config>(text).unwrap().validate().is_err(), "{text}");
  }
  toml::from_str::<Config>(include_str!("../kact.toml"))
    .unwrap()
    .validate()
    .unwrap();
}

#[test]
fn motion_is_time_based_and_uses_modes() {
  for curve in ["linear", "sigmoid", "exponential"] {
    let mut config = Config::default();
    config.motion.curve_type = curve.into();
    config.motion.acceleration = 0.05;
    config.modes.fast_multiplier = 3.0;
    let engine = MotionEngine::with_modes(config.motion, config.modes);
    let mut initial = AppState {
      active: true,
      ..Default::default()
    };
    initial.input.mode = kact::core::Mode::Fast;
    initial.input.press_direction(Direction::Right);
    let (whole_velocity, whole_distance) = engine.tick(&initial, 0.2);
    let mut split = initial.clone();
    let mut distance = Vector2D::zero();
    for _ in 0..20 {
      let (velocity, delta) = engine.tick(&split, 0.01);
      split.velocity = velocity;
      distance = distance.add(&delta);
    }
    assert!((whole_velocity.x - split.velocity.x).abs() < 1e-8, "{curve}");
    assert!((whole_distance.x - distance.x).abs() < 1e-8, "{curve}");
    split.input.active_directions.clear();
    let (whole_velocity, whole_distance) = engine.tick(&split, 0.2);
    distance = Vector2D::zero();
    for _ in 0..20 {
      let (velocity, delta) = engine.tick(&split, 0.01);
      split.velocity = velocity;
      distance = distance.add(&delta);
    }
    assert!((whole_velocity.x - split.velocity.x).abs() < 1e-8);
    assert!((whole_distance.x - distance.x).abs() < 1e-8);
  }
  let config = Config::default();
  let mut motion = config.motion;
  motion.acceleration = 1.0;
  let mut modes = config.modes;
  modes.fast_multiplier = 4.0;
  let engine = MotionEngine::with_modes(motion, modes);
  let mut state = AppState {
    active: true,
    ..Default::default()
  };
  state.input.mode = kact::core::Mode::Fast;
  state.input.press_direction(Direction::Right);
  state.input.press_direction(Direction::Down);
  assert!((engine.tick(&state, 0.1).0.magnitude() - 8000.0).abs() < 1e-8);
  state.emergency_stop = true;
  assert_eq!(engine.tick(&state, 0.1).1, Vector2D::zero());
}

#[test]
fn grid_labels_span_screens_and_filter_without_ambiguous_prefixes() {
  use kact::core::navigation::{Navigation, labels};
  use kact::desktop::Rect;
  let screens = [
    Rect {
      x: -100.0,
      y: 0.0,
      width: 100.0,
      height: 80.0,
    },
    Rect {
      x: 0.0,
      y: 0.0,
      width: 200.0,
      height: 100.0,
    },
  ];
  let mut navigation = Navigation::grid(&screens, 2, 2, "ab").unwrap();
  assert_eq!(navigation.targets.len(), 8);
  assert_eq!(navigation.targets[0].bounds.width, 50.0);
  assert_eq!(navigation.targets[4].bounds.width, 100.0);
  assert_eq!(navigation.targets[7].label, "bbb");
  assert!(navigation.type_char('b').is_none());
  assert_eq!(navigation.visible().len(), 4);
  navigation.type_char('z');
  assert_eq!(navigation.prefix, "b");
  navigation.backspace();
  assert!(navigation.prefix.is_empty());
  navigation.type_char('a');
  navigation.type_char('a');
  assert_eq!(navigation.type_char('a').unwrap().bounds.x, -100.0);
  let labels = labels(1000, "abc").unwrap();
  assert_eq!(labels.iter().collect::<std::collections::HashSet<_>>().len(), 1000);
  assert!(labels.iter().all(|label| label.len() == labels[0].len()));
  assert!(Navigation::grid(&screens, 0, 2, "ab").is_err());
}

#[test]
fn core_edge_cases_are_safe() {
  use kact::core::navigation::{Navigation, labels};
  use kact::desktop::Rect;

  let mut input = InputState::new();
  for direction in [Direction::Up, Direction::Down, Direction::Left, Direction::Right] {
    input.press_direction(direction);
  }
  input.release_direction(Direction::Down);
  input.set_mode(Mode::Precise);
  assert!(input.get_input_vector().magnitude() > 0.0);
  input.set_mode(Mode::Fast);

  let mut state = AppState::new();
  state.toggle_active();
  state.trigger_emergency_stop();
  assert!(state.emergency_stop && !state.active);

  let mut config = Config::default();
  config.motion.friction = 0.0;
  let mut engine = MotionEngine::new(config.motion.clone());
  engine.update_config(config.motion.clone());
  engine.update_modes(config.modes.clone());
  state.active = true;
  state.emergency_stop = false;
  state.velocity = Vector2D::new(0.005, 0.0);
  assert_eq!(engine.tick(&state, 0.1), (Vector2D::zero(), Vector2D::zero()));

  config.motion.friction = 0.95;
  engine.update_config(config.motion.clone());
  assert_eq!(engine.tick(&state, 0.1).0, Vector2D::zero());
  config.motion.acceleration = 0.5;
  let mut modes = config.modes;
  modes.precise_multiplier = 0.5;
  engine.update_config(config.motion.clone());
  engine.update_modes(modes);
  state.input = InputState::new();
  state.input.set_mode(Mode::Precise);
  state.input.press_direction(Direction::Right);
  state.velocity = Vector2D::new(config.motion.max_speed * 0.5, 0.0);
  assert_eq!(engine.tick(&state, 0.1).0, state.velocity);

  assert!(labels(100_001, "ab").is_err());
  assert!(labels(1, "a").is_err());
  assert!(
    Navigation::from_rects(
      &[Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 1.0
      }],
      "ab"
    )
    .is_err()
  );
  let mut navigation = Navigation::from_rects(
    &[Rect {
      x: 0.0,
      y: 0.0,
      width: 1.0,
      height: 1.0,
    }],
    "ab",
  )
  .unwrap();
  assert!(navigation.type_char('1').is_none());
}
