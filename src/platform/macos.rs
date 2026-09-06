use super::{CursorActuator, InputEvent, InputListener};
use crate::core::state::Mode;
use crate::core::types::{Direction, Vector2D};
use crate::{Error, Result};
use core_foundation::runloop::{CFRunLoop, kCFRunLoopCommonModes};
use core_graphics::display::CGDisplay;
use core_graphics::event::{
  CGEvent, CGEventTap, CGEventTapLocation, CGEventTapOptions, CGEventTapPlacement, CGEventType, EventField,
};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use crossbeam_channel::{Receiver, Sender, bounded};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// macOS virtual key codes
const KEY_W: i64 = 0x0D;
const KEY_A: i64 = 0x00;
const KEY_S: i64 = 0x01;
const KEY_D: i64 = 0x02;
const KEY_1: i64 = 0x12;
const KEY_2: i64 = 0x13;
const KEY_3: i64 = 0x14;
const KEY_SPACE: i64 = 0x31;
const KEY_ESCAPE: i64 = 0x35;

pub struct MacOSInputListener {
  event_rx: Receiver<InputEvent>,
  running: bool,
  tap_thread: Option<thread::JoinHandle<()>>,
}

impl MacOSInputListener {
  pub fn new() -> Result<Self> {
    let (_, rx) = bounded::<InputEvent>(100);

    Ok(Self {
      event_rx: rx,
      running: false,
      tap_thread: None,
    })
  }

  fn map_keycode_to_event(keycode: i64, is_press: bool) -> Option<InputEvent> {
    match keycode {
      KEY_W => Some(if is_press {
        InputEvent::DirectionPressed(Direction::Up)
      } else {
        InputEvent::DirectionReleased(Direction::Up)
      }),
      KEY_S => Some(if is_press {
        InputEvent::DirectionPressed(Direction::Down)
      } else {
        InputEvent::DirectionReleased(Direction::Down)
      }),
      KEY_A => Some(if is_press {
        InputEvent::DirectionPressed(Direction::Left)
      } else {
        InputEvent::DirectionReleased(Direction::Left)
      }),
      KEY_D => Some(if is_press {
        InputEvent::DirectionPressed(Direction::Right)
      } else {
        InputEvent::DirectionReleased(Direction::Right)
      }),
      KEY_1 if is_press => Some(InputEvent::ModeChanged(Mode::Normal)),
      KEY_2 if is_press => Some(InputEvent::ModeChanged(Mode::Precise)),
      KEY_3 if is_press => Some(InputEvent::ModeChanged(Mode::Fast)),
      KEY_SPACE if is_press => Some(InputEvent::ToggleActive),
      KEY_ESCAPE if is_press => Some(InputEvent::EmergencyStop),
      _ => None,
    }
  }
}

impl InputListener for MacOSInputListener {
  fn start(&mut self) -> Result<()> {
    let (tx, rx) = bounded::<InputEvent>(100);
    self.event_rx = rx;
    self.running = true;

    // Spawn thread for CGEventTap and CFRunLoop
    let handle = thread::spawn(move || {
      if let Err(e) = run_event_tap(tx) {
        tracing::error!("Event tap failed: {}", e);
      }
    });

    self.tap_thread = Some(handle);
    tracing::info!("macOS input listener started with CGEventTap");
    tracing::info!("Note: Requires accessibility permissions in System Preferences");

    Ok(())
  }

  fn next_event(&mut self) -> Result<Option<InputEvent>> {
    if !self.running {
      return Ok(None);
    }

    match self.event_rx.recv_timeout(Duration::from_millis(10)) {
      Ok(event) => Ok(Some(event)),
      Err(crossbeam_channel::RecvTimeoutError::Timeout) => Ok(None),
      Err(crossbeam_channel::RecvTimeoutError::Disconnected) => Ok(None),
    }
  }

  fn stop(&mut self) -> Result<()> {
    self.running = false;
    // Note: tap_thread will be terminated when CFRunLoop stops
    // In a production implementation, we'd send a signal to stop the run loop gracefully
    Ok(())
  }
}

fn run_event_tap(tx: Sender<InputEvent>) -> Result<()> {
  let tx = Arc::new(tx);

  // Create event tap for keyboard events
  let tap = CGEventTap::new(
    CGEventTapLocation::HID,
    CGEventTapPlacement::HeadInsertEventTap,
    CGEventTapOptions::ListenOnly, // Use ListenOnly to avoid interfering with other apps
    vec![CGEventType::KeyDown, CGEventType::KeyUp],
    {
      let tx = Arc::clone(&tx);
      move |_proxy, event_type, event| {
        let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
        let is_press = matches!(event_type, CGEventType::KeyDown);

        if let Some(input_event) = MacOSInputListener::map_keycode_to_event(keycode, is_press) {
          let _ = tx.try_send(input_event);
        }

        Some(event.clone())
      }
    },
  )
  .map_err(|_| {
    Error::Platform("Failed to create CGEventTap. Enable accessibility permissions for this app.".to_string())
  })?;

  tap.enable();

  // Add tap to run loop and run it (blocks until stopped)
  unsafe {
    let run_loop = CFRunLoop::get_current();

    let source = tap
      .mach_port
      .create_runloop_source(0)
      .map_err(|_| Error::Platform("Failed to create run loop source".to_string()))?;

    run_loop.add_source(&source, kCFRunLoopCommonModes);
    CFRunLoop::run_current();
  }

  Ok(())
}

pub struct MacOSCursorActuator {}

impl MacOSCursorActuator {
  pub fn new() -> Result<Self> {
    Ok(Self {})
  }
}

impl CursorActuator for MacOSCursorActuator {
  fn move_relative(&mut self, delta: Vector2D) -> Result<()> {
    let current = self.get_position()?;
    let new_pos = current.add(&delta);
    self.move_absolute(new_pos)
  }

  fn move_absolute(&mut self, position: Vector2D) -> Result<()> {
    let display = CGDisplay::main();
    let point = core_graphics::geometry::CGPoint {
      x: position.x,
      y: position.y,
    };

    let result = display.move_cursor_to_point(point);

    if result.is_err() {
      return Err(Error::Platform(format!(
        "Failed to move cursor to ({}, {})",
        position.x, position.y
      )));
    }

    tracing::trace!("Moved cursor to ({}, {})", position.x, position.y);

    Ok(())
  }

  fn get_position(&self) -> Result<Vector2D> {
    // Use CGEventSource to get current mouse position
    let event_source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
      .map_err(|_| Error::Platform("Failed to create event source".to_string()))?;

    let event = CGEvent::new(event_source).map_err(|_| Error::Platform("Failed to create event".to_string()))?;

    let location = event.location();
    Ok(Vector2D {
      x: location.x,
      y: location.y,
    })
  }
}
