use super::{CursorActuator, InputEvent, InputListener};
use crate::core::state::Mode;
use crate::core::types::{Direction, Vector2D};
use crate::{Error, Result};
use crossbeam_channel::{Receiver, Sender, bounded};
use std::thread;
use std::time::Duration;

#[cfg(target_os = "linux")]
use x11::xlib;
#[cfg(target_os = "linux")]
use x11::xrecord;
#[cfg(target_os = "linux")]
use x11::xtest;

// X11 keysyms (from /usr/include/X11/keysymdef.h)
#[cfg(target_os = "linux")]
const XK_W: u32 = 0x0077;
#[cfg(target_os = "linux")]
const XK_A: u32 = 0x0061;
#[cfg(target_os = "linux")]
const XK_S: u32 = 0x0073;
#[cfg(target_os = "linux")]
const XK_D: u32 = 0x0064;
#[cfg(target_os = "linux")]
const XK_1: u32 = 0x0031;
#[cfg(target_os = "linux")]
const XK_2: u32 = 0x0032;
#[cfg(target_os = "linux")]
const XK_3: u32 = 0x0033;
#[cfg(target_os = "linux")]
const XK_SPACE: u32 = 0x0020;
#[cfg(target_os = "linux")]
const XK_ESCAPE: u32 = 0xff1b;

pub struct LinuxInputListener {
  event_rx: Receiver<InputEvent>,
  running: bool,
  #[cfg(target_os = "linux")]
  record_thread: Option<thread::JoinHandle<()>>,
}

impl LinuxInputListener {
  pub fn new() -> Result<Self> {
    let (_, rx) = bounded::<InputEvent>(100);

    Ok(Self {
      event_rx: rx,
      running: false,
      #[cfg(target_os = "linux")]
      record_thread: None,
    })
  }

  #[cfg(target_os = "linux")]
  fn map_keysym_to_event(keysym: u32, is_press: bool) -> Option<InputEvent> {
    match keysym {
      XK_W => Some(if is_press {
        InputEvent::DirectionPressed(Direction::Up)
      } else {
        InputEvent::DirectionReleased(Direction::Up)
      }),
      XK_S => Some(if is_press {
        InputEvent::DirectionPressed(Direction::Down)
      } else {
        InputEvent::DirectionReleased(Direction::Down)
      }),
      XK_A => Some(if is_press {
        InputEvent::DirectionPressed(Direction::Left)
      } else {
        InputEvent::DirectionReleased(Direction::Left)
      }),
      XK_D => Some(if is_press {
        InputEvent::DirectionPressed(Direction::Right)
      } else {
        InputEvent::DirectionReleased(Direction::Right)
      }),
      XK_1 if is_press => Some(InputEvent::ModeChanged(Mode::Normal)),
      XK_2 if is_press => Some(InputEvent::ModeChanged(Mode::Precise)),
      XK_3 if is_press => Some(InputEvent::ModeChanged(Mode::Fast)),
      XK_SPACE if is_press => Some(InputEvent::ToggleActive),
      XK_ESCAPE if is_press => Some(InputEvent::EmergencyStop),
      _ => None,
    }
  }
}

impl InputListener for LinuxInputListener {
  fn start(&mut self) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
      let (tx, rx) = bounded::<InputEvent>(100);
      self.event_rx = rx;
      self.running = true;

      // Spawn thread for XRecord
      let handle = thread::spawn(move || {
        if let Err(e) = run_xrecord(tx) {
          tracing::error!("XRecord failed: {}", e);
        }
      });

      self.record_thread = Some(handle);
      tracing::info!("Linux input listener started with XRecord");
      tracing::info!("Note: May require X11 input permissions");
      
      return Ok(());
    }

    #[cfg(not(target_os = "linux"))]
    {
      self.running = true;
      tracing::warn!("Linux input listener running in stub mode (not on Linux)");
      Ok(())
    }
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
    // Note: record_thread will be terminated when XRecord context is disabled
    Ok(())
  }
}

#[cfg(target_os = "linux")]
fn run_xrecord(tx: Sender<InputEvent>) -> Result<()> {
  unsafe {
    // Open display connections
    let data_display = xlib::XOpenDisplay(std::ptr::null());
    if data_display.is_null() {
      return Err(Error::Platform(
        "Failed to open X11 display for data".to_string(),
      ));
    }

    let ctrl_display = xlib::XOpenDisplay(std::ptr::null());
    if ctrl_display.is_null() {
      xlib::XCloseDisplay(data_display);
      return Err(Error::Platform(
        "Failed to open X11 display for control".to_string(),
      ));
    }

    // Set up XRecord range for key events
    let mut range = xrecord::XRecordAllocRange();
    if range.is_null() {
      xlib::XCloseDisplay(data_display);
      xlib::XCloseDisplay(ctrl_display);
      return Err(Error::Platform("Failed to allocate XRecord range".to_string()));
    }

    (*range).device_events.first = xlib::KeyPress as u8;
    (*range).device_events.last = xlib::KeyRelease as u8;

    // Create XRecord context
    let mut clients = xrecord::XRecordAllClients;
    let context = xrecord::XRecordCreateContext(
      ctrl_display,
      0,
      &mut clients,
      1,
      &mut range as *mut *mut xrecord::XRecordRange,
      1,
    );

    if context == 0 {
      xlib::XCloseDisplay(data_display);
      xlib::XCloseDisplay(ctrl_display);
      return Err(Error::Platform(
        "Failed to create XRecord context".to_string(),
      ));
    }

    xlib::XSync(ctrl_display, xlib::False);

    // XRecord callback data
    struct CallbackData {
      tx: Sender<InputEvent>,
      display: *mut xlib::Display,
    }

    let mut callback_data = CallbackData {
      tx,
      display: data_display,
    };

    extern "C" fn xrecord_callback(
      _closure: *mut i8,
      raw_data: *mut xrecord::XRecordInterceptData,
    ) {
      unsafe {
        if raw_data.is_null() {
          return;
        }

        let data = &*raw_data;
        let callback_data = &*(_closure as *mut CallbackData);

        if data.category == xrecord::XRecordFromServer {
          let event = data.data as *const u8;
          let event_type = *event;

          if event_type == xlib::KeyPress as u8 || event_type == xlib::KeyRelease as u8 {
            let keycode = *event.offset(1);
            let keysym = xlib::XKeycodeToKeysym(callback_data.display, keycode, 0) as u32;
            let is_press = event_type == xlib::KeyPress as u8;

            if let Some(input_event) = LinuxInputListener::map_keysym_to_event(keysym, is_press) {
              let _ = callback_data.tx.try_send(input_event);
            }
          }
        }

        xrecord::XRecordFreeData(raw_data);
      }
    }

    // Enable XRecord context (this blocks until disabled)
    let status = xrecord::XRecordEnableContext(
      data_display,
      context,
      Some(xrecord_callback),
      &mut callback_data as *mut CallbackData as *mut i8,
    );

    if status == 0 {
      tracing::error!("XRecordEnableContext failed");
    }

    // Cleanup
    xrecord::XRecordDisableContext(ctrl_display, context);
    xrecord::XRecordFreeContext(ctrl_display, context);
    xlib::XCloseDisplay(data_display);
    xlib::XCloseDisplay(ctrl_display);
  }

  Ok(())
}

pub struct LinuxCursorActuator {
  #[cfg(target_os = "linux")]
  display: Option<*mut xlib::Display>,
}

impl LinuxCursorActuator {
  pub fn new() -> Result<Self> {
    #[cfg(target_os = "linux")]
    {
      // Open X11 display
      let display = unsafe { xlib::XOpenDisplay(std::ptr::null()) };

      if display.is_null() {
        return Err(Error::Platform(
          "Failed to open X11 display. Is DISPLAY set?".to_string(),
        ));
      }

      Ok(Self { display: Some(display) })
    }

    #[cfg(not(target_os = "linux"))]
    Ok(Self {})
  }
}

impl Drop for LinuxCursorActuator {
  fn drop(&mut self) {
    #[cfg(target_os = "linux")]
    if let Some(display) = self.display {
      unsafe {
        xlib::XCloseDisplay(display);
      }
    }
  }
}

impl CursorActuator for LinuxCursorActuator {
  fn move_relative(&mut self, delta: Vector2D) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
      if let Some(display) = self.display {
        unsafe {
          xtest::XTestFakeRelativeMotionEvent(
            display,
            -1, // default screen
            delta.x as i32,
            delta.y as i32,
            0, // delay
          );
          xlib::XFlush(display);
        }

        tracing::trace!("Moved cursor relative ({}, {})", delta.x, delta.y);
        return Ok(());
      } else {
        return Err(Error::Platform("X11 display not initialized".to_string()));
      }
    }

    #[cfg(not(target_os = "linux"))]
    {
      tracing::trace!("move_relative: ({}, {}) - stub", delta.x, delta.y);
      Ok(())
    }
  }

  fn move_absolute(&mut self, position: Vector2D) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
      if let Some(display) = self.display {
        let screen = unsafe { xlib::XDefaultScreen(display) };

        unsafe {
          xtest::XTestFakeMotionEvent(
            display,
            screen,
            position.x as i32,
            position.y as i32,
            0, // delay
          );
          xlib::XFlush(display);
        }

        tracing::trace!("Moved cursor absolute ({}, {})", position.x, position.y);
        return Ok(());
      } else {
        return Err(Error::Platform("X11 display not initialized".to_string()));
      }
    }

    #[cfg(not(target_os = "linux"))]
    {
      tracing::trace!("move_absolute: ({}, {}) - stub", position.x, position.y);
      Ok(())
    }
  }

  fn get_position(&self) -> Result<Vector2D> {
    #[cfg(target_os = "linux")]
    {
      if let Some(display) = self.display {
        let screen = unsafe { xlib::XDefaultScreen(display) };
        let root = unsafe { xlib::XRootWindow(display, screen) };

        let mut root_return: xlib::Window = 0;
        let mut child_return: xlib::Window = 0;
        let mut root_x: i32 = 0;
        let mut root_y: i32 = 0;
        let mut win_x: i32 = 0;
        let mut win_y: i32 = 0;
        let mut mask: u32 = 0;

        unsafe {
          xlib::XQueryPointer(
            display,
            root,
            &mut root_return,
            &mut child_return,
            &mut root_x,
            &mut root_y,
            &mut win_x,
            &mut win_y,
            &mut mask,
          );
        }

        return Ok(Vector2D {
          x: root_x as f64,
          y: root_y as f64,
        });
      } else {
        return Err(Error::Platform("X11 display not initialized".to_string()));
      }
    }

    #[cfg(not(target_os = "linux"))]
    Ok(Vector2D::zero())
  }
}
