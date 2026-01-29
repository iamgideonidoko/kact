use crate::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
  pub motion: MotionConfig,
  pub keybindings: KeyBindings,
  pub modes: ModeConfig,
  pub system: SystemConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MotionConfig {
  /// Acceleration curve type: "sigmoid", "exponential", "linear"
  pub curve_type: String,
  /// Maximum pixels per second
  pub max_speed: f64,
  /// Acceleration factor (0.0-1.0)
  pub acceleration: f64,
  /// Friction coefficient (0.0-1.0)
  pub friction: f64,
  /// Target frames per second for motion engine
  pub target_fps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
  pub move_up: String,
  pub move_down: String,
  pub move_left: String,
  pub move_right: String,
  pub mode_normal: String,
  pub mode_precise: String,
  pub mode_fast: String,
  pub toggle_active: String,
  pub emergency_stop: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeConfig {
  pub normal_multiplier: f64,
  pub precise_multiplier: f64,
  pub fast_multiplier: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
  /// Enable hot-reload of config file
  pub hot_reload: bool,
  /// Log level: "error", "warn", "info", "debug", "trace"
  pub log_level: String,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      motion: MotionConfig {
        curve_type: "sigmoid".to_string(),
        max_speed: 2000.0,
        acceleration: 0.8,
        friction: 0.95,
        target_fps: 144,
      },
      keybindings: KeyBindings {
        move_up: "w".to_string(),
        move_down: "s".to_string(),
        move_left: "a".to_string(),
        move_right: "d".to_string(),
        mode_normal: "1".to_string(),
        mode_precise: "2".to_string(),
        mode_fast: "3".to_string(),
        toggle_active: "space".to_string(),
        emergency_stop: "escape".to_string(),
      },
      modes: ModeConfig {
        normal_multiplier: 1.0,
        precise_multiplier: 0.3,
        fast_multiplier: 2.5,
      },
      system: SystemConfig {
        hot_reload: true,
        log_level: "info".to_string(),
      },
    }
  }
}

impl Config {
  pub fn load(path: &Path) -> Result<Self> {
    let contents = fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
  }

  pub fn load_or_default(path: &Path) -> Self {
    Self::load(path).unwrap_or_else(|e| {
      eprintln!("Failed to load config: {}. Using defaults.", e);
      Self::default()
    })
  }

  pub fn default_path() -> PathBuf {
    PathBuf::from("kact.toml")
  }
}
