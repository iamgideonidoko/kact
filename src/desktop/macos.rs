#![allow(deprecated, unexpected_cfgs)]
use super::*;
mod visual;
use cocoa::appkit::{
  NSApp, NSApplication, NSApplicationActivationPolicy, NSBackingStoreType, NSWindow, NSWindowStyleMask,
};
use cocoa::base::{NO, YES, id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString};
use core_foundation::base::{CFEqual, CFRelease, CFRetain, CFTypeRef, TCFType};
use core_foundation::string::CFString;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc::{class, msg_send, sel, sel_impl};
use std::{
  collections::{BTreeMap, HashSet, VecDeque},
  marker::PhantomData,
  rc::Rc,
  sync::{
    Once,
    atomic::{AtomicU64, Ordering},
  },
  time::{Duration, Instant},
};

struct OverlayData {
  targets: Vec<Target>,
  visual_bounds: Vec<Rect>,
  refreshing: bool,
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
  elements: Vec<Element>,
  manual_accessibility_pids: HashSet<i32>,
  enhanced_user_interface_pids: HashSet<i32>,
  visual_fallback: bool,
  semantic_snapshot: Option<VisualSnapshot>,
  visual_snapshot: Option<VisualSnapshot>,
  last_scan: Option<ScanInfo>,
  element_observer: Option<ElementObserver>,
  element_dirty: Box<ElementDirty>,
  observed_generation: u64,
  _main_thread: PhantomData<Rc<()>>,
}

/// Shared with the AX callback. A generation, rather than a queue of AX
/// events, coalesces notification storms without retaining app UI objects.
struct ElementDirty(AtomicU64);

impl ElementDirty {
  fn generation(&self) -> u64 {
    self.0.load(Ordering::Acquire)
  }

  fn mark(&self) {
    self.0.fetch_add(1, Ordering::Release);
  }
}

struct ElementObserver {
  _observer: OwnedCf,
  source: CFTypeRef,
  run_loop: CFTypeRef,
  pid: i32,
  window_hash: usize,
}

impl Drop for ElementObserver {
  fn drop(&mut self) {
    unsafe { CFRunLoopRemoveSource(self.run_loop, self.source, kCFRunLoopCommonModes) }
  }
}

extern "C" fn accessibility_changed(
  _: CFTypeRef,
  _: CFTypeRef,
  _: core_foundation::string::CFStringRef,
  context: *mut std::ffi::c_void,
) {
  if let Some(dirty) = unsafe { context.cast::<ElementDirty>().as_ref() } {
    dirty.mark();
  }
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

fn label_origin(cell: NSRect, size: NSSize, position: LabelPosition) -> NSPoint {
  let center = || {
    NSPoint::new(
      cell.origin.x + (cell.size.width - size.width) / 2.0,
      cell.origin.y + (cell.size.height - size.height) / 2.0,
    )
  };
  if position == LabelPosition::Center || cell.size.width < size.width + 8.0 || cell.size.height < size.height + 4.0 {
    return center();
  }
  match position {
    LabelPosition::Center => center(),
    LabelPosition::Top => NSPoint::new(
      cell.origin.x + (cell.size.width - size.width) / 2.0,
      cell.origin.y + cell.size.height - size.height - 2.0,
    ),
    LabelPosition::Right => NSPoint::new(
      cell.origin.x + cell.size.width - size.width - 4.0,
      cell.origin.y + (cell.size.height - size.height) / 2.0,
    ),
    LabelPosition::Bottom => NSPoint::new(
      cell.origin.x + (cell.size.width - size.width) / 2.0,
      cell.origin.y + 2.0,
    ),
    LabelPosition::Left => NSPoint::new(
      cell.origin.x + 4.0,
      cell.origin.y + (cell.size.height - size.height) / 2.0,
    ),
    LabelPosition::TopLeft => NSPoint::new(
      cell.origin.x + 4.0,
      cell.origin.y + cell.size.height - size.height - 2.0,
    ),
    LabelPosition::TopRight => NSPoint::new(
      cell.origin.x + cell.size.width - size.width - 4.0,
      cell.origin.y + cell.size.height - size.height - 2.0,
    ),
    LabelPosition::BottomLeft => NSPoint::new(cell.origin.x + 4.0, cell.origin.y + 2.0),
    LabelPosition::BottomRight => NSPoint::new(cell.origin.x + cell.size.width - size.width - 4.0, cell.origin.y + 2.0),
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
      if target.focused {
        let c = color(&a.highlight, 0.95);
        let _: () = msg_send![c, setStroke];
        let path: id = msg_send![class!(NSBezierPath), bezierPathWithRoundedRect: cell xRadius: 4.0f64 yRadius: 4.0f64];
        let _: () = msg_send![path, setLineWidth: 3.0f64];
        let _: () = msg_send![path, stroke];
      }
      // An empty label hides text while preserving the filtered grid geometry.
      if target.label.is_empty() {
        continue;
      }
      let visual = data.visual_bounds.contains(&target.bounds);
      let displayed_label = if visual {
        format!("V {}", target.label)
      } else {
        target.label.clone()
      };
      let text = string(&displayed_label);
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
        let offset = u64::from(visual) * 2;
        let range = cocoa::foundation::NSRange::new(offset, data.prefix.encode_utf16().count() as u64);
        let _: () = msg_send![label, addAttributes: prefix_attributes range: range];
      }
      let size: NSSize = msg_send![label, size];
      let origin = label_origin(cell, size, a.label_position);
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
    if data.refreshing {
      let text = string("Refreshing targets…");
      let font: id = msg_send![class!(NSFont), boldSystemFontOfSize: a.font_size];
      let attributes: id = msg_send![class!(NSDictionary), dictionaryWithObject: font forKey: string("NSFont")];
      let label: id = msg_send![class!(NSAttributedString), alloc];
      let label: id = msg_send![label, initWithString: text attributes: attributes];
      let size: NSSize = msg_send![label, size];
      let point = NSPoint::new(
        (data.screen.width - size.width) / 2.0,
        (data.screen.height - size.height) / 2.0,
      );
      let _: () = msg_send![label, drawAtPoint: point];
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
        elements: Vec::new(),
        manual_accessibility_pids: HashSet::new(),
        enhanced_user_interface_pids: HashSet::new(),
        visual_fallback: false,
        semantic_snapshot: None,
        visual_snapshot: None,
        last_scan: None,
        element_observer: None,
        element_dirty: Box::new(ElementDirty(AtomicU64::new(0))),
        observed_generation: 0,
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
        data.visual_bounds = if appearance.visual_targets {
          self
            .elements
            .iter()
            .filter(|element| matches!(element.source, TargetSource::Visual))
            .map(|element| element.bounds)
            .collect()
        } else {
          vec![]
        };
        data.prefix = prefix.to_owned();
        data.appearance = appearance.clone();
        data.refreshing = appearance.refreshing;
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
          visual_bounds: if appearance.visual_targets {
            self
              .elements
              .iter()
              .filter(|element| matches!(element.source, TargetSource::Visual))
              .map(|element| element.bounds)
              .collect()
          } else {
            vec![]
          },
          refreshing: appearance.refreshing,
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

  /// Begins observing current focused app/window. Notifications are advisory:
  /// apps may reject names, so runtime must retain bounded polling fallback.
  pub fn observe_element_changes(&mut self) -> anyhow::Result<()> {
    unsafe {
      let system = OwnedCf(AXUIElementCreateSystemWide());
      AXUIElementSetMessagingTimeout(system.0, 0.05);
      let Some(app) = focused_application(system.0) else {
        self.element_observer = None;
        return Ok(());
      };
      let mut pid = 0i32;
      if AXUIElementGetPid(app.0, &mut pid) != 0 {
        self.element_observer = None;
        return Ok(());
      }
      let window = attribute(app.0, "AXFocusedWindow").unwrap_or_else(|| OwnedCf(CFRetain(app.0)));
      let window_hash = core_foundation::base::CFHash(window.0);
      if self
        .element_observer
        .as_ref()
        .is_some_and(|observer| observer.pid == pid && observer.window_hash == window_hash)
      {
        return Ok(());
      }
      self.element_observer = create_element_observer(pid, app.0, window.0, self.element_dirty.as_ref());
      self.observed_generation = self.element_dirty.generation();
    }
    Ok(())
  }

  /// Returns one coalesced dirty signal since prior call. Callback retains no
  /// AX object, keeping notification handling constant-time and bounded.
  pub fn take_element_refresh_requested(&mut self) -> bool {
    let generation = self.element_dirty.generation();
    let changed = generation != self.observed_generation;
    self.observed_generation = generation;
    changed
  }

  pub fn stop_observing_element_changes(&mut self) {
    self.element_observer = None;
    self.observed_generation = self.element_dirty.generation();
  }

  /// Identifies the foreground process, focused window, and its current geometry.
  pub fn focus_token(&self) -> anyhow::Result<String> {
    unsafe {
      let system = OwnedCf(AXUIElementCreateSystemWide());
      AXUIElementSetMessagingTimeout(system.0, 0.05);
      let app = focused_application(system.0).ok_or_else(|| anyhow::anyhow!("Cannot read focused application"))?;
      let mut pid = 0i32;
      anyhow::ensure!(AXUIElementGetPid(app.0, &mut pid) == 0, "Cannot read focused process");
      let window = attribute(app.0, "AXFocusedWindow").unwrap_or(app);
      let hash = core_foundation::base::CFHash(window.0);
      Ok(format!("{pid}:{hash}:{:?}", read_rect(window.0)))
    }
  }
  pub fn elements(&mut self) -> anyhow::Result<Vec<Rect>> {
    self.refresh_elements(None)
  }

  /// Enables the app's explicit enhanced accessibility tree once for the
  /// foreground process. This is opt-in because some apps change their UI
  /// while an assistive client is attached.
  pub fn set_enhanced_user_interface(&mut self, enabled: bool) -> anyhow::Result<()> {
    self.set_enhanced_user_interface_for(enabled, None)
  }

  pub fn set_enhanced_user_interface_for(&mut self, enabled: bool, pid: Option<i32>) -> anyhow::Result<()> {
    if !enabled {
      return Ok(());
    }
    unsafe {
      anyhow::ensure!(
        AXIsProcessTrusted(),
        "Accessibility access required: System Settings > Privacy & Security > Accessibility"
      );
      let system = OwnedCf(AXUIElementCreateSystemWide());
      AXUIElementSetMessagingTimeout(system.0, 0.05);
      let app = match pid {
        Some(pid) => OwnedCf(AXUIElementCreateApplication(pid)),
        None => focused_application(system.0).ok_or_else(|| anyhow::anyhow!("Cannot read focused application"))?,
      };
      let mut pid = 0i32;
      anyhow::ensure!(AXUIElementGetPid(app.0, &mut pid) == 0, "Cannot read focused process");
      if !self.enhanced_user_interface_pids.contains(&pid) && set_bool_attribute(app.0, "AXEnhancedUserInterface", true)
      {
        self.enhanced_user_interface_pids.insert(pid);
      }
    }
    Ok(())
  }

  /// Enables pixel-derived OCR targets only after a skeletal AX scan.
  pub fn set_visual_fallback(&mut self, enabled: bool) {
    self.visual_fallback = enabled;
  }

  /// Filters the cached element snapshot; it never traverses accessibility.
  pub fn matching_elements(&self, query: &str) -> Vec<Rect> {
    self
      .elements
      .iter()
      .filter(|target| matches_filter(&target.text, &target.role, query))
      .map(|target| target.bounds)
      .collect()
  }

  fn refresh_elements(&mut self, pid: Option<i32>) -> anyhow::Result<Vec<Rect>> {
    let mut scan = discover_elements(pid)?;
    // Chromium/Electron can expose browser chrome before their web viewport is
    // hydrated. AXManualAccessibility requests that tree once per process; it
    // is narrower than enhanced UI/screen-reader emulation.
    if scan.skeletal() {
      let enabled = bool_attribute(scan.app.0, "AXManualAccessibility") == Some(true)
        || unsafe { set_bool_attribute(scan.app.0, "AXManualAccessibility", true) };
      if enabled {
        self.manual_accessibility_pids.insert(scan.pid);
      } else {
        self.manual_accessibility_pids.remove(&scan.pid);
      }
      // Electron may publish titlebar controls before asynchronously hydrating
      // its useful tree. Prefer those semantic targets (for example Spotify's
      // playback controls) before using pixel-derived OCR.
      // Include discovery cost in the hydration deadline. A rejected enabling
      // request must not incur sleeps on every activation of a native app.
      let deadline = Instant::now() + Duration::from_millis(600);
      while (enabled || self.enhanced_user_interface_pids.contains(&scan.pid))
        && scan.skeletal()
        && Instant::now() < deadline
      {
        std::thread::sleep(Duration::from_millis(50).min(deadline.saturating_duration_since(Instant::now())));
        scan = discover_elements(Some(scan.pid))?;
      }
    }
    if let Some(window) = scan.info.window_bounds {
      let stable = stabilize_visual_targets(
        &mut self.semantic_snapshot,
        scan.pid,
        stable_bounds(window),
        scan.targets.iter().map(|target| target.bounds).collect(),
      );
      for (target, bounds) in scan.targets.iter_mut().zip(stable) {
        target.bounds = bounds;
      }
    } else {
      self.semantic_snapshot = None;
    }
    if needs_visual_fallback(self.visual_fallback, scan.skeletal(), scan.info.truncated) {
      let window = scan
        .info
        .window_bounds
        .ok_or_else(|| anyhow::anyhow!("visual fallback requires bounds for focused window"))?;
      match visual_targets_after_warmup(window) {
        Ok(visual_targets) => {
          let visual_targets = stabilize_visual_targets(
            &mut self.visual_snapshot,
            scan.pid,
            stable_bounds(window),
            visual_targets
              .into_iter()
              .map(|target| stable_bounds(target.bounds))
              .collect(),
          );
          merge_visual_targets(&mut scan.targets, visual_targets);
        }
        // A usable semantic tree must not become unavailable because optional
        // Screen Recording access is denied.
        Err(error) if !scan.skeletal() => {
          tracing::debug!(%error, "Visual supplement unavailable; using accessibility targets")
        }
        Err(error) => return Err(error),
      }
    } else {
      self.visual_snapshot = None;
    }
    scan.info.manual_accessibility_enabled = self.manual_accessibility_pids.contains(&scan.pid);
    // Always publish this scan's targets. The previous snapshot can belong to
    // a different process/window and cannot safely fill a truncated scan.
    self.last_scan = Some(scan.info);
    self.elements = scan.targets;
    Ok(self.elements.iter().map(|target| target.bounds).collect())
  }

  /// Performs a semantic activation when the exact displayed target still exists.
  /// Returns false when pointer injection remains required.
  pub fn press_element(&self, bounds: Rect) -> bool {
    self
      .elements
      .iter()
      .find(|target| target.bounds == bounds)
      .and_then(|target| target.node.as_ref())
      .is_some_and(|node| unsafe {
        AXUIElementPerformAction(node.0, CFString::new("AXPress").as_concrete_TypeRef()) == 0
      })
  }

  pub fn inspect_elements(&mut self, show_text: bool, pid: Option<i32>) -> anyhow::Result<serde_json::Value> {
    let started = Instant::now();
    self.refresh_elements(pid)?;
    Ok(serde_json::json!({
      "mode": "accessibility",
      "duration_ms": started.elapsed().as_millis(),
      "application": self.last_scan.as_ref().map(|scan| &scan.application),
      "window": self.last_scan.as_ref().map(|scan| serde_json::json!({ "role": scan.window_role, "bounds": scan.window_bounds })),
      "node_count": self.last_scan.as_ref().map(|scan| scan.visited),
      "diagnostics": self.last_scan.as_ref().map(|scan| serde_json::json!({
        "candidate_count": scan.candidates,
        "accepted_before_deduplication": scan.accepted,
        "rejected": scan.rejected,
        "truncated": scan.truncated,
        "manual_accessibility_enabled": scan.manual_accessibility_enabled,
        "web_searches": scan.web_searches,
        "web_search_results": scan.web_search_results,
        "web_areas": scan.web_areas,
        "text": if show_text { "included" } else { "redacted" },
      })),
      "targets": self.elements.iter().map(|target| serde_json::json!({
        "role": target.role,
        "actions": target.actions,
        "bounds": target.bounds,
        "score": target.score,
        "source": target.source.name(),
        "has_text": !target.text.is_empty(),
        "text": show_text.then_some(&target.text),
      })).collect::<Vec<_>>(),
      "target_count": self.elements.len(),
    }))
  }
}

fn needs_visual_fallback(enabled: bool, skeletal: bool, truncated: bool) -> bool {
  enabled && skeletal && !truncated
}

/// Electron windows can report an accessible focused window before their first
/// composited frame is available to CoreGraphics. Retry empty OCR results for a
/// short, bounded warm-up instead of making users activate elements twice.
fn visual_targets_after_warmup(window: Rect) -> anyhow::Result<Vec<visual::VisualTarget>> {
  let mut transient_error = None;
  for attempt in 0..4 {
    match visual::text_targets(window) {
      Ok(targets) if !targets.is_empty() || attempt == 3 => return Ok(targets),
      Ok(_) => {}
      Err(error)
        if error.to_string().contains("could not capture focused window")
          || error.to_string().contains("no pixels") =>
      {
        transient_error = Some(error);
      }
      Err(error) => return Err(error),
    }
    if attempt != 3 {
      std::thread::sleep(Duration::from_millis(150));
    }
  }
  Err(transient_error.unwrap_or_else(|| anyhow::anyhow!("visual fallback found no visible text")))
}

/// Caches OCR geometry for one focused window. Vision can move or omit a text
/// box between otherwise identical captures; only two matching changed scans
/// replace the displayed snapshot.
struct VisualSnapshot {
  pid: i32,
  window: Rect,
  targets: Vec<Rect>,
  pending: Option<Vec<Rect>>,
}

fn stabilize_visual_targets(
  snapshot: &mut Option<VisualSnapshot>,
  pid: i32,
  window: Rect,
  mut fresh: Vec<Rect>,
) -> Vec<Rect> {
  sort_rects(&mut fresh);
  match snapshot {
    Some(snapshot) if snapshot.pid == pid && snapshot.window == window => {
      if visual_layout_matches(&snapshot.targets, &fresh) {
        snapshot.pending = None;
      } else if snapshot
        .pending
        .as_ref()
        .is_some_and(|pending| visual_layout_matches(pending, &fresh))
      {
        snapshot.targets = fresh;
        snapshot.pending = None;
      } else {
        snapshot.pending = Some(fresh);
      }
      snapshot.targets.clone()
    }
    _ => {
      *snapshot = Some(VisualSnapshot {
        pid,
        window,
        targets: fresh.clone(),
        pending: None,
      });
      fresh
    }
  }
}

fn visual_layout_matches(previous: &[Rect], fresh: &[Rect]) -> bool {
  previous.len() == fresh.len()
    && previous.iter().zip(fresh).all(|(&previous, &fresh)| {
      let overlap = clipped(previous, fresh).map_or(0.0, |rect| rect.width * rect.height);
      overlap >= (previous.width * previous.height).max(fresh.width * fresh.height) * 0.8
    })
}

/// Adds OCR text only where Accessibility did not already provide a click
/// target. This keeps semantic `AXPress` controls authoritative and prevents
/// duplicate labels on button text, links, and menu items.
fn merge_visual_targets(targets: &mut Vec<Element>, visual_targets: Vec<Rect>) {
  for bounds in visual_targets {
    if targets.iter().any(|existing| covers(existing.bounds, bounds)) {
      continue;
    }
    targets.push(Element {
      node: None,
      bounds,
      score: 1,
      depth: 0,
      role: "VisualText".into(),
      actions: vec![],
      text: vec![],
      source: TargetSource::Visual,
    });
  }
  sort_targets(targets);
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

/// Accessibility coordinates may contain sub-point jitter between otherwise
/// identical scans. Canonical points keep target identity and label assignment
/// stable while remaining well below pointer-placement precision.
fn stable_bounds(bounds: Rect) -> Rect {
  Rect {
    x: bounds.x.round(),
    y: bounds.y.round(),
    width: bounds.width.round(),
    height: bounds.height.round(),
  }
}

fn intersects(a: Rect, b: Rect) -> bool {
  a.x < b.x + b.width && a.x + a.width > b.x && a.y < b.y + b.height && a.y + a.height > b.y
}

#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
  fn AXUIElementCreateSystemWide() -> CFTypeRef;
  fn AXUIElementCreateApplication(pid: i32) -> CFTypeRef;
  fn AXObserverCreate(
    pid: i32,
    callback: extern "C" fn(CFTypeRef, CFTypeRef, core_foundation::string::CFStringRef, *mut std::ffi::c_void),
    out_observer: *mut CFTypeRef,
  ) -> i32;
  fn AXObserverAddNotification(
    observer: CFTypeRef,
    element: CFTypeRef,
    notification: core_foundation::string::CFStringRef,
    context: *mut std::ffi::c_void,
  ) -> i32;
  fn AXObserverGetRunLoopSource(observer: CFTypeRef) -> CFTypeRef;
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
  fn AXUIElementCopyParameterizedAttributeValue(
    element: CFTypeRef,
    attribute: core_foundation::string::CFStringRef,
    parameter: CFTypeRef,
    result: *mut CFTypeRef,
  ) -> i32;
  fn AXUIElementCopyActionNames(element: CFTypeRef, names: *mut CFTypeRef) -> i32;
  fn AXUIElementPerformAction(element: CFTypeRef, action: core_foundation::string::CFStringRef) -> i32;
  fn AXUIElementSetAttributeValue(
    element: CFTypeRef,
    attribute: core_foundation::string::CFStringRef,
    value: CFTypeRef,
  ) -> i32;
  fn AXUIElementSetMessagingTimeout(element: CFTypeRef, timeout: f32) -> i32;
  fn AXValueGetValue(value: CFTypeRef, kind: u32, out: *mut std::ffi::c_void) -> bool;
  fn AXValueGetTypeID() -> usize;
  fn AXIsProcessTrusted() -> bool;
  fn CFRunLoopGetCurrent() -> CFTypeRef;
  fn CFRunLoopAddSource(run_loop: CFTypeRef, source: CFTypeRef, mode: CFTypeRef);
  fn CFRunLoopRemoveSource(run_loop: CFTypeRef, source: CFTypeRef, mode: CFTypeRef);
  static kCFRunLoopCommonModes: CFTypeRef;
}

unsafe fn create_element_observer(
  pid: i32,
  app: CFTypeRef,
  window: CFTypeRef,
  dirty: &ElementDirty,
) -> Option<ElementObserver> {
  let mut observer = std::ptr::null();
  if unsafe { AXObserverCreate(pid, accessibility_changed, &mut observer) } != 0 || observer.is_null() {
    return None;
  }
  let observer = OwnedCf(observer);
  let context = (dirty as *const ElementDirty).cast_mut().cast();
  let mut registered = false;
  for name in ["AXFocusedWindowChanged", "AXMainWindowChanged"] {
    registered |=
      unsafe { AXObserverAddNotification(observer.0, app, CFString::new(name).as_concrete_TypeRef(), context) == 0 };
  }
  for name in ["AXMoved", "AXResized", "AXLayoutChanged", "AXUIElementDestroyed"] {
    registered |=
      unsafe { AXObserverAddNotification(observer.0, window, CFString::new(name).as_concrete_TypeRef(), context) == 0 };
  }
  if !registered {
    return None;
  }
  let source = unsafe { AXObserverGetRunLoopSource(observer.0) };
  if source.is_null() {
    return None;
  }
  let run_loop = unsafe { CFRunLoopGetCurrent() };
  unsafe { CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes) };
  Some(ElementObserver {
    _observer: observer,
    source,
    run_loop,
    pid,
    window_hash: unsafe { core_foundation::base::CFHash(window) },
  })
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

/// AXFocusedApplication is unavailable to some detached launchd processes.
/// NSWorkspace still reports foreground PID in active user session.
fn focused_application(system: CFTypeRef) -> Option<OwnedCf> {
  attribute(system, "AXFocusedApplication").or_else(|| unsafe {
    let workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let running: id = msg_send![workspace, frontmostApplication];
    if running == nil {
      return None;
    }
    let pid: i32 = msg_send![running, processIdentifier];
    let app = AXUIElementCreateApplication(pid);
    (!app.is_null()).then_some(OwnedCf(app))
  })
}

unsafe fn set_bool_attribute(element: CFTypeRef, name: &str, value: bool) -> bool {
  let name = CFString::new(name);
  let value = unsafe {
    if value {
      core_foundation::boolean::kCFBooleanTrue.cast()
    } else {
      core_foundation::boolean::kCFBooleanFalse.cast()
    }
  };
  unsafe { AXUIElementSetAttributeValue(element, name.as_concrete_TypeRef(), value) == 0 }
}

fn string_attribute(element: CFTypeRef, name: &str) -> Option<String> {
  use core_foundation::base::CFGetTypeID;
  let value = attribute(element, name)?;
  unsafe {
    (CFGetTypeID(value.0) == core_foundation::string::CFStringGetTypeID())
      .then(|| CFString::wrap_under_get_rule(value.0.cast()).to_string())
  }
}

fn bool_attribute(element: CFTypeRef, name: &str) -> Option<bool> {
  use core_foundation::base::CFGetTypeID;
  let value = attribute(element, name)?;
  unsafe {
    (CFGetTypeID(value.0) == core_foundation::boolean::CFBooleanGetTypeID())
      .then(|| value.0 == core_foundation::boolean::kCFBooleanTrue.cast())
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

struct Element {
  node: Option<OwnedCf>,
  bounds: Rect,
  score: i32,
  depth: usize,
  role: String,
  actions: Vec<String>,
  text: Vec<String>,
  source: TargetSource,
}

#[derive(Clone, Copy)]
enum TargetSource {
  Semantic,
  Visual,
}
impl TargetSource {
  fn name(self) -> &'static str {
    match self {
      Self::Semantic => "accessibility",
      Self::Visual => "visual",
    }
  }
}

struct Discovery {
  app: OwnedCf,
  pid: i32,
  targets: Vec<Element>,
  info: ScanInfo,
}

impl Discovery {
  fn skeletal(&self) -> bool {
    if self.targets.is_empty() {
      return true;
    }
    let Some(window) = self.info.window_bounds else {
      return false;
    };
    titlebar_only(
      &self.targets.iter().map(|target| target.bounds).collect::<Vec<_>>(),
      window,
    )
  }
}

fn titlebar_only(targets: &[Rect], window: Rect) -> bool {
  targets.len() <= 3
    && targets
      .iter()
      .all(|target| target.height <= 32.0 && target.y < window.y + 64.0)
}

struct ScanInfo {
  application: String,
  window_role: String,
  window_bounds: Option<Rect>,
  visited: usize,
  candidates: usize,
  accepted: usize,
  rejected: BTreeMap<&'static str, usize>,
  truncated: bool,
  manual_accessibility_enabled: bool,
  web_searches: usize,
  web_search_results: usize,
  web_areas: usize,
}

fn record_rejection(reasons: &mut BTreeMap<&'static str, usize>, reason: &'static str) {
  *reasons.entry(reason).or_default() += 1;
}

const ACTIONABLE_ACTIONS: &[&str] = &["AXPress", "AXConfirm", "AXShowMenu", "AXIncrement", "AXDecrement"];

fn actions(element: CFTypeRef) -> Vec<String> {
  use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
  unsafe {
    let mut values = std::ptr::null();
    if AXUIElementCopyActionNames(element, &mut values) != 0 || values.is_null() {
      return vec![];
    }
    let values = OwnedCf(values);
    (0..CFArrayGetCount(values.0.cast()))
      .filter_map(|index| {
        let value = CFArrayGetValueAtIndex(values.0.cast(), index);
        (!value.is_null()).then(|| CFString::wrap_under_get_rule(value.cast()).to_string())
      })
      .collect()
  }
}

fn children(element: CFTypeRef, attribute_name: &str, limit: usize) -> Vec<OwnedCf> {
  use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
  let name = CFString::new(attribute_name);
  let mut result = Vec::new();
  unsafe {
    for offset in (0..limit).step_by(256) {
      let mut values = std::ptr::null();
      if AXUIElementCopyAttributeValues(element, name.as_concrete_TypeRef(), offset as isize, 256, &mut values) != 0
        || values.is_null()
      {
        break;
      }
      let values = OwnedCf(values);
      let count = CFArrayGetCount(values.0.cast()) as usize;
      for index in 0..count {
        let child = CFArrayGetValueAtIndex(values.0.cast(), index as isize);
        if !child.is_null() {
          result.push(OwnedCf(CFRetain(child)));
        }
      }
      if count < 256 {
        break;
      }
    }
  }
  result
}

/// Chromium and WebKit web areas can provide already-filtered descendants via
/// this public parameterized AX attribute. Unsupported apps return an empty
/// result and retain normal paged child traversal below.
fn web_search_children(element: CFTypeRef, limit: usize) -> Vec<OwnedCf> {
  use core_foundation::array::{CFArrayGetCount, CFArrayGetValueAtIndex};
  if limit == 0 {
    return vec![];
  }
  unsafe {
    let predicate: id = msg_send![class!(NSMutableDictionary), dictionary];
    let key = string("AXAnyTypeSearchKey");
    let visible: id = msg_send![class!(NSNumber), numberWithBool: YES];
    let count: id = msg_send![class!(NSNumber), numberWithInteger: limit as isize];
    let _: () = msg_send![predicate, setObject: key forKey: string("AXSearchKey")];
    let _: () = msg_send![predicate, setObject: visible forKey: string("AXVisibleOnly")];
    let _: () = msg_send![predicate, setObject: count forKey: string("AXResultsLimit")];
    let attribute = CFString::new("AXUIElementsForSearchPredicate");
    let mut values = std::ptr::null();
    if AXUIElementCopyParameterizedAttributeValue(
      element,
      attribute.as_concrete_TypeRef(),
      predicate.cast(),
      &mut values,
    ) != 0
      || values.is_null()
    {
      return vec![];
    }
    let values = OwnedCf(values);
    (0..CFArrayGetCount(values.0.cast()))
      .filter_map(|index| {
        let child = CFArrayGetValueAtIndex(values.0.cast(), index);
        (!child.is_null() && CFEqual(child, element) == 0).then(|| OwnedCf(CFRetain(child)))
      })
      .collect()
  }
}

fn comparable_rect(a: Rect, b: Rect) -> bool {
  let intersection = clipped(a, b).map_or(0.0, |r| r.width * r.height);
  intersection >= (a.width * a.height).max(b.width * b.height) * 0.95
}

/// A visual text box belongs to an accessibility target when nearly all of
/// its pixels lie inside that target. OCR often finds only a button's caption.
fn covers(target: Rect, visual: Rect) -> bool {
  let overlap = clipped(target, visual).map_or(0.0, |rect| rect.width * rect.height);
  overlap >= visual.width * visual.height * 0.9
}

fn has_supported_action(actions: &[String]) -> bool {
  actions
    .iter()
    .any(|action| ACTIONABLE_ACTIONS.contains(&action.as_str()))
}

fn score(actions: &[String], named: bool, bounds: Rect) -> i32 {
  let action_score = if actions.iter().any(|action| action == "AXPress") {
    100
  } else if actions
    .iter()
    .any(|action| ACTIONABLE_ACTIONS.contains(&action.as_str()))
  {
    80
  } else {
    0
  };
  action_score + i32::from(named) * 10 + (bounds.width.min(bounds.height).min(40.0) as i32 / 4)
}

fn matches_filter(text: &[String], role: &str, query: &str) -> bool {
  let query = normalize(query);
  !query.is_empty()
    && text
      .iter()
      .map(String::as_str)
      .chain(std::iter::once(role))
      .any(|value| normalize(value).contains(&query))
}

fn normalize(value: &str) -> String {
  value
    .split_whitespace()
    .flat_map(str::chars)
    .flat_map(char::to_lowercase)
    .collect()
}

fn discover_elements(pid: Option<i32>) -> anyhow::Result<Discovery> {
  unsafe {
    anyhow::ensure!(
      AXIsProcessTrusted(),
      "Accessibility access required: System Settings > Privacy & Security > Accessibility"
    );
    let system = OwnedCf(AXUIElementCreateSystemWide());
    AXUIElementSetMessagingTimeout(system.0, 0.05);
    let app = match pid {
      Some(pid) => {
        let app = AXUIElementCreateApplication(pid);
        anyhow::ensure!(!app.is_null(), "Cannot create accessibility element for process {pid}");
        OwnedCf(app)
      }
      None => focused_application(system.0).ok_or_else(|| anyhow::anyhow!("Cannot read focused application"))?,
    };
    let mut actual_pid = 0i32;
    anyhow::ensure!(
      AXUIElementGetPid(app.0, &mut actual_pid) == 0,
      "Cannot read focused process"
    );
    AXUIElementSetMessagingTimeout(app.0, 0.05);
    let root = attribute(app.0, "AXFocusedWindow").unwrap_or_else(|| OwnedCf(CFRetain(app.0)));
    let root_bounds = read_rect(root.0);
    let application = string_attribute(app.0, "AXTitle").unwrap_or_default();
    let window_role = string_attribute(root.0, "AXRole").unwrap_or_default();
    let started = Instant::now();
    let budget = Duration::from_millis(350);
    // AX visible-search already returns flattened web descendants. Expanding
    // each result again turns one viewport into an unbounded browser-tree walk.
    let mut queue = VecDeque::from([(root, 0usize, root_bounds, true)]);
    let mut visited = 0usize;
    let mut candidates = 0usize;
    let mut accepted = 0usize;
    let mut rejected = BTreeMap::new();
    let mut truncated = false;
    let mut web_searches = 0usize;
    let mut web_search_results = 0usize;
    let mut web_areas = 0usize;
    let mut targets = Vec::new();
    while let Some((node, depth, mut clip, expand_children)) = queue.pop_front() {
      if visited >= 4000 || started.elapsed() >= budget {
        truncated = true;
        break;
      }
      visited += 1;
      let role = string_attribute(node.0, "AXRole").unwrap_or_default();
      if role == "AXWebArea" {
        web_areas += 1;
      }
      if bool_attribute(node.0, "AXHidden") == Some(true) {
        record_rejection(&mut rejected, "hidden");
        continue;
      }
      if matches!(role.as_str(), "AXScrollArea" | "AXWindow" | "AXSheet")
        && let Some(bounds) = read_rect(node.0)
      {
        clip = clip.map_or(Some(bounds), |parent| clipped(bounds, parent));
        if clip.is_none() {
          record_rejection(&mut rejected, "outside-viewport");
          continue;
        }
      }
      let actions = actions(node.0);
      let actionable = has_supported_action(&actions);
      if actionable {
        candidates += 1;
      }
      if !actionable {
        record_rejection(&mut rejected, "not-actionable");
      } else if bool_attribute(node.0, "AXEnabled") == Some(false) {
        record_rejection(&mut rejected, "disabled");
      } else if let Some(bounds) = read_rect(node.0) {
        if let Some(bounds) = clip.map_or(Some(bounds), |visible| clipped(bounds, visible)) {
          let text = ["AXTitle", "AXDescription", "AXValue", "AXHelp", "AXIdentifier"]
            .into_iter()
            .filter_map(|name| string_attribute(node.0, name))
            .filter(|value| !value.trim().is_empty())
            .collect::<Vec<_>>();
          let bounds = stable_bounds(bounds);
          let score = score(&actions, !text.is_empty(), bounds);
          if score > 0 {
            accepted += 1;
            targets.push(Element {
              node: Some(OwnedCf(CFRetain(node.0))),
              bounds,
              score,
              depth,
              role: role.clone(),
              actions,
              text,
              source: TargetSource::Semantic,
            });
          } else {
            record_rejection(&mut rejected, "score-zero");
          }
        } else {
          record_rejection(&mut rejected, "outside-viewport");
        }
      } else {
        record_rejection(&mut rejected, "missing-bounds");
      }
      if !expand_children {
        continue;
      }
      if depth >= 32 || started.elapsed() >= budget {
        if !queue.is_empty() {
          truncated = true;
        }
        continue;
      }
      let remaining = 4000usize.saturating_sub(visited + queue.len());
      let mut flattened_web_results = false;
      let mut descendants = if role == "AXWebArea" {
        web_searches += 1;
        // This public visible-only query returns every candidate in viewport.
        // Its results are terminal: recursively walking each one would turn a
        // bounded viewport query back into an unbounded browser-tree walk.
        let matches = web_search_children(node.0, remaining);
        web_search_results += matches.len();
        flattened_web_results = !matches.is_empty();
        matches
      } else if matches!(role.as_str(), "AXTable" | "AXOutline") {
        children(node.0, "AXVisibleRows", remaining)
      } else {
        vec![]
      };
      if descendants.is_empty() && matches!(role.as_str(), "AXScrollArea" | "AXTable" | "AXOutline") {
        descendants = children(node.0, "AXVisibleChildren", remaining);
      }
      if descendants.is_empty() {
        descendants = children(node.0, "AXChildren", remaining);
      }
      for child in descendants {
        queue.push_back((child, depth + 1, clip, !flattened_web_results));
      }
    }
    targets.sort_by_key(|target| (-(target.depth as isize), -(target.score as isize)));
    let mut deduplicated: Vec<Element> = Vec::new();
    for target in targets {
      if !deduplicated
        .iter()
        .any(|existing| comparable_rect(existing.bounds, target.bounds))
      {
        deduplicated.push(target);
      } else {
        record_rejection(&mut rejected, "duplicate-bounds");
      }
    }
    // AX child order is not stable in browsers. Labels must follow a stable
    // reading order, never traversal timing or action-score ties.
    sort_targets(&mut deduplicated);
    Ok(Discovery {
      app,
      pid: actual_pid,
      targets: deduplicated,
      info: ScanInfo {
        application,
        window_role,
        window_bounds: root_bounds,
        visited,
        candidates,
        accepted,
        rejected,
        truncated,
        manual_accessibility_enabled: false,
        web_searches,
        web_search_results,
        web_areas,
      },
    })
  }
}

fn sort_targets(targets: &mut [Element]) {
  targets.sort_by(|a, b| {
    a.bounds
      .y
      .total_cmp(&b.bounds.y)
      .then_with(|| a.bounds.x.total_cmp(&b.bounds.x))
      .then_with(|| a.bounds.height.total_cmp(&b.bounds.height))
      .then_with(|| a.bounds.width.total_cmp(&b.bounds.width))
      .then_with(|| a.role.cmp(&b.role))
      .then_with(|| a.actions.cmp(&b.actions))
  });
}

fn sort_rects(rects: &mut [Rect]) {
  rects.sort_by(|a, b| {
    a.y
      .total_cmp(&b.y)
      .then_with(|| a.x.total_cmp(&b.x))
      .then_with(|| a.height.total_cmp(&b.height))
      .then_with(|| a.width.total_cmp(&b.width))
  });
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dirty_generation_coalesces_notification_storms() {
    let dirty = ElementDirty(AtomicU64::new(0));
    assert_eq!(dirty.generation(), 0);
    dirty.mark();
    dirty.mark();
    assert_eq!(dirty.generation(), 2);
  }

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
  #[test]
  fn target_ranking_prefers_press_and_deduplicates_near_identical_controls() {
    let bounds = Rect {
      x: 10.0,
      y: 10.0,
      width: 40.0,
      height: 20.0,
    };
    assert!(score(&["AXPress".into()], true, bounds) > score(&["AXShowMenu".into()], true, bounds));
    assert!(!has_supported_action(&[]));
    assert_eq!(
      stable_bounds(Rect {
        x: 10.49,
        y: 20.51,
        width: 40.49,
        height: 20.51,
      }),
      Rect {
        x: 10.0,
        y: 21.0,
        width: 40.0,
        height: 21.0,
      }
    );
    assert!(covers(
      Rect {
        x: 10.0,
        y: 10.0,
        width: 100.0,
        height: 40.0,
      },
      Rect {
        x: 20.0,
        y: 20.0,
        width: 40.0,
        height: 12.0,
      }
    ));
    assert!(!covers(
      Rect {
        x: 10.0,
        y: 10.0,
        width: 20.0,
        height: 20.0,
      },
      Rect {
        x: 25.0,
        y: 10.0,
        width: 20.0,
        height: 20.0,
      }
    ));
    assert!(comparable_rect(
      bounds,
      Rect {
        x: 10.1,
        y: 10.1,
        width: 39.8,
        height: 19.8,
      }
    ));
    assert!(!comparable_rect(
      bounds,
      Rect {
        x: 60.0,
        y: 10.0,
        width: 40.0,
        height: 20.0,
      }
    ));
    assert_eq!(score(&[], false, bounds), 5);
  }
  #[test]
  fn semantic_filter_normalizes_case_and_whitespace_and_includes_role() {
    assert!(matches_filter(&["  Play   Song ".into()], "AXButton", "play song"));
    assert!(matches_filter(&[], "AXCheckBox", "checkbox"));
    assert!(!matches_filter(&["Pause".into()], "AXButton", "play"));
  }
  #[test]
  fn semantic_controls_bypass_ocr_even_when_visual_fallback_is_enabled() {
    assert!(!needs_visual_fallback(true, false, false));
    assert!(!needs_visual_fallback(true, false, true));
    assert!(!needs_visual_fallback(true, true, true));
    assert!(!needs_visual_fallback(false, true, false));
    assert!(needs_visual_fallback(true, true, false));
  }

  #[test]
  fn visual_snapshot_ignores_jitter_and_confirms_layout_changes() {
    let window = Rect {
      x: 0.0,
      y: 0.0,
      width: 400.0,
      height: 300.0,
    };
    let initial = Rect {
      x: 10.0,
      y: 10.0,
      width: 100.0,
      height: 20.0,
    };
    let jittered = Rect { x: 12.0, ..initial };
    let changed = Rect { x: 150.0, ..initial };
    let mut snapshot = None;

    assert_eq!(
      stabilize_visual_targets(&mut snapshot, 42, window, vec![initial]),
      vec![initial]
    );
    assert_eq!(
      stabilize_visual_targets(&mut snapshot, 42, window, vec![jittered]),
      vec![initial]
    );
    assert_eq!(
      stabilize_visual_targets(&mut snapshot, 42, window, vec![changed]),
      vec![initial]
    );
    assert_eq!(
      stabilize_visual_targets(&mut snapshot, 42, window, vec![changed]),
      vec![changed]
    );
  }

  #[test]
  fn mixed_semantic_and_visual_targets_keep_semantic_controls_authoritative() {
    let semantic = Rect {
      x: 10.0,
      y: 10.0,
      width: 100.0,
      height: 30.0,
    };
    let mut targets = vec![Element {
      node: None,
      bounds: semantic,
      score: 100,
      depth: 1,
      role: "AXButton".into(),
      actions: vec!["AXPress".into()],
      text: vec!["Play".into()],
      source: TargetSource::Semantic,
    }];
    merge_visual_targets(
      &mut targets,
      vec![
        Rect {
          x: 30.0,
          y: 18.0,
          width: 40.0,
          height: 12.0,
        },
        Rect {
          x: 130.0,
          y: 10.0,
          width: 60.0,
          height: 20.0,
        },
      ],
    );
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0].role, "AXButton");
    assert_eq!(targets[1].role, "VisualText");
  }

  #[test]
  fn nested_clickable_controls_are_not_duplicates() {
    let card = Rect {
      x: 0.0,
      y: 0.0,
      width: 200.0,
      height: 200.0,
    };
    let play = Rect {
      x: 160.0,
      y: 160.0,
      width: 32.0,
      height: 32.0,
    };
    assert!(!comparable_rect(card, play));
    assert!(!comparable_rect(play, card));
    assert!(comparable_rect(play, play));
    assert!(!has_supported_action(&["AXRaise".into()]));
  }
  #[test]
  fn titlebar_only_detects_a_skeletal_app_tree() {
    let window = Rect {
      x: 100.0,
      y: 200.0,
      width: 800.0,
      height: 600.0,
    };
    assert!(titlebar_only(
      &[Rect {
        x: 110.0,
        y: 210.0,
        width: 16.0,
        height: 16.0,
      }],
      window
    ));
    assert!(!titlebar_only(
      &[Rect {
        x: 110.0,
        y: 300.0,
        width: 16.0,
        height: 16.0,
      }],
      window
    ));
  }
  #[test]
  fn label_positions_stay_inside_large_targets_and_center_small_ones() {
    fn assert_point(point: NSPoint, x: f64, y: f64) {
      assert_eq!((point.x, point.y), (x, y));
    }
    let cell = NSRect::new(NSPoint::new(10.0, 20.0), NSSize::new(100.0, 60.0));
    let size = NSSize::new(20.0, 10.0);
    assert_point(label_origin(cell, size, LabelPosition::Top), 50.0, 68.0);
    assert_point(label_origin(cell, size, LabelPosition::Right), 86.0, 45.0);
    assert_point(label_origin(cell, size, LabelPosition::Bottom), 50.0, 22.0);
    assert_point(label_origin(cell, size, LabelPosition::Left), 14.0, 45.0);
    assert_point(label_origin(cell, size, LabelPosition::TopLeft), 14.0, 68.0);
    assert_point(label_origin(cell, size, LabelPosition::TopRight), 86.0, 68.0);
    assert_point(label_origin(cell, size, LabelPosition::BottomLeft), 14.0, 22.0);
    assert_point(label_origin(cell, size, LabelPosition::BottomRight), 86.0, 22.0);
    let small = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(20.0, 10.0));
    assert_point(label_origin(small, size, LabelPosition::TopLeft), 0.0, 0.0);
  }
}
