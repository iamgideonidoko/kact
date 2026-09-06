use crate::core::types::Vector2D;
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, atomic::AtomicBool};

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;

#[derive(Debug, Clone)]
pub struct KeyEvent {
  pub key: String,
  pub modifiers: Vec<String>,
  pub pressed: bool,
  pub repeat: bool,
}
impl KeyEvent {
  pub fn shortcut(&self) -> String {
    let mut parts = self.modifiers.clone();
    parts.push(self.key.clone());
    canonical_shortcut(&parts.join("+")).unwrap_or_else(|_| self.key.clone())
  }
}
pub type InputEvent = KeyEvent;

pub fn canonical_shortcut(value: &str) -> Result<String> {
  let parts: Vec<_> = value.split('+').map(|p| p.trim().to_lowercase()).collect();
  let mut modifiers = Vec::new();
  let mut key = None;
  for part in parts {
    let part = match part.as_str() {
      "control" => "ctrl",
      "option" => "alt",
      "command" | "super" | "meta" => "cmd",
      "esc" => "escape",
      "period" => ".",
      "comma" => ",",
      "equal" => "=",
      "minus" => "-",
      "leftbracket" => "[",
      "rightbracket" => "]",
      "backslash" => "\\",
      "return" => "enter",
      " " => "space",
      other => other,
    };
    if matches!(part, "ctrl" | "alt" | "shift" | "cmd") {
      if modifiers.contains(&part.to_string()) {
        return Err(Error::Platform("Duplicate shortcut modifier".into()));
      }
      modifiers.push(part.to_string());
    } else if part.is_empty() || key.replace(part.to_string()).is_some() {
      return Err(Error::Platform(format!("Invalid shortcut: {value}")));
    }
  }
  let key = key.ok_or_else(|| Error::Platform(format!("Shortcut needs a key: {value}")))?;
  if !(key.chars().count() == 1
    || matches!(
      key.as_str(),
      "space"
        | "escape"
        | "enter"
        | "tab"
        | "backspace"
        | "delete"
        | "left"
        | "right"
        | "up"
        | "down"
        | "home"
        | "end"
        | "pageup"
        | "pagedown"
    )
    || key
      .strip_prefix('f')
      .and_then(|s| s.parse::<u8>().ok())
      .is_some_and(|n| (1..=20).contains(&n)))
  {
    return Err(Error::Platform(format!("Unknown shortcut key: {key}")));
  }
  modifiers.sort();
  modifiers.push(key);
  Ok(modifiers.join("+"))
}

#[derive(Clone)]
pub struct InputOptions {
  pub global_shortcuts: Vec<String>,
  pub navigation_keys: Vec<String>,
  pub label_keys: Vec<String>,
  pub labels_active: Arc<AtomicBool>,
  pub active: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum MouseButton {
  Left,
  Right,
  Middle,
}

pub trait InputListener: Send {
  fn start(&mut self) -> Result<()>;
  fn next_event(&mut self) -> Result<Option<KeyEvent>>;
  fn stop(&mut self) -> Result<()>;
}
pub trait CursorActuator {
  fn move_relative(&mut self, delta: Vector2D) -> Result<()>;
  fn move_absolute(&mut self, position: Vector2D) -> Result<()>;
  fn get_position(&self) -> Result<Vector2D>;
  fn click(&mut self, button: MouseButton, count: u8, modifiers: &[String]) -> Result<()>;
  fn button_down(&mut self, button: MouseButton, modifiers: &[String]) -> Result<()>;
  fn button_up(&mut self, button: MouseButton, modifiers: &[String]) -> Result<()>;
  /// Positive values scroll left and up, in pixels (X11 uses wheel steps).
  fn scroll(&mut self, dx: i32, dy: i32) -> Result<()>;
  fn release_all(&mut self) -> Result<()>;
}
pub fn create_input_listener(options: InputOptions) -> Result<Box<dyn InputListener>> {
  #[cfg(target_os = "macos")]
  return Ok(Box::new(macos::MacOSInputListener::new(options)?));
  #[cfg(target_os = "linux")]
  return linux::create_input_listener(options);
  #[cfg(not(any(target_os = "macos", target_os = "linux")))]
  Err(Error::Platform("Unsupported platform".into()))
}
pub fn create_cursor_actuator() -> Result<Box<dyn CursorActuator>> {
  #[cfg(target_os = "macos")]
  return Ok(Box::new(macos::MacOSCursorActuator::new()?));
  #[cfg(target_os = "linux")]
  return Ok(Box::new(linux::LinuxCursorActuator::new()?));
  #[cfg(not(any(target_os = "macos", target_os = "linux")))]
  Err(Error::Platform("Unsupported platform".into()))
}
pub fn accessibility_trusted(prompt: bool) -> bool {
  #[cfg(target_os = "macos")]
  return macos::accessibility_trusted(prompt);
  #[cfg(not(target_os = "macos"))]
  {
    let _ = prompt;
    true
  }
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn shortcut_normalization() {
    assert_eq!(canonical_shortcut("Control+Option+K").unwrap(), "alt+ctrl+k");
    assert!(canonical_shortcut("ctrl+ctrl+k").is_err());
    assert!(canonical_shortcut("ctrl").is_err());
    assert!(canonical_shortcut("a+b").is_err());
  }
}
