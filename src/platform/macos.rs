use super::{CursorActuator, InputEvent, InputListener};
use crate::core::types::Vector2D;
use crate::{Error, Result};

pub struct MacOSInputListener {
  // TODO: Implement using Core Graphics event tap
}

impl MacOSInputListener {
  pub fn new() -> Result<Self> {
    Ok(Self {})
  }
}

impl InputListener for MacOSInputListener {
  fn start(&mut self) -> Result<()> {
    // TODO: Set up CGEventTap for keyboard monitoring
    Err(Error::Platform("MacOS input listener not yet implemented".to_string()))
  }

  fn next_event(&mut self) -> Result<Option<InputEvent>> {
    // TODO: Poll for next input event
    Ok(None)
  }

  fn stop(&mut self) -> Result<()> {
    // TODO: Clean up event tap
    Ok(())
  }
}

pub struct MacOSCursorActuator {
  // TODO: Implement using Core Graphics
}

impl MacOSCursorActuator {
  pub fn new() -> Result<Self> {
    Ok(Self {})
  }
}

impl CursorActuator for MacOSCursorActuator {
  fn move_relative(&mut self, delta: Vector2D) -> Result<()> {
    // TODO: Use CGWarpMouseCursorPosition or CGEventCreateMouseEvent
    // For now, placeholder implementation
    tracing::trace!("move_relative: ({}, {})", delta.x, delta.y);
    Ok(())
  }

  fn move_absolute(&mut self, position: Vector2D) -> Result<()> {
    // TODO: Use CGWarpMouseCursorPosition
    tracing::trace!("move_absolute: ({}, {})", position.x, position.y);
    Ok(())
  }

  fn get_position(&self) -> Result<Vector2D> {
    // TODO: Use CGEventGetLocation
    Ok(Vector2D::zero())
  }
}
