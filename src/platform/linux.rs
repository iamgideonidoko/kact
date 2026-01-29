use super::{CursorActuator, InputEvent, InputListener};
use crate::core::types::Vector2D;
use crate::{Error, Result};

pub struct LinuxInputListener {
  // TODO: Implement using X11 or evdev
}

impl LinuxInputListener {
  pub fn new() -> Result<Self> {
    Ok(Self {})
  }
}

impl InputListener for LinuxInputListener {
  fn start(&mut self) -> Result<()> {
    // TODO: Set up X11 event monitoring or evdev
    Err(Error::Platform("Linux input listener not yet implemented".to_string()))
  }

  fn next_event(&mut self) -> Result<Option<InputEvent>> {
    // TODO: Poll for next input event
    Ok(None)
  }

  fn stop(&mut self) -> Result<()> {
    // TODO: Clean up
    Ok(())
  }
}

pub struct LinuxCursorActuator {
  // TODO: Implement using X11 XTest extension
}

impl LinuxCursorActuator {
  pub fn new() -> Result<Self> {
    Ok(Self {})
  }
}

impl CursorActuator for LinuxCursorActuator {
  fn move_relative(&mut self, delta: Vector2D) -> Result<()> {
    // TODO: Use XTestFakeRelativeMotionEvent
    tracing::trace!("move_relative: ({}, {})", delta.x, delta.y);
    Ok(())
  }

  fn move_absolute(&mut self, position: Vector2D) -> Result<()> {
    // TODO: Use XTestFakeMotionEvent
    tracing::trace!("move_absolute: ({}, {})", position.x, position.y);
    Ok(())
  }

  fn get_position(&self) -> Result<Vector2D> {
    // TODO: Use XQueryPointer
    Ok(Vector2D::zero())
  }
}
