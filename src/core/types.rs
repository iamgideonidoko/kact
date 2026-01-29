use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector2D {
  pub x: f64,
  pub y: f64,
}

impl Vector2D {
  pub fn new(x: f64, y: f64) -> Self {
    Self { x, y }
  }

  pub fn zero() -> Self {
    Self { x: 0.0, y: 0.0 }
  }

  pub fn magnitude(&self) -> f64 {
    (self.x * self.x + self.y * self.y).sqrt()
  }

  pub fn normalize(&self) -> Self {
    let mag = self.magnitude();
    if mag > 0.0 {
      Self {
        x: self.x / mag,
        y: self.y / mag,
      }
    } else {
      Self::zero()
    }
  }

  pub fn scale(&self, scalar: f64) -> Self {
    Self {
      x: self.x * scalar,
      y: self.y * scalar,
    }
  }

  pub fn add(&self, other: &Vector2D) -> Self {
    Self {
      x: self.x + other.x,
      y: self.y + other.y,
    }
  }
}
