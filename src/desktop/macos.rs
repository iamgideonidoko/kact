#![allow(deprecated, unexpected_cfgs)]
use super::*;
use cocoa::appkit::{
  NSApp, NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow, NSWindowStyleMask,
};
use cocoa::base::{NO, YES, id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use core_foundation::base::{CFRelease, CFRetain, CFTypeRef, TCFType};
use core_foundation::string::CFString;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::{
  collections::{HashSet, VecDeque},
  marker::PhantomData,
  rc::Rc,
  sync::Once,
  time::{Duration, Instant},
};

struct OverlayData {
  targets: Vec<Target>,
  prefix: String,
  appearance: Appearance,
  screen: Rect,
}
struct Overlay {
  window: id,
  screen: Rect,
}

unsafe fn overlay_data(view: id) -> *mut OverlayData {
  unsafe { (*(*view).get_ivar::<*mut std::ffi::c_void>("kactData")).cast() }
}

/// Owns AppKit objects; intentionally neither Send nor Sync.
pub struct Desktop {
  app: id,
  overlays: Vec<Overlay>,
  _main_thread: PhantomData<Rc<()>>,
}

unsafe fn string(s: &str) -> id {
  unsafe { NSString::alloc(nil).init_str(s).autorelease() }
}
unsafe fn color(hex: &str, alpha: f64) -> id {
  let rgb = u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0xFFFFFF);
  unsafe {
    msg_send![class!(NSColor), colorWithSRGBRed: ((rgb >> 16) & 255) as f64 / 255.0 green: ((rgb >> 8) & 255) as f64 / 255.0 blue: (rgb & 255) as f64 / 255.0 alpha: alpha]
  }
}

extern "C" fn can_become_key(_: &Object, _: Sel) -> cocoa::base::BOOL {
  NO
}
extern "C" fn dealloc(this: &mut Object, _: Sel) {
  unsafe {
    let data = (*this.get_ivar::<*mut std::ffi::c_void>("kactData")).cast::<OverlayData>();
    if !data.is_null() {
      drop(Box::from_raw(data));
      this.set_ivar("kactData", std::ptr::null_mut::<std::ffi::c_void>());
    }
    let _: () = msg_send![super(this, class!(NSView)), dealloc];
  }
}
extern "C" fn draw(this: &Object, _: Sel, _: NSRect) {
  unsafe {
    let Some(data) = overlay_data(this as *const Object as id).as_ref() else {
      return;
    };
    let a = &data.appearance;
    for target in &data.targets {
      if !label_matches(&target.label, &data.prefix) {
        continue;
      }
      let b = target.bounds;
      let cell = NSRect::new(
        NSPoint::new(
          b.x - data.screen.x,
          data.screen.height - (b.y - data.screen.y) - b.height,
        ),
        NSSize::new(b.width, b.height),
      );
      if a.grid_lines {
        let c = color(&a.foreground, 0.22);
        let _: () = msg_send![c, setStroke];
        let path: id = msg_send![class!(NSBezierPath), bezierPathWithRect: cell];
        let _: () = msg_send![path, setLineWidth: 0.5f64];
        let _: () = msg_send![path, stroke];
      }
      // An empty label hides text while preserving the filtered grid geometry.
      if target.label.is_empty() {
        continue;
      }
      let text = string(&target.label);
      let font: id = msg_send![class!(NSFont), boldSystemFontOfSize: a.font_size];
      let attributes: id = msg_send![class!(NSMutableDictionary), dictionary];
      let _: () = msg_send![attributes, setObject: font forKey: string("NSFont")];
      let foreground = color(&a.foreground, 1.0);
      let _: () = msg_send![attributes, setObject: foreground forKey: string("NSColor")];
      let label: id = msg_send![class!(NSMutableAttributedString), alloc];
      let label: id = msg_send![label, initWithString: text attributes: attributes];
      if !data.prefix.is_empty() {
        let prefix_attributes: id =
          msg_send![class!(NSDictionary), dictionaryWithObject: color(&a.highlight, 1.0) forKey: string("NSColor")];
        let range = cocoa::foundation::NSRange::new(0, data.prefix.encode_utf16().count() as u64);
        let _: () = msg_send![label, addAttributes: prefix_attributes range: range];
      }
      let size: NSSize = msg_send![label, size];
      let origin = NSPoint::new(
        cell.origin.x + (cell.size.width - size.width) / 2.0,
        cell.origin.y + (cell.size.height - size.height) / 2.0,
      );
      let plate = NSRect::new(
        NSPoint::new(origin.x - 4.0, origin.y - 2.0),
        NSSize::new(size.width + 8.0, size.height + 4.0),
      );
      let background = color(&a.background, a.opacity);
      let _: () = msg_send![background, setFill];
      let path: id = msg_send![class!(NSBezierPath), bezierPathWithRoundedRect: plate xRadius: 3.0f64 yRadius: 3.0f64];
      let _: () = msg_send![path, fill];
      let _: () = msg_send![label, drawAtPoint: origin];
      let _: () = msg_send![label, release];
    }
  }
}

fn register_classes() {
  static ONCE: Once = Once::new();
  ONCE.call_once(|| unsafe {
    let mut view = ClassDecl::new("KactOverlayView", class!(NSView)).unwrap();
    view.add_ivar::<*mut std::ffi::c_void>("kactData");
    view.add_method(sel!(drawRect:), draw as extern "C" fn(&Object, Sel, NSRect));
    view.add_method(sel!(dealloc), dealloc as extern "C" fn(&mut Object, Sel));
    view.register();
    let mut panel = ClassDecl::new("KactOverlayPanel", class!(NSPanel)).unwrap();
    panel.add_method(
      sel!(canBecomeKeyWindow),
      can_become_key as extern "C" fn(&Object, Sel) -> cocoa::base::BOOL,
    );
    panel.add_method(
      sel!(canBecomeMainWindow),
      can_become_key as extern "C" fn(&Object, Sel) -> cocoa::base::BOOL,
    );
    panel.register();
  });
}

impl Desktop {
  pub fn new() -> anyhow::Result<Self> {
    unsafe {
      let main: cocoa::base::BOOL = msg_send![class!(NSThread), isMainThread];
      anyhow::ensure!(main == YES, "Desktop must be initialized on the main thread");
      register_classes();
      let pool = NSAutoreleasePool::new(nil);
      let app = NSApp();
      app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory);
      app.finishLaunching();
      pool.drain();
      Ok(Self {
        app,
        overlays: Vec::new(),
        _main_thread: PhantomData,
      })
    }
  }

  pub fn screens(&self) -> anyhow::Result<Vec<Rect>> {
    unsafe {
      let pool = NSAutoreleasePool::new(nil);
      let screens: id = msg_send![class!(NSScreen), screens];
      let count: usize = msg_send![screens, count];
      if count == 0 {
        pool.drain();
        anyhow::bail!("No displays available");
      }
      let primary: id = msg_send![screens, objectAtIndex: 0usize];
      let primary_frame: NSRect = msg_send![primary, frame];
      let top = primary_frame.origin.y + primary_frame.size.height;
      let result = (0..count)
        .map(|i| {
          let screen: id = msg_send![screens, objectAtIndex: i];
          let frame: NSRect = msg_send![screen, frame];
          Rect {
            x: frame.origin.x,
            y: top - frame.origin.y - frame.size.height,
            width: frame.size.width,
            height: frame.size.height,
          }
        })
        .collect();
      pool.drain();
      Ok(result)
    }
  }

  pub fn show(&mut self, targets: &[Target], prefix: &str, appearance: &Appearance) -> anyhow::Result<()> {
    let screens = self.screens()?;
    anyhow::ensure!(
      appearance.font_size.is_finite() && (6.0..=96.0).contains(&appearance.font_size),
      "Overlay font size must be between 6 and 96"
    );
    anyhow::ensure!(
      (0.0..=1.0).contains(&appearance.opacity),
      "Overlay opacity must be between 0 and 1"
    );
    for color in [&appearance.foreground, &appearance.background, &appearance.highlight] {
      anyhow::ensure!(
        color.len() == 7 && color.starts_with('#') && color[1..].bytes().all(|b| b.is_ascii_hexdigit()),
        "Overlay colors must use #RRGGBB"
      );
    }
    if self.overlays.len() == screens.len()
      && self
        .overlays
        .iter()
        .zip(&screens)
        .all(|(overlay, screen)| overlay.screen == *screen)
    {
      for overlay in &mut self.overlays {
        let view: id = unsafe { msg_send![overlay.window, contentView] };
        let data = unsafe { overlay_data(view).as_mut() };
        let Some(data) = data else {
          continue;
        };
        data.targets = targets
          .iter()
          .filter(|target| intersects(target.bounds, data.screen))
          .cloned()
          .collect();
        data.prefix = prefix.to_owned();
        data.appearance = appearance.clone();
        unsafe {
          let _: () = msg_send![view, setNeedsDisplay: YES];
        }
      }
      return Ok(());
    }
    self.hide();
    let top = screens[0].height;
    unsafe {
      let pool = NSAutoreleasePool::new(nil);
      for screen in screens {
        let frame = NSRect::new(
          NSPoint::new(screen.x, top - screen.y - screen.height),
          NSSize::new(screen.width, screen.height),
        );
        let window: id = msg_send![Class::get("KactOverlayPanel").unwrap(), alloc];
        let window = window.initWithContentRect_styleMask_backing_defer_(
          frame,
          NSWindowStyleMask::NSBorderlessWindowMask | NSWindowStyleMask::from_bits_retain(1 << 7),
          NSBackingStoreType::NSBackingStoreBuffered,
          NO,
        );
        let _: () = msg_send![window, setReleasedWhenClosed: NO];
        let _: () = msg_send![window, setOpaque: NO];
        let _: () = msg_send![window, setBackgroundColor: color("#000000", 0.0)];
        let _: () = msg_send![window, setHasShadow: NO];
        let _: () = msg_send![window, setIgnoresMouseEvents: YES];
        let _: () = msg_send![window, setHidesOnDeactivate: NO];
        let _: () = msg_send![window, setLevel: 25isize];
        let _: () = msg_send![window, setCollectionBehavior: (1usize | 16 | 256)];
        let data = Box::into_raw(Box::new(OverlayData {
          targets: targets
            .iter()
            .filter(|t| intersects(t.bounds, screen))
            .cloned()
            .collect(),
          prefix: prefix.to_owned(),
          appearance: appearance.clone(),
          screen,
        }));
        let view: id = msg_send![Class::get("KactOverlayView").unwrap(), alloc];
        let view: id = msg_send![view, initWithFrame: NSRect::new(NSPoint::new(0.0, 0.0), frame.size)];
        (*view).set_ivar("kactData", data.cast::<std::ffi::c_void>());
        let _: () = msg_send![window, setContentView: view];
        let _: () = msg_send![view, release];
        let _: () = msg_send![window, orderFrontRegardless];
        self.overlays.push(Overlay { window, screen });
      }
      pool.drain();
    }
    Ok(())
  }

  pub fn hide(&mut self) {
    for overlay in self.overlays.drain(..) {
      unsafe {
        let _: () = msg_send![overlay.window, orderOut: nil];
        let _: () = msg_send![overlay.window, close];
        let _: () = msg_send![overlay.window, release];
      }
    }
  }
  pub fn pump(&mut self) {
    unsafe {
      let pool = NSAutoreleasePool::new(nil);
      for _ in 0..64 {
        let deadline: id = msg_send![class!(NSDate), distantPast];
        let event: id = msg_send![self.app, nextEventMatchingMask: usize::MAX untilDate: deadline inMode: string("kCFRunLoopDefaultMode") dequeue: YES];
        if event == nil {
          break;
        }
        let _: () = msg_send![self.app, sendEvent: event];
      }
      let _: () = msg_send![self.app, updateWindows];
      pool.drain();
    }
  }
  /// Identifies the foreground process, focused window, and its current geometry.
  pub fn focus_token(&self) -> anyhow::Result<String> {
    unsafe {
      let system = OwnedCf(AXUIElementCreateSystemWide());
      AXUIElementSetMessagingTimeout(system.0, 0.05);
      let app = attribute(system.0, "AXFocusedApplication")
        .ok_or_else(|| anyhow::anyhow!("Cannot read focused application"))?;
      let mut pid = 0i32;
      anyhow::ensure!(AXUIElementGetPid(app.0, &mut pid) == 0, "Cannot read focused process");
      let window = attribute(app.0, "AXFocusedWindow").unwrap_or(app);
      let hash = core_foundation::base::CFHash(window.0);
      Ok(format!("{pid}:{hash}:{:?}", read_rect(window.0)))
    }
  }
  pub fn elements(&self) -> anyhow::Result<Vec<Rect>> {
    discover_elements()
  }
}
impl Drop for Desktop {
  fn drop(&mut self) {
    self.hide();
  }
}

fn label_matches(label: &str, prefix: &str) -> bool {
  label.is_empty() || label.starts_with(prefix)
}

fn clipped(a: Rect, b: Rect) -> Option<Rect> {
  let x = a.x.max(b.x);
  let y = a.y.max(b.y);
  let width = (a.x + a.width).min(b.x + b.width) - x;
  let height = (a.y + a.height).min(b.y + b.height) - y;
  (width > 1.0 && height > 1.0).then_some(Rect { x, y, width, height })
}

fn intersects(a: Rect, b: Rect) -> bool {
  a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
  fn AXUIElementCreateSystemWide() -> CFTypeRef;
  fn AXUIElementGetPid(element: CFTypeRef, pid: *mut i32) -> i32;
  fn AXUIElementCopyAttributeValue(
    element: CFTypeRef,
    attribute: core_foundation::string::CFStringRef,
    result: *mut CFTypeRef,
  ) -> i32;
  fn AXUIElementCopyAttributeValues(
    element: CFTypeRef,
    attribute: core_foundation::string::CFStringRef,
    index: isize,
    max_values: isize,
    result: *mut CFTypeRef,
  ) -> i32;
  fn AXUIElementSetMessagingTimeout(element: CFTypeRef, timeout: f32) -> i32;
  fn AXValueGetValue(value: CFTypeRef, kind: u32, out: *mut std::ffi::c_void) -> bool;
  fn AXValueGetTypeID() -> usize;
  fn AXIsProcessTrusted() -> bool;
}
struct OwnedCf(CFTypeRef);
impl Drop for OwnedCf {
  fn drop(&mut self) {
    unsafe {
      CFRelease(self.0);
    }
  }
}
fn attribute(element: CFTypeRef, name: &str) -> Option<OwnedCf> {
  unsafe {
    let name = CFString::new(name);
    let mut value = std::ptr::null();
    if AXUIElementCopyAttributeValue(element, name.as_concrete_TypeRef(), &mut value) == 0 && !value.is_null() {
      Some(OwnedCf(value))
    } else {
      None
    }
  }
}
fn read_rect(element: CFTypeRef) -> Option<Rect> {
  use core_foundation::base::CFGetTypeID;
  let position = attribute(element, "AXPosition")?;
  let size = attribute(element, "AXSize")?;
  let mut point = core_graphics::geometry::CGPoint::new(0.0, 0.0);
  let mut dimensions = core_graphics::geometry::CGSize::new(0.0, 0.0);
  unsafe {
    if CFGetTypeID(position.0) != AXValueGetTypeID()
      || CFGetTypeID(size.0) != AXValueGetTypeID()
      || !AXValueGetValue(
        position.0,
        1,
        (&mut point as *mut core_graphics::geometry::CGPoint).cast(),
      )
      || !AXValueGetValue(
        size.0,
        2,
        (&mut dimensions as *mut core_graphics::geometry::CGSize).cast(),
      )
    {
      return None;
    }
  }
  if dimensions.width > 1.0
    && dimensions.height > 1.0
    && [point.x, point.y, dimensions.width, dimensions.height]
      .iter()
      .all(|n| n.is_finite())
  {
    Some(Rect {
      x: point.x,
      y: point.y,
      width: dimensions.width,
      height: dimensions.height,
    })
  } else {
    None
  }
}

fn discover_elements() -> anyhow::Result<Vec<Rect>> {
  use core_foundation::{
    array::{CFArrayGetCount, CFArrayGetValueAtIndex},
    base::CFGetTypeID,
  };
  unsafe {
    anyhow::ensure!(
      AXIsProcessTrusted(),
      "Accessibility access required: System Settings > Privacy & Security > Accessibility"
    );
    let system = OwnedCf(AXUIElementCreateSystemWide());
    AXUIElementSetMessagingTimeout(system.0, 0.05);
    let app =
      attribute(system.0, "AXFocusedApplication").ok_or_else(|| anyhow::anyhow!("Cannot read focused application"))?;
    let root = attribute(app.0, "AXFocusedWindow").unwrap_or(app);
    let started = Instant::now();
    let budget = Duration::from_millis(350);
    let root_bounds = read_rect(root.0);
    let mut queue = VecDeque::from([(root, 0usize, root_bounds)]);
    let mut visited = 0;
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    while let Some((node, depth, mut clip)) = queue.pop_front() {
      if visited >= 4000 || started.elapsed() >= budget {
        break;
      }
      visited += 1;
      let role = attribute(node.0, "AXRole").and_then(|v| {
        (CFGetTypeID(v.0) == core_foundation::string::CFStringGetTypeID())
          .then(|| CFString::wrap_under_get_rule(v.0.cast()).to_string())
      });
      let target = matches!(
        role.as_deref(),
        Some(
          "AXButton"
            | "AXCheckBox"
            | "AXRadioButton"
            | "AXLink"
            | "AXTextField"
            | "AXTextArea"
            | "AXComboBox"
            | "AXPopUpButton"
            | "AXSlider"
            | "AXTab"
            | "AXMenuItem"
            | "AXDisclosureTriangle"
            | "AXImage"
            | "AXCell"
        )
      );
      let hidden = attribute(node.0, "AXHidden").is_some_and(|value| {
        CFGetTypeID(value.0) == core_foundation::boolean::CFBooleanGetTypeID()
          && value.0 == core_foundation::boolean::kCFBooleanTrue.cast()
      });
      if hidden {
        continue;
      }
      let disabled = attribute(node.0, "AXEnabled").is_some_and(|value| {
        CFGetTypeID(value.0) == core_foundation::boolean::CFBooleanGetTypeID()
          && value.0 == core_foundation::boolean::kCFBooleanFalse.cast()
      });
      if matches!(role.as_deref(), Some("AXScrollArea" | "AXWindow" | "AXSheet"))
        && let Some(bounds) = read_rect(node.0)
      {
        if let Some(parent) = clip {
          clip = clipped(bounds, parent);
          if clip.is_none() {
            continue;
          }
        } else {
          clip = Some(bounds);
        }
      }
      if target
        && !disabled
        && let Some(bounds) = read_rect(node.0)
        && let Some(rect) = clip.map_or(Some(bounds), |clip| clipped(bounds, clip))
        && seen.insert((rect.x as i64, rect.y as i64, rect.width as i64, rect.height as i64))
      {
        result.push(rect);
      }
      if depth >= 32 || started.elapsed() >= budget {
        continue;
      }
      let name = CFString::new("AXChildren");
      let mut children = std::ptr::null();
      let remaining = 4000usize.saturating_sub(visited + queue.len()).min(512);
      if remaining > 0
        && AXUIElementCopyAttributeValues(node.0, name.as_concrete_TypeRef(), 0, remaining as isize, &mut children) == 0
        && !children.is_null()
      {
        let children = OwnedCf(children);
        for i in 0..CFArrayGetCount(children.0.cast()) {
          let child = CFArrayGetValueAtIndex(children.0.cast(), i);
          if !child.is_null() {
            queue.push_back((OwnedCf(CFRetain(child)), depth + 1, clip));
          }
        }
      }
    }
    Ok(result)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn hidden_labels_keep_filtered_geometry_and_scrolled_targets_are_clipped() {
    assert!(label_matches("", "a"));
    assert!(label_matches("ab", "a"));
    assert!(!label_matches("bb", "a"));
    let viewport = Rect {
      x: 100.0,
      y: 100.0,
      width: 200.0,
      height: 100.0,
    };
    assert_eq!(
      clipped(
        Rect {
          x: 110.0,
          y: 50.0,
          width: 20.0,
          height: 80.0
        },
        viewport
      ),
      Some(Rect {
        x: 110.0,
        y: 100.0,
        width: 20.0,
        height: 30.0
      })
    );
    assert_eq!(
      clipped(
        Rect {
          x: 110.0,
          y: 50.0,
          width: 20.0,
          height: 30.0
        },
        viewport
      ),
      None
    );
  }
  #[test]
  fn display_intersection_handles_negative_origins_and_edges() {
    let screen = Rect {
      x: -1920.0,
      y: -200.0,
      width: 1920.0,
      height: 1080.0,
    };
    assert!(intersects(
      Rect {
        x: -20.0,
        y: 10.0,
        width: 40.0,
        height: 40.0
      },
      screen
    ));
    assert!(!intersects(
      Rect {
        x: 0.0,
        y: 10.0,
        width: 40.0,
        height: 40.0
      },
      screen
    ));
    assert!(!intersects(
      Rect {
        x: -20.0,
        y: 880.0,
        width: 40.0,
        height: 40.0
      },
      screen
    ));
  }
}
