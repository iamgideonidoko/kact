#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
  Normal,
  Precise,
  Fast,
}

impl Default for Mode {
  fn default() -> Self {
    Mode::Normal
  }
}
