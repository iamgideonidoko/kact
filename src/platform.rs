use crate::Result;
use crate::core::state::Mode;
use crate::core::types::{Direction, Vector2D};

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "linux")]
pub mod linux;

/// Platform-agnostic input event
#[derive(Debug, Clone)]
pub enum InputEvent {
  DirectionPressed(Direction),
  DirectionReleased(Direction),
  ModeChanged(Mode),
  ToggleActive,
  EmergencyStop,
}

/// Trait for listening to keyboard input (Thread A)
pub trait InputListener: Send {
  fn start(&mut self) -> Result<()>;
  fn next_event(&mut self) -> Result<Option<InputEvent>>;
  fn stop(&mut self) -> Result<()>;
}

/// Trait for moving the mouse cursor (Thread B)
pub trait CursorActuator: Send {
  fn move_relative(&mut self, delta: Vector2D) -> Result<()>;
  fn move_absolute(&mut self, position: Vector2D) -> Result<()>;
  fn get_position(&self) -> Result<Vector2D>;
}

/// Factory function to create platform-specific input listener
pub fn create_input_listener() -> Result<Box<dyn InputListener>> {
  #[cfg(target_os = "macos")]
  return Ok(Box::new(macos::MacOSInputListener::new()?));

  #[cfg(target_os = "linux")]
  return Ok(Box::new(linux::LinuxInputListener::new()?));

  #[cfg(not(any(target_os = "macos", target_os = "linux")))]
  return Err(crate::Error::Platform("Unsupported platform".to_string()));
}

/// Factory function to create platform-specific cursor actuator
pub fn create_cursor_actuator() -> Result<Box<dyn CursorActuator>> {
  #[cfg(target_os = "macos")]
  return Ok(Box::new(macos::MacOSCursorActuator::new()?));

  #[cfg(target_os = "linux")]
  return Ok(Box::new(linux::LinuxCursorActuator::new()?));

  #[cfg(not(any(target_os = "macos", target_os = "linux")))]
  return Err(crate::Error::Platform("Unsupported platform".to_string()));
}
