use super::types::{Direction, Vector2D};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
  Normal,
  Precise,
  Fast,
}

impl Default for Mode {
  fn default() -> Self {
    Mode::Normal
  }
}

#[derive(Debug, Clone, Default)]
pub struct InputState {
  pub active_directions: HashSet<Direction>,
  pub mode: Mode,
}

impl InputState {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn press_direction(&mut self, dir: Direction) {
    self.active_directions.insert(dir);
  }

  pub fn release_direction(&mut self, dir: Direction) {
    self.active_directions.remove(&dir);
  }

  pub fn set_mode(&mut self, mode: Mode) {
    self.mode = mode;
  }

  pub fn get_input_vector(&self) -> Vector2D {
    let mut result = Vector2D::zero();
    for dir in &self.active_directions {
      let v = dir.to_vector();
      result = result.add(&v);
    }
    result.normalize()
  }
}

#[derive(Debug, Clone)]
pub struct AppState {
  pub active: bool,
  pub emergency_stop: bool,
  pub input: InputState,
  pub velocity: Vector2D,
  pub position: Vector2D,
}

impl Default for AppState {
  fn default() -> Self {
    Self {
      active: false,
      emergency_stop: false,
      input: InputState::new(),
      velocity: Vector2D::zero(),
      position: Vector2D::zero(),
    }
  }
}

impl AppState {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn toggle_active(&mut self) {
    self.active = !self.active;
  }

  pub fn trigger_emergency_stop(&mut self) {
    self.emergency_stop = true;
    self.active = false;
  }
}
