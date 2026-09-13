//! Opt-in pixel-derived targets. Never called by normal Accessibility discovery.

use crate::desktop::Rect;
use anyhow::{Context, Result, bail, ensure};
use cocoa::base::{NO, id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSRect};
use objc::{class, msg_send, sel, sel_impl};
use std::ffi::c_void;

#[derive(Debug, Clone)]
pub(super) struct VisualTarget {
  pub bounds: Rect,
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
  fn CGPreflightScreenCaptureAccess() -> bool;
  fn CGRequestScreenCaptureAccess() -> bool;
  fn CGWindowListCreateImage(bounds: CGRect, options: u32, window_id: u32, image_options: u32) -> *mut c_void;
  fn CGImageGetWidth(image: *mut c_void) -> usize;
  fn CGImageGetHeight(image: *mut c_void) -> usize;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CGPoint {
  x: f64,
  y: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CGSize {
  width: f64,
  height: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct CGRect {
  origin: CGPoint,
  size: CGSize,
}

/// Extract text-region rectangles using Vision from pixels within `window`.
///
/// Screen Recording is a hard precondition. `CGWindowListCreateImage` remains
/// deliberately scoped to the focused window rectangle: this feature neither
/// captures nor retains other display contents.
pub(super) fn text_targets(window: Rect) -> Result<Vec<VisualTarget>> {
  ensure!(
    [window.x, window.y, window.width, window.height]
      .iter()
      .all(|value| value.is_finite())
      && window.width > 1.0
      && window.height > 1.0,
    "visual fallback requires bounds for focused window"
  );
  unsafe {
    ensure!(
      CGPreflightScreenCaptureAccess() || CGRequestScreenCaptureAccess(),
      "visual fallback requires Screen Recording permission for Kact in System Settings > Privacy & Security > Screen Recording"
    );
    let pool = NSAutoreleasePool::new(nil);
    // Vision has no C symbols here; load it explicitly before resolving ObjC classes.
    let vision: id = msg_send![class!(NSBundle), bundleWithPath: crate::desktop::macos::string("/System/Library/Frameworks/Vision.framework")];
    let loaded: cocoa::base::BOOL = msg_send![vision, load];
    if loaded == NO {
      pool.drain();
      bail!("Vision text recognition is unavailable on this macOS version");
    }
    let image = CGWindowListCreateImage(
      CGRect {
        origin: CGPoint {
          x: window.x,
          y: window.y,
        },
        size: CGSize {
          width: window.width,
          height: window.height,
        },
      },
      1, // kCGWindowListOptionOnScreenOnly
      0, // kCGNullWindowID
      0, // kCGWindowImageDefault
    );
    if image.is_null() {
      pool.drain();
      bail!("could not capture focused window; verify Screen Recording permission");
    }
    let image_width = CGImageGetWidth(image);
    let image_height = CGImageGetHeight(image);
    if image_width == 0 || image_height == 0 {
      core_foundation::base::CFRelease(image);
      pool.drain();
      bail!("captured focused window has no pixels");
    }
    let request: id = msg_send![class!(VNRecognizeTextRequest), alloc];
    let request: id = msg_send![request, init];
    if request == nil {
      core_foundation::base::CFRelease(image);
      pool.drain();
      bail!("Vision text recognition is unavailable on this macOS version");
    }
    // Fast recognition keeps activation bounded; target geometry matters, not OCR text.
    let _: () = msg_send![request, setRecognitionLevel: 0isize];
    let _: () = msg_send![request, setUsesLanguageCorrection: NO];
    let handler: id = msg_send![class!(VNImageRequestHandler), alloc];
    let options: id = msg_send![class!(NSDictionary), dictionary];
    let handler: id = msg_send![handler, initWithCGImage: image options: options];
    let requests: id = msg_send![class!(NSArray), arrayWithObject: request];
    let mut error: id = nil;
    let ok: cocoa::base::BOOL = msg_send![handler, performRequests: requests error: &mut error];
    let result = if ok == NO {
      let description: id = if error == nil {
        nil
      } else {
        msg_send![error, localizedDescription]
      };
      let detail = if description == nil {
        "Vision text recognition failed".to_owned()
      } else {
        let utf8: *const std::os::raw::c_char = msg_send![description, UTF8String];
        if utf8.is_null() {
          "Vision text recognition failed".to_owned()
        } else {
          std::ffi::CStr::from_ptr(utf8).to_string_lossy().into_owned()
        }
      };
      Err(anyhow::anyhow!(detail))
    } else {
      let results: id = msg_send![request, results];
      let count: usize = if results == nil { 0 } else { msg_send![results, count] };
      let mut targets = Vec::with_capacity(count);
      for index in 0..count {
        let observation: id = msg_send![results, objectAtIndex: index];
        let box_: NSRect = msg_send![observation, boundingBox];
        // Vision uses normalized lower-left coordinates; Kact uses top-left.
        let bounds = Rect {
          x: window.x + box_.origin.x * window.width,
          y: window.y + (1.0 - box_.origin.y - box_.size.height) * window.height,
          width: box_.size.width * window.width,
          height: box_.size.height * window.height,
        };
        if bounds.width >= 6.0 && bounds.height >= 6.0 {
          targets.push(VisualTarget { bounds });
        }
      }
      Ok(targets)
    };
    let _: () = msg_send![handler, release];
    let _: () = msg_send![request, release];
    core_foundation::base::CFRelease(image);
    pool.drain();
    result.context("visual fallback could not derive text targets")
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn visual_target_keeps_only_geometry() {
    let target = VisualTarget {
      bounds: Rect {
        x: 1.0,
        y: 2.0,
        width: 3.0,
        height: 4.0,
      },
    };
    assert_eq!(target.bounds.width, 3.0);
  }
}
