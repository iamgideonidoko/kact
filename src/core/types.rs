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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Direction {
  Up,
  Down,
  Left,
  Right,
}

impl Direction {
  pub fn to_vector(&self) -> Vector2D {
    match self {
      Direction::Up => Vector2D::new(0.0, -1.0),
      Direction::Down => Vector2D::new(0.0, 1.0),
      Direction::Left => Vector2D::new(-1.0, 0.0),
      Direction::Right => Vector2D::new(1.0, 0.0),
    }
  }
}
