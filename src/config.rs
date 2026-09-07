use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
  pub motion: MotionConfig,
  pub keybindings: KeyBindings,
  pub modes: ModeConfig,
  pub system: SystemConfig,
  pub navigation: NavigationConfig,
  pub appearance: AppearanceConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct MotionConfig {
  pub curve_type: String,
  pub max_speed: f64,
  pub acceleration: f64,
  /// Velocity retained per 1/60 second after input stops.
  pub friction: f64,
  pub target_fps: u32,
}
impl Default for MotionConfig {
  fn default() -> Self {
    Self {
      curve_type: "sigmoid".into(),
      max_speed: 2000.0,
      acceleration: 0.8,
      friction: 0.95,
      target_fps: 144,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct KeyBindings {
  /// Enables custom global and local shortcuts. Empty by default.
  pub enabled: bool,
  /// Capture keys only after navigation is explicitly activated.
  pub navigation_enabled: bool,
  pub preset: String,
  pub global: HashMap<String, String>,
  pub local: HashMap<String, String>,
}
impl Default for KeyBindings {
  fn default() -> Self {
    Self {
      enabled: false,
      navigation_enabled: true,
      preset: "vi".into(),
      global: HashMap::new(),
      local: HashMap::new(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ModeConfig {
  pub normal_multiplier: f64,
  pub precise_multiplier: f64,
  pub fast_multiplier: f64,
}
impl Default for ModeConfig {
  fn default() -> Self {
    Self {
      normal_multiplier: 1.0,
      precise_multiplier: 0.3,
      fast_multiplier: 2.5,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SystemConfig {
  pub hot_reload: bool,
  pub log_level: String,
}
impl Default for SystemConfig {
  fn default() -> Self {
    Self {
      hot_reload: true,
      log_level: "info".into(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NavigationConfig {
  pub rows: usize,
  pub columns: usize,
  pub alphabet: String,
  pub auto_click: bool,
}
impl Default for NavigationConfig {
  fn default() -> Self {
    Self {
      rows: 12,
      columns: 20,
      alphabet: "asdfghjklqwertyuiopzxcvbnm".into(),
      auto_click: false,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppearanceConfig {
  pub font_size: f64,
  pub foreground: String,
  pub background: String,
  pub highlight: String,
  pub opacity: f64,
  pub grid_lines: bool,
}
impl Default for AppearanceConfig {
  fn default() -> Self {
    Self {
      font_size: 14.0,
      foreground: "#FFFFFF".into(),
      background: "#17212B".into(),
      highlight: "#FFD166".into(),
      opacity: 0.85,
      grid_lines: true,
    }
  }
}

pub(crate) fn invalid(message: impl Into<String>) -> crate::Error {
  std::io::Error::new(std::io::ErrorKind::InvalidInput, message.into()).into()
}

pub fn validate_alphabet(alphabet: &str) -> Result<()> {
  let mut seen = std::collections::HashSet::new();
  if alphabet.len() < 2 || !alphabet.bytes().all(|c| c.is_ascii_lowercase() && seen.insert(c)) {
    return Err(invalid(
      "navigation.alphabet must contain at least two unique lowercase ASCII letters",
    ));
  }
  Ok(())
}

impl Config {
  pub fn load(path: &Path) -> Result<Self> {
    let config: Self = toml::from_str(&fs::read_to_string(path)?)?;
    config.validate()?;
    Ok(config)
  }

  pub fn validate(&self) -> Result<()> {
    let range = |name: &str, value: f64, min: f64, max: f64| -> Result<()> {
      if !value.is_finite() || !(min..=max).contains(&value) {
        return Err(invalid(format!("{name} must be finite and within {min}..={max}")));
      }
      Ok(())
    };
    range("motion.max_speed", self.motion.max_speed, 1.0, 100_000.0)?;
    range("motion.acceleration", self.motion.acceleration, 0.001, 1.0)?;
    range("motion.friction", self.motion.friction, 0.0, 0.9999)?;
    if !(1..=1000).contains(&self.motion.target_fps) {
      return Err(invalid("motion.target_fps must be within 1..=1000"));
    }
    if !["linear", "sigmoid", "exponential"].contains(&self.motion.curve_type.as_str()) {
      return Err(invalid("motion.curve_type must be linear, sigmoid, or exponential"));
    }
    for (name, value) in [
      ("normal", self.modes.normal_multiplier),
      ("precise", self.modes.precise_multiplier),
      ("fast", self.modes.fast_multiplier),
    ] {
      range(&format!("modes.{name}_multiplier"), value, 0.01, 10.0)?;
    }
    if !(1..=100).contains(&self.navigation.rows) || !(1..=100).contains(&self.navigation.columns) {
      return Err(invalid("navigation rows and columns must be within 1..=100"));
    }
    validate_alphabet(&self.navigation.alphabet)?;
    range("appearance.font_size", self.appearance.font_size, 8.0, 96.0)?;
    range("appearance.opacity", self.appearance.opacity, 0.1, 1.0)?;
    for color in [
      &self.appearance.foreground,
      &self.appearance.background,
      &self.appearance.highlight,
    ] {
      if color.len() != 7 || !color.starts_with('#') || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit) {
        return Err(invalid("appearance colors must use #RRGGBB"));
      }
    }
    if !["error", "warn", "info", "debug", "trace", "off"].contains(&self.system.log_level.as_str()) {
      return Err(invalid("invalid system.log_level"));
    }
    if !["emacs", "vi"].contains(&self.keybindings.preset.as_str()) {
      return Err(invalid("keybindings.preset must be emacs or vi"));
    }
    for (key, action) in self.keybindings.global.iter().chain(self.keybindings.local.iter()) {
      if key.trim().is_empty() || action.trim().is_empty() {
        return Err(invalid("keybindings require nonempty shortcuts and actions"));
      }
    }
    Ok(())
  }

  pub fn default_path() -> PathBuf {
    if let Some(path) = std::env::var_os("XDG_CONFIG_HOME").filter(|p| Path::new(p).is_absolute()) {
      return PathBuf::from(path).join("kact/kact.toml");
    }
    std::env::var_os("HOME")
      .map(PathBuf::from)
      .unwrap_or_else(|| PathBuf::from("."))
      .join(".config/kact/kact.toml")
  }
}

#[cfg(test)]
mod tests {
  use super::Config;

  #[test]
  fn only_vi_and_emacs_presets_are_valid() {
    let config = Config::default();
    assert_eq!(config.keybindings.preset, "vi");
    assert!(config.validate().is_ok());

    let mut emacs = config.clone();
    emacs.keybindings.preset = "emacs".into();
    assert!(emacs.validate().is_ok());

    let mut removed = config;
    removed.keybindings.preset = "system".into();
    assert!(removed.validate().is_err());
  }
}
