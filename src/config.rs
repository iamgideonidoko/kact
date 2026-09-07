use crate::{Result, desktop::LabelPosition};
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
  pub grid: NavigationOverride,
  pub elements: NavigationOverride,
}
impl Default for NavigationConfig {
  fn default() -> Self {
    Self {
      rows: 12,
      columns: 20,
      alphabet: "asdfghjklqwertyuiopzxcvbnm".into(),
      auto_click: false,
      grid: NavigationOverride::default(),
      elements: NavigationOverride::default(),
    }
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NavigationOverride {
  pub rows: Option<usize>,
  pub columns: Option<usize>,
  pub alphabet: Option<String>,
  pub auto_click: Option<bool>,
}

impl NavigationConfig {
  fn resolve(&self, override_config: &NavigationOverride) -> Self {
    Self {
      rows: override_config.rows.unwrap_or(self.rows),
      columns: override_config.columns.unwrap_or(self.columns),
      alphabet: override_config
        .alphabet
        .clone()
        .unwrap_or_else(|| self.alphabet.clone()),
      auto_click: override_config.auto_click.unwrap_or(self.auto_click),
      grid: NavigationOverride::default(),
      elements: NavigationOverride::default(),
    }
  }

  pub fn for_grid(&self) -> Self {
    self.resolve(&self.grid)
  }

  pub fn for_elements(&self) -> Self {
    self.resolve(&self.elements)
  }

  pub fn alphabets(&self) -> Vec<String> {
    let mut result = vec![
      self.alphabet.clone(),
      self.for_grid().alphabet,
      self.for_elements().alphabet,
    ];
    result.sort();
    result.dedup();
    result
  }

  fn validate(&self) -> Result<()> {
    for (name, resolved) in [
      ("navigation", self.clone()),
      ("navigation.grid", self.for_grid()),
      ("navigation.elements", self.for_elements()),
    ] {
      if !(1..=100).contains(&resolved.rows) || !(1..=100).contains(&resolved.columns) {
        return Err(invalid(format!("{name} rows and columns must be within 1..=100")));
      }
      validate_alphabet(&resolved.alphabet)?;
    }
    Ok(())
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
  pub label_position: LabelPosition,
  pub grid: AppearanceOverride,
  pub elements: AppearanceOverride,
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
      label_position: LabelPosition::Center,
      grid: AppearanceOverride::default(),
      elements: AppearanceOverride::default(),
    }
  }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppearanceOverride {
  pub font_size: Option<f64>,
  pub foreground: Option<String>,
  pub background: Option<String>,
  pub highlight: Option<String>,
  pub opacity: Option<f64>,
  pub grid_lines: Option<bool>,
  pub label_position: Option<LabelPosition>,
}

impl AppearanceConfig {
  fn resolve(&self, override_config: &AppearanceOverride) -> Self {
    Self {
      font_size: override_config.font_size.unwrap_or(self.font_size),
      foreground: override_config
        .foreground
        .clone()
        .unwrap_or_else(|| self.foreground.clone()),
      background: override_config
        .background
        .clone()
        .unwrap_or_else(|| self.background.clone()),
      highlight: override_config
        .highlight
        .clone()
        .unwrap_or_else(|| self.highlight.clone()),
      opacity: override_config.opacity.unwrap_or(self.opacity),
      grid_lines: override_config.grid_lines.unwrap_or(self.grid_lines),
      label_position: override_config.label_position.unwrap_or(self.label_position),
      grid: AppearanceOverride::default(),
      elements: AppearanceOverride::default(),
    }
  }

  pub fn for_grid(&self) -> Self {
    self.resolve(&self.grid)
  }

  pub fn for_elements(&self) -> Self {
    self.resolve(&self.elements)
  }

  fn validate(&self, range: impl Fn(&str, f64, f64, f64) -> Result<()>) -> Result<()> {
    for (name, resolved) in [
      ("appearance", self.clone()),
      ("appearance.grid", self.for_grid()),
      ("appearance.elements", self.for_elements()),
    ] {
      range(&format!("{name}.font_size"), resolved.font_size, 8.0, 96.0)?;
      range(&format!("{name}.opacity"), resolved.opacity, 0.1, 1.0)?;
      for color in [&resolved.foreground, &resolved.background, &resolved.highlight] {
        if color.len() != 7 || !color.starts_with('#') || !color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit) {
          return Err(invalid(format!("{name} colors must use #RRGGBB")));
        }
      }
    }
    Ok(())
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
    self.navigation.validate()?;
    self.appearance.validate(range)?;
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

  #[test]
  fn mode_overrides_inherit_and_validate() {
    let config: Config = toml::from_str(
      r##"
      [navigation]
      rows = 10
      alphabet = "abcd"
      [navigation.grid]
      columns = 8
      [navigation.elements]
      auto_click = true

      [appearance]
      background = "#112233"
      [appearance.grid]
      grid_lines = false
      [appearance.elements]
      opacity = 0.5
      label_position = "top-left"
      "##,
    )
    .unwrap();
    config.validate().unwrap();
    let grid = config.navigation.for_grid();
    assert_eq!((grid.rows, grid.columns, grid.alphabet.as_str()), (10, 8, "abcd"));
    assert!(config.navigation.for_elements().auto_click);
    assert!(!config.appearance.for_grid().grid_lines);
    assert_eq!(config.appearance.for_elements().opacity, 0.5);
    assert_eq!(
      config.appearance.for_elements().label_position,
      crate::desktop::LabelPosition::TopLeft
    );
  }
}
