use super::{CursorActuator, InputListener, InputOptions, MouseButton};
use crate::core::types::Vector2D;
use crate::{Error, Result};
use x11::{xlib, xtest};

pub fn create_input_listener(_options: InputOptions) -> Result<Box<dyn InputListener>> {
  Err(Error::Platform("Built-in keyboard capture is available on macOS. On X11, bind kact commands in your window manager or shortcut tool.".into()))
}

pub struct LinuxCursorActuator {
  display: *mut xlib::Display,
  held: Vec<MouseButton>,
  remainder: Vector2D,
}
impl LinuxCursorActuator {
  pub fn new() -> Result<Self> {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() {
      return Err(Error::Platform(
        "Native Wayland cursor injection is unsupported; use an X11 session".into(),
      ));
    }
    let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };
    if display.is_null() {
      return Err(Error::Platform("Cannot open X11 display; check DISPLAY".into()));
    }
    let (mut event, mut error, mut major, mut minor) = (0, 0, 0, 0);
    if unsafe { xtest::XTestQueryExtension(display, &mut event, &mut error, &mut major, &mut minor) } == 0 {
      unsafe {
        xlib::XCloseDisplay(display);
      }
      return Err(Error::Platform("X11 server lacks XTest extension".into()));
    }
    Ok(Self {
      display,
      held: Vec::new(),
      remainder: Vector2D::zero(),
    })
  }
  fn flush(&self, success: i32) -> Result<()> {
    if success == 0 {
      return Err(Error::Platform("X11 input injection failed".into()));
    }
    unsafe {
      xlib::XFlush(self.display);
    }
    Ok(())
  }
  fn button(&self, button: u32, pressed: bool) -> Result<()> {
    self.flush(unsafe { xtest::XTestFakeButtonEvent(self.display, button, i32::from(pressed), 0) })
  }
  fn reject_modifiers(modifiers: &[String]) -> Result<()> {
    if !modifiers.is_empty() {
      return Err(Error::Platform(
        "Modified clicks are currently available on macOS only".into(),
      ));
    }
    Ok(())
  }
}
fn button_number(button: MouseButton) -> u32 {
  match button {
    MouseButton::Left => 1,
    MouseButton::Middle => 2,
    MouseButton::Right => 3,
  }
}
impl CursorActuator for LinuxCursorActuator {
  fn move_relative(&mut self, delta: Vector2D) -> Result<()> {
    if !delta.x.is_finite() || !delta.y.is_finite() {
      return Err(Error::Platform("Cursor coordinates must be finite".into()));
    }
    let total = delta.add(&self.remainder);
    let whole = Vector2D::new(total.x.round(), total.y.round());
    if whole.x != 0.0 || whole.y != 0.0 {
      let target = self.get_position()?.add(&whole);
      self.move_absolute(target)?;
    }
    self.remainder = Vector2D::new(total.x - whole.x, total.y - whole.y);
    Ok(())
  }
  fn move_absolute(&mut self, position: Vector2D) -> Result<()> {
    if !position.x.is_finite() || !position.y.is_finite() {
      return Err(Error::Platform("Cursor coordinates must be finite".into()));
    }
    self.flush(unsafe {
      xtest::XTestFakeMotionEvent(
        self.display,
        -1,
        position.x.round() as i32,
        position.y.round() as i32,
        0,
      )
    })?;
    self.remainder = Vector2D::zero();
    Ok(())
  }
  fn get_position(&self) -> Result<Vector2D> {
    let root = unsafe { xlib::XDefaultRootWindow(self.display) };
    let (mut root_return, mut child) = (0, 0);
    let (mut root_x, mut root_y, mut win_x, mut win_y) = (0, 0, 0, 0);
    let mut mask = 0;
    let success = unsafe {
      xlib::XQueryPointer(
        self.display,
        root,
        &mut root_return,
        &mut child,
        &mut root_x,
        &mut root_y,
        &mut win_x,
        &mut win_y,
        &mut mask,
      )
    };
    if success == 0 {
      return Err(Error::Platform("Cannot query X11 pointer".into()));
    }
    Ok(Vector2D {
      x: root_x as f64,
      y: root_y as f64,
    })
  }
  fn click(&mut self, button: MouseButton, count: u8, modifiers: &[String]) -> Result<()> {
    Self::reject_modifiers(modifiers)?;
    if !(1..=3).contains(&count) {
      return Err(Error::Platform("Click count must be 1 through 3".into()));
    }
    if self.held.contains(&button) {
      return Err(Error::Platform("Release held button before clicking it".into()));
    }
    for _ in 0..count {
      self.button_down(button, modifiers)?;
      self.button_up(button, modifiers)?;
    }
    Ok(())
  }
  fn button_down(&mut self, button: MouseButton, modifiers: &[String]) -> Result<()> {
    Self::reject_modifiers(modifiers)?;
    if !self.held.contains(&button) {
      self.button(button_number(button), true)?;
      self.held.push(button);
    }
    Ok(())
  }
  fn button_up(&mut self, button: MouseButton, modifiers: &[String]) -> Result<()> {
    Self::reject_modifiers(modifiers)?;
    self.button(button_number(button), false)?;
    self.held.retain(|held| *held != button);
    Ok(())
  }
  fn scroll(&mut self, dx: i32, dy: i32) -> Result<()> {
    if dx.unsigned_abs() > 1000 || dy.unsigned_abs() > 1000 {
      return Err(Error::Platform("X11 scroll must be within 1000 wheel steps".into()));
    }
    for (amount, negative, positive) in [(dy, 5, 4), (dx, 7, 6)] {
      for _ in 0..amount.unsigned_abs() {
        let button = if amount < 0 { negative } else { positive };
        self.button(button, true)?;
        self.button(button, false)?;
      }
    }
    Ok(())
  }
  fn release_all(&mut self) -> Result<()> {
    let mut error = None;
    for button in self.held.clone() {
      if let Err(e) = self.button_up(button, &[]) {
        error = Some(e);
      }
    }
    error.map_or(Ok(()), Err)
  }
}
impl Drop for LinuxCursorActuator {
  fn drop(&mut self) {
    let _ = self.release_all();
    unsafe {
      xlib::XCloseDisplay(self.display);
    }
  }
}
