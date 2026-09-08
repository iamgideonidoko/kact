use super::types::Vector2D;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GlideEasing {
  Linear,
  EaseInOut,
  EaseOut,
}

pub struct Glide {
  start: Vector2D,
  target: Vector2D,
  duration: Duration,
  easing: GlideEasing,
}

impl Glide {
  pub fn new(start: Vector2D, target: Vector2D, duration: Duration, easing: GlideEasing) -> Self {
    Self {
      start,
      target,
      duration,
      easing,
    }
  }

  pub fn point_at(&self, elapsed: Duration) -> (Vector2D, bool) {
    if elapsed >= self.duration {
      return (self.target, true);
    }
    let progress = elapsed.as_secs_f64() / self.duration.as_secs_f64();
    let progress = match self.easing {
      GlideEasing::Linear => progress,
      GlideEasing::EaseInOut => progress * progress * (3.0 - 2.0 * progress),
      GlideEasing::EaseOut => 1.0 - (1.0 - progress).powi(3),
    };
    (
      self
        .start
        .add(&self.target.add(&self.start.scale(-1.0)).scale(progress)),
      false,
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn glides_reach_the_exact_target_with_each_easing() {
    let start = Vector2D::new(10.0, -20.0);
    let target = Vector2D::new(110.0, 80.0);
    for easing in [GlideEasing::Linear, GlideEasing::EaseInOut, GlideEasing::EaseOut] {
      let glide = Glide::new(start, target, Duration::from_millis(100), easing);
      assert_eq!(glide.point_at(Duration::ZERO), (start, false));
      assert_eq!(glide.point_at(Duration::from_millis(100)), (target, true));
      assert_eq!(glide.point_at(Duration::from_millis(200)), (target, true));
    }
  }

  #[test]
  fn easings_have_the_expected_midpoint_shape() {
    let start = Vector2D::zero();
    let target = Vector2D::new(100.0, 0.0);
    let duration = Duration::from_millis(100);
    assert_eq!(
      Glide::new(start, target, duration, GlideEasing::Linear)
        .point_at(Duration::from_millis(50))
        .0
        .x,
      50.0
    );
    assert_eq!(
      Glide::new(start, target, duration, GlideEasing::EaseInOut)
        .point_at(Duration::from_millis(50))
        .0
        .x,
      50.0
    );
    assert!(
      Glide::new(start, target, duration, GlideEasing::EaseOut)
        .point_at(Duration::from_millis(50))
        .0
        .x
        > 50.0
    );
  }
}
