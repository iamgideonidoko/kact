use super::{CursorActuator, InputListener, InputOptions, KeyEvent, MouseButton};
use crate::core::types::Vector2D;
use crate::{Error, Result};
use core_foundation::base::{CFRelease, TCFType};
use core_foundation::boolean::CFBoolean;
use core_foundation::dictionary::CFDictionary;
use core_foundation::mach_port::{CFMachPort, CFMachPortRef};
use core_foundation::runloop::{CFRunLoop, kCFRunLoopDefaultMode};
use core_foundation::string::CFString;
use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGEventType, CGMouseButton, EventField};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
use core_graphics::geometry::CGPoint;
use crossbeam_channel::{Receiver, Sender, bounded};
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

type EventRef = *mut c_void;
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
  fn AXIsProcessTrustedWithOptions(options: *const c_void) -> bool;
  fn CGEventTapCreate(
    location: u32,
    placement: u32,
    options: u32,
    mask: u64,
    callback: unsafe extern "C" fn(*mut c_void, u32, EventRef, *mut c_void) -> EventRef,
    data: *mut c_void,
  ) -> CFMachPortRef;
  fn CGEventTapEnable(tap: CFMachPortRef, enabled: bool);
  fn CGEventGetIntegerValueField(event: EventRef, field: u32) -> i64;
  fn CGEventGetFlags(event: EventRef) -> u64;
  fn CGEventKeyboardGetUnicodeString(event: EventRef, max: usize, actual: *mut usize, text: *mut u16);
}

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
  fn TISCopyCurrentKeyboardLayoutInputSource() -> *const c_void;
  fn TISGetInputSourceProperty(source: *const c_void, property: *const c_void) -> *const c_void;
  static kTISPropertyUnicodeKeyLayoutData: *const c_void;
  fn LMGetKbdType() -> u8;
  fn UCKeyTranslate(
    layout: *const c_void,
    key: u16,
    action: u16,
    modifiers: u32,
    keyboard: u32,
    options: u32,
    dead: *mut u32,
    max: u32,
    actual: *mut u32,
    text: *mut u16,
  ) -> i32;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
  fn CFDataGetBytePtr(data: *const c_void) -> *const u8;
}

// Translate without modifiers so Option and Control chords retain their base key.
fn layout_key(code: i64) -> Option<String> {
  unsafe {
    let source = TISCopyCurrentKeyboardLayoutInputSource();
    if source.is_null() {
      return None;
    }
    let data = TISGetInputSourceProperty(source, kTISPropertyUnicodeKeyLayoutData);
    let mut result = None;
    if !data.is_null() {
      let layout = CFDataGetBytePtr(data);
      if !layout.is_null() {
        let mut text = [0_u16; 8];
        let (mut dead, mut length) = (0, 0);
        let status = UCKeyTranslate(
          layout.cast(),
          code as u16,
          0,
          0,
          LMGetKbdType().into(),
          1,
          &mut dead,
          text.len() as u32,
          &mut length,
          text.as_mut_ptr(),
        );
        if status == 0 && length > 0 && length as usize <= text.len() {
          let key = String::from_utf16_lossy(&text[..length as usize]).to_lowercase();
          if key.chars().count() == 1 && !key.chars().any(char::is_control) {
            result = Some(key);
          }
        }
      }
    }
    CFRelease(source);
    result
  }
}

pub fn accessibility_trusted(prompt: bool) -> bool {
  let options =
    CFDictionary::from_CFType_pairs(&[(CFString::new("AXTrustedCheckOptionPrompt"), CFBoolean::from(prompt))]);
  unsafe { AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef().cast()) }
}

struct TapState {
  options: InputOptions,
  tx: Sender<KeyEvent>,
  failed: Arc<AtomicBool>,
  // Keep the original chord through modifier changes, repeats, and deactivation.
  captured: HashMap<i64, KeyEvent>,
}
unsafe extern "C" fn callback(_proxy: *mut c_void, kind: u32, event: EventRef, data: *mut c_void) -> EventRef {
  // A panic may never unwind through the system callback.
  let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    let state = unsafe { &mut *data.cast::<TapState>() };
    if kind == u32::MAX || kind == u32::MAX - 1 {
      state.failed.store(true, Ordering::Release);
      return event;
    }
    // Observe wheel activity without consuming it. This covers web views that
    // fail to send AX layout notifications while preserving native scrolling.
    if kind == 22 {
      if state.options.active.load(Ordering::Acquire) {
        state.options.scroll_activity.store(true, Ordering::Release);
      }
      return event;
    }
    if event.is_null() || !matches!(kind, 10 | 11) {
      return event;
    }
    let code = unsafe { CGEventGetIntegerValueField(event, EventField::KEYBOARD_EVENT_KEYCODE) };
    let pressed = kind == 10;
    let repeat = unsafe { CGEventGetIntegerValueField(event, EventField::KEYBOARD_EVENT_AUTOREPEAT) } != 0;
    let original = state.captured.get(&code).cloned();
    if let Some(mut input) = original {
      if !pressed {
        state.captured.remove(&code);
      } else if !repeat {
        // Duplicate key-down events are possible around event-tap ownership
        // changes. They are not OS autorepeat and must not repeat an action.
        return std::ptr::null_mut();
      }
      input.pressed = pressed;
      input.repeat = repeat;
      if state.tx.try_send(input).is_err() {
        state.failed.store(true, Ordering::Release);
      }
      return std::ptr::null_mut();
    }
    if !pressed || repeat {
      return event;
    }
    if state.options.global_shortcuts.is_empty() && !state.options.active.load(Ordering::Acquire) {
      return event;
    }
    let Some(key) = key_name(code, event) else {
      return event;
    };
    let flags = unsafe { CGEventGetFlags(event) };
    let modifiers = [
      (1 << 18, "ctrl"),
      (1 << 19, "alt"),
      (1 << 17, "shift"),
      (1 << 20, "cmd"),
    ]
    .into_iter()
    .filter(|(mask, _)| flags & mask != 0)
    .map(|(_, name)| name.to_string())
    .collect();
    let input = KeyEvent {
      key,
      modifiers,
      pressed,
      repeat,
    };
    let shortcut = input.shortcut();
    let capture = state.options.global_shortcuts.contains(&shortcut)
      || (state.options.active.load(Ordering::Acquire)
        && (state.options.navigation_keys.contains(&shortcut)
          || state.options.labels_active.load(Ordering::Acquire) && state.options.label_keys.contains(&shortcut)));
    if !capture {
      return event;
    }
    state.captured.insert(code, input.clone());
    if state.tx.try_send(input).is_err() {
      state.failed.store(true, Ordering::Release);
    }
    std::ptr::null_mut()
  }));
  match result {
    Ok(event) => event,
    Err(_) => {
      unsafe { &*data.cast::<TapState>() }
        .failed
        .store(true, Ordering::Release);
      event
    }
  }
}

fn key_name(code: i64, event: EventRef) -> Option<String> {
  let special = match code {
    36 | 76 => "enter",
    48 => "tab",
    49 => "space",
    51 => "backspace",
    53 => "escape",
    117 => "delete",
    123 => "left",
    124 => "right",
    125 => "down",
    126 => "up",
    115 => "home",
    119 => "end",
    116 => "pageup",
    121 => "pagedown",
    122 => "f1",
    120 => "f2",
    99 => "f3",
    118 => "f4",
    96 => "f5",
    97 => "f6",
    98 => "f7",
    100 => "f8",
    101 => "f9",
    109 => "f10",
    103 => "f11",
    111 => "f12",
    105 => "f13",
    107 => "f14",
    113 => "f15",
    106 => "f16",
    64 => "f17",
    79 => "f18",
    80 => "f19",
    90 => "f20",
    _ => "",
  };
  if !special.is_empty() {
    return Some(special.into());
  }
  if let Some(key) = layout_key(code) {
    return Some(key);
  }
  let mut text = [0_u16; 8];
  let mut length = 0;
  unsafe {
    CGEventKeyboardGetUnicodeString(event, text.len(), &mut length, text.as_mut_ptr());
  }
  if length > 0 && length <= text.len() {
    let key = String::from_utf16_lossy(&text[..length]).to_lowercase();
    if key.chars().count() == 1 && !key.chars().any(char::is_control) {
      return Some(key);
    }
  }
  // Control chords can expose control characters rather than printable Unicode.
  let key = match code {
    0 => "a",
    1 => "s",
    2 => "d",
    3 => "f",
    4 => "h",
    5 => "g",
    6 => "z",
    7 => "x",
    8 => "c",
    9 => "v",
    11 => "b",
    12 => "q",
    13 => "w",
    14 => "e",
    15 => "r",
    16 => "y",
    17 => "t",
    18 => "1",
    19 => "2",
    20 => "3",
    21 => "4",
    22 => "6",
    23 => "5",
    24 => "=",
    25 => "9",
    26 => "7",
    27 => "-",
    28 => "8",
    29 => "0",
    30 => "]",
    31 => "o",
    32 => "u",
    33 => "[",
    34 => "i",
    35 => "p",
    37 => "l",
    38 => "j",
    39 => "'",
    40 => "k",
    41 => ";",
    42 => "\\",
    43 => ",",
    44 => "/",
    45 => "n",
    46 => "m",
    47 => ".",
    50 => "`",
    _ => return None,
  };
  Some(key.into())
}

pub struct MacOSInputListener {
  options: InputOptions,
  rx: Receiver<KeyEvent>,
  stop: Arc<AtomicBool>,
  failed: Arc<AtomicBool>,
  thread: Option<JoinHandle<()>>,
}
impl MacOSInputListener {
  pub fn new(options: InputOptions) -> Result<Self> {
    Ok(Self {
      options,
      rx: bounded(1).1,
      stop: Arc::new(AtomicBool::new(false)),
      failed: Arc::new(AtomicBool::new(false)),
      thread: None,
    })
  }
}
impl InputListener for MacOSInputListener {
  fn start(&mut self) -> Result<()> {
    if self.thread.is_some() {
      return Ok(());
    }
    self.stop.store(false, Ordering::Release);
    self.failed.store(false, Ordering::Release);
    let (tx, rx) = bounded(256);
    self.rx = rx;
    let (ready_tx, ready_rx) = bounded(1);
    let stop = self.stop.clone();
    let failed = self.failed.clone();
    let options = self.options.clone();
    self.thread = Some(thread::spawn(move || {
      let mut state = Box::new(TapState {
        options,
        tx,
        failed: failed.clone(),
        captured: HashMap::new(),
      });
      let raw = unsafe {
        CGEventTapCreate(
          1,
          0,
          0,
          (1 << 10) | (1 << 11) | (1 << 22),
          callback,
          (&mut *state as *mut TapState).cast(),
        )
      };
      if raw.is_null() {
        let _ = ready_tx.send(Err(Error::Platform(
          "Cannot capture keyboard. Grant Accessibility and Input Monitoring permission, then restart kact.".into(),
        )));
        return;
      }
      let tap = unsafe { CFMachPort::wrap_under_create_rule(raw) };
      let source = match tap.create_runloop_source(0) {
        Ok(source) => source,
        Err(_) => {
          let _ = ready_tx.send(Err(Error::Platform("Cannot create keyboard run loop source".into())));
          return;
        }
      };
      let run_loop = CFRunLoop::get_current();
      unsafe {
        run_loop.add_source(&source, kCFRunLoopDefaultMode);
        CGEventTapEnable(raw, true);
      }
      let _ = ready_tx.send(Ok(()));
      while !stop.load(Ordering::Acquire) && !failed.load(Ordering::Acquire) {
        unsafe {
          CFRunLoop::run_in_mode(kCFRunLoopDefaultMode, Duration::from_millis(10), false);
        }
      }
      unsafe {
        CGEventTapEnable(raw, false);
        run_loop.remove_source(&source, kCFRunLoopDefaultMode);
      }
    }));
    match ready_rx.recv_timeout(Duration::from_secs(3)) {
      Ok(Ok(())) => Ok(()),
      Ok(Err(error)) => {
        self.stop()?;
        Err(error)
      }
      Err(_) => {
        self.stop()?;
        Err(Error::Platform("Keyboard listener startup timed out".into()))
      }
    }
  }
  fn next_event(&mut self) -> Result<Option<KeyEvent>> {
    if self.failed.load(Ordering::Acquire) {
      return Err(Error::Platform(
        "Keyboard capture stopped or overflowed; release held actions and restart capture".into(),
      ));
    }
    match self.rx.try_recv() {
      Ok(event) => Ok(Some(event)),
      Err(crossbeam_channel::TryRecvError::Empty) => Ok(None),
      Err(_) if self.thread.is_none() => Ok(None),
      Err(_) => Err(Error::Platform("Keyboard listener disconnected".into())),
    }
  }
  fn stop(&mut self) -> Result<()> {
    self.stop.store(true, Ordering::Release);
    if let Some(thread) = self.thread.take() {
      thread
        .join()
        .map_err(|_| Error::Platform("Keyboard listener panicked".into()))?;
    }
    Ok(())
  }
}
impl Drop for MacOSInputListener {
  fn drop(&mut self) {
    let _ = self.stop();
  }
}

pub struct MacOSCursorActuator {
  held: Vec<(MouseButton, Vec<String>)>,
}
impl MacOSCursorActuator {
  pub fn new() -> Result<Self> {
    if !accessibility_trusted(false) {
      return Err(Error::Platform("Grant Accessibility permission to kact or its terminal in System Settings > Privacy & Security > Accessibility".into()));
    }
    Ok(Self { held: Vec::new() })
  }
  fn source() -> Result<CGEventSource> {
    CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
      .map_err(|_| Error::Platform("Cannot create event source".into()))
  }
  fn mouse_event(
    &self,
    button: MouseButton,
    kind: CGEventType,
    count: i64,
    modifiers: &[String],
    point: Vector2D,
  ) -> Result<()> {
    if !matches!(
      kind,
      CGEventType::LeftMouseUp | CGEventType::RightMouseUp | CGEventType::OtherMouseUp
    ) && !accessibility_trusted(false)
    {
      return Err(Error::Platform(
        "Accessibility permission was revoked; enable it before retrying".into(),
      ));
    }
    let event = CGEvent::new_mouse_event(
      Self::source()?,
      kind,
      CGPoint::new(point.x, point.y),
      native_button(button),
    )
    .map_err(|_| Error::Platform("Cannot create mouse event".into()))?;
    event.set_flags(modifier_flags(modifiers)?);
    event.set_integer_value_field(EventField::MOUSE_EVENT_CLICK_STATE, count);
    event.post(CGEventTapLocation::HID);
    Ok(())
  }
}
fn native_button(button: MouseButton) -> CGMouseButton {
  match button {
    MouseButton::Left => CGMouseButton::Left,
    MouseButton::Right => CGMouseButton::Right,
    MouseButton::Middle => CGMouseButton::Center,
  }
}
fn button_event(button: MouseButton, pressed: bool) -> CGEventType {
  match (button, pressed) {
    (MouseButton::Left, true) => CGEventType::LeftMouseDown,
    (MouseButton::Left, false) => CGEventType::LeftMouseUp,
    (MouseButton::Right, true) => CGEventType::RightMouseDown,
    (MouseButton::Right, false) => CGEventType::RightMouseUp,
    (MouseButton::Middle, true) => CGEventType::OtherMouseDown,
    (MouseButton::Middle, false) => CGEventType::OtherMouseUp,
  }
}
fn modifier_flags(modifiers: &[String]) -> Result<CGEventFlags> {
  let mut flags = CGEventFlags::empty();
  for modifier in modifiers {
    flags |= match modifier.as_str() {
      "ctrl" | "control" => CGEventFlags::CGEventFlagControl,
      "alt" | "option" => CGEventFlags::CGEventFlagAlternate,
      "shift" => CGEventFlags::CGEventFlagShift,
      "cmd" | "command" | "super" | "meta" => CGEventFlags::CGEventFlagCommand,
      _ => return Err(Error::Platform(format!("Unknown modifier: {modifier}"))),
    };
  }
  Ok(flags)
}
impl CursorActuator for MacOSCursorActuator {
  fn move_relative(&mut self, delta: Vector2D) -> Result<()> {
    self.move_absolute(self.get_position()?.add(&delta))
  }
  fn move_absolute(&mut self, position: Vector2D) -> Result<()> {
    if !position.x.is_finite() || !position.y.is_finite() {
      return Err(Error::Platform("Cursor coordinates must be finite".into()));
    }
    let (button, modifiers) = self.held.last().cloned().unwrap_or((MouseButton::Left, Vec::new()));
    let kind = if self.held.is_empty() {
      CGEventType::MouseMoved
    } else {
      match button {
        MouseButton::Left => CGEventType::LeftMouseDragged,
        MouseButton::Right => CGEventType::RightMouseDragged,
        MouseButton::Middle => CGEventType::OtherMouseDragged,
      }
    };
    self.mouse_event(button, kind, 0, &modifiers, position)
  }
  fn get_position(&self) -> Result<Vector2D> {
    let event = CGEvent::new(Self::source()?).map_err(|_| Error::Platform("Cannot read cursor position".into()))?;
    let point = event.location();
    Ok(Vector2D { x: point.x, y: point.y })
  }
  fn click(&mut self, button: MouseButton, count: u8, modifiers: &[String]) -> Result<()> {
    if !(1..=3).contains(&count) {
      return Err(Error::Platform("Click count must be 1 through 3".into()));
    }
    if self.held.iter().any(|(held, _)| *held == button) {
      return Err(Error::Platform("Release held button before clicking it".into()));
    }
    let point = self.get_position()?;
    for click in 1..=count {
      self.mouse_event(button, button_event(button, true), click.into(), modifiers, point)?;
      self.held.push((button, modifiers.to_vec()));
      self.mouse_event(button, button_event(button, false), click.into(), modifiers, point)?;
      self.held.retain(|(held, _)| *held != button);
    }
    Ok(())
  }
  fn button_down(&mut self, button: MouseButton, modifiers: &[String]) -> Result<()> {
    if self.held.iter().any(|(held, _)| *held == button) {
      return Ok(());
    }
    self.mouse_event(button, button_event(button, true), 1, modifiers, self.get_position()?)?;
    self.held.push((button, modifiers.to_vec()));
    Ok(())
  }
  fn button_up(&mut self, button: MouseButton, modifiers: &[String]) -> Result<()> {
    let modifiers = if modifiers.is_empty() {
      self
        .held
        .iter()
        .find(|(held, _)| *held == button)
        .map_or(modifiers, |(_, stored)| stored.as_slice())
    } else {
      modifiers
    };
    self.mouse_event(button, button_event(button, false), 1, modifiers, self.get_position()?)?;
    self.held.retain(|(held, _)| *held != button);
    Ok(())
  }
  fn scroll(&mut self, dx: i32, dy: i32) -> Result<()> {
    if !accessibility_trusted(false) {
      return Err(Error::Platform("Accessibility permission was revoked".into()));
    }
    let event = CGEvent::new_scroll_event(Self::source()?, 0, 2, dy, dx, 0)
      .map_err(|_| Error::Platform("Cannot create scroll event".into()))?;
    event.set_flags(CGEventFlags::empty());
    event.post(CGEventTapLocation::HID);
    Ok(())
  }
  fn release_all(&mut self) -> Result<()> {
    let held = self.held.clone();
    let mut error = None;
    for (button, modifiers) in held {
      if let Err(e) = self.button_up(button, &modifiers) {
        error = Some(e);
      }
    }
    error.map_or(Ok(()), Err)
  }
}
impl Drop for MacOSCursorActuator {
  fn drop(&mut self) {
    let _ = self.release_all();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  unsafe extern "C" {
    fn CGEventCreateKeyboardEvent(source: *const c_void, key: u16, pressed: bool) -> EventRef;
    fn CGEventSetIntegerValueField(event: EventRef, field: u32, value: i64);
  }
  #[test]
  fn captured_events_keep_original_chord_through_repeat_and_deactivation() {
    let (tx, rx) = bounded(4);
    let mut state = TapState {
      options: InputOptions {
        global_shortcuts: Vec::new(),
        navigation_keys: Vec::new(),
        label_keys: Vec::new(),
        labels_active: Arc::new(AtomicBool::new(false)),
        active: Arc::new(AtomicBool::new(false)),
        scroll_activity: Arc::new(AtomicBool::new(false)),
      },
      tx,
      failed: Arc::new(AtomicBool::new(false)),
      captured: HashMap::new(),
    };
    state.captured.insert(
      40,
      KeyEvent {
        key: "k".into(),
        modifiers: vec!["ctrl".into()],
        pressed: true,
        repeat: false,
      },
    );
    unsafe {
      let press = CGEventCreateKeyboardEvent(std::ptr::null(), 40, true);
      assert!(!press.is_null());
      CGEventSetIntegerValueField(press, EventField::KEYBOARD_EVENT_AUTOREPEAT, 1);
      assert!(callback(std::ptr::null_mut(), 10, press, (&mut state as *mut TapState).cast()).is_null());
      CFRelease(press.cast());
      let repeated = rx.try_recv().unwrap();
      assert!(repeated.repeat);
      assert!(repeated.pressed);
      assert_eq!(repeated.shortcut(), "ctrl+k");
      let release = CGEventCreateKeyboardEvent(std::ptr::null(), 40, false);
      assert!(!release.is_null());
      assert!(callback(std::ptr::null_mut(), 11, release, (&mut state as *mut TapState).cast()).is_null());
      CFRelease(release.cast());
    }
    let released = rx.try_recv().unwrap();
    assert!(!released.pressed);
    assert_eq!(released.shortcut(), "ctrl+k");
    assert!(state.captured.is_empty());
  }
  #[test]
  fn overflow_reports_failure_instead_of_silently_losing_release() {
    let (tx, _rx) = bounded(0);
    let mut state = TapState {
      options: InputOptions {
        global_shortcuts: Vec::new(),
        navigation_keys: Vec::new(),
        label_keys: Vec::new(),
        labels_active: Arc::new(AtomicBool::new(false)),
        active: Arc::new(AtomicBool::new(false)),
        scroll_activity: Arc::new(AtomicBool::new(false)),
      },
      tx,
      failed: Arc::new(AtomicBool::new(false)),
      captured: HashMap::new(),
    };
    state.captured.insert(
      40,
      KeyEvent {
        key: "k".into(),
        modifiers: vec![],
        pressed: true,
        repeat: false,
      },
    );
    unsafe {
      let release = CGEventCreateKeyboardEvent(std::ptr::null(), 40, false);
      assert!(!release.is_null());
      callback(std::ptr::null_mut(), 11, release, (&mut state as *mut TapState).cast());
      CFRelease(release.cast());
    }
    assert!(state.failed.load(Ordering::Acquire));
  }
  #[test]
  fn wheel_activity_is_passed_through_and_marks_refresh() {
    let (tx, _rx) = bounded(1);
    let scroll_activity = Arc::new(AtomicBool::new(false));
    let mut state = TapState {
      options: InputOptions {
        global_shortcuts: Vec::new(),
        navigation_keys: Vec::new(),
        label_keys: Vec::new(),
        labels_active: Arc::new(AtomicBool::new(true)),
        active: Arc::new(AtomicBool::new(true)),
        scroll_activity: Arc::clone(&scroll_activity),
      },
      tx,
      failed: Arc::new(AtomicBool::new(false)),
      captured: HashMap::new(),
    };
    let event = 1usize as EventRef;
    assert_eq!(
      unsafe { callback(std::ptr::null_mut(), 22, event, (&mut state as *mut TapState).cast()) },
      event
    );
    assert!(scroll_activity.load(Ordering::Acquire));
  }
}
