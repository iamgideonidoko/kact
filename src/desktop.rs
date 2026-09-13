//! Native desktop presentation. All methods must be called on the main thread.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LabelPosition {
  #[default]
  Center,
  Top,
  Right,
  Bottom,
  Left,
  TopLeft,
  TopRight,
  BottomLeft,
  BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Rect {
  pub x: f64,
  pub y: f64,
  pub width: f64,
  pub height: f64,
}

#[derive(Debug, Clone)]
pub struct Target {
  pub label: String,
  pub bounds: Rect,
  pub focused: bool,
}

#[derive(Debug, Clone)]
pub struct Appearance {
  pub font_size: f64,
  pub foreground: String,
  pub background: String,
  pub highlight: String,
  pub opacity: f64,
  pub grid_lines: bool,
  pub label_position: LabelPosition,
  pub visual_targets: bool,
  pub refreshing: bool,
}

impl Default for Appearance {
  fn default() -> Self {
    Self {
      font_size: 16.0,
      foreground: "#FFFFFF".into(),
      background: "#17212B".into(),
      highlight: "#F5C451".into(),
      opacity: 0.85,
      grid_lines: true,
      label_position: LabelPosition::Center,
      visual_targets: false,
      refreshing: false,
    }
  }
}

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::Desktop;

#[cfg(not(target_os = "macos"))]
pub struct Desktop;

#[cfg(not(target_os = "macos"))]
impl Desktop {
  pub fn new() -> anyhow::Result<Self> {
    anyhow::bail!("Native overlays currently require macOS")
  }
  pub fn screens(&self) -> anyhow::Result<Vec<Rect>> {
    anyhow::bail!("Display discovery currently requires macOS")
  }
  pub fn elements(&self) -> anyhow::Result<Vec<Rect>> {
    anyhow::bail!("Accessibility targeting currently requires macOS")
  }
  pub fn set_enhanced_user_interface(&mut self, _: bool) -> anyhow::Result<()> {
    anyhow::bail!("Accessibility targeting currently requires macOS")
  }
  pub fn set_visual_fallback(&mut self, _: bool) {}
  pub fn observe_element_changes(&mut self) -> anyhow::Result<()> {
    Ok(())
  }
  pub fn take_element_refresh_requested(&mut self) -> bool {
    false
  }
  pub fn stop_observing_element_changes(&mut self) {}
  pub fn matching_elements(&self, _: &str) -> Vec<Rect> {
    vec![]
  }
  pub fn focus_token(&self) -> anyhow::Result<String> {
    anyhow::bail!("Focus tracking currently requires macOS")
  }
  pub fn show(&mut self, _: &[Target], _: &str, _: &Appearance) -> anyhow::Result<()> {
    anyhow::bail!("Native overlays currently require macOS")
  }
  pub fn hide(&mut self) {}
  pub fn pump(&mut self) {}
}
