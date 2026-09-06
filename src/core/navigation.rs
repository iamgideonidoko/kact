use crate::Result;
use crate::config::{invalid, validate_alphabet};
use crate::desktop::{Rect, Target};

/// Equal-length labels are prefix-free, so a complete label always selects one target.
pub fn labels(count: usize, alphabet: &str) -> Result<Vec<String>> {
  validate_alphabet(alphabet)?;
  if count > 100_000 {
    return Err(invalid("too many navigation targets (maximum 100000)"));
  }
  let alphabet = alphabet.as_bytes();
  let mut length = 1;
  let mut capacity = alphabet.len();
  while capacity < count {
    capacity *= alphabet.len();
    length += 1;
  }
  Ok(
    (0..count)
      .map(|mut index| {
        let mut label = vec![alphabet[0]; length];
        for character in label.iter_mut().rev() {
          *character = alphabet[index % alphabet.len()];
          index /= alphabet.len();
        }
        String::from_utf8(label).expect("validated ASCII alphabet")
      })
      .collect(),
  )
}

#[derive(Debug, Clone)]
pub struct Navigation {
  pub targets: Vec<Target>,
  pub prefix: String,
}

impl Navigation {
  pub fn from_rects(bounds: &[Rect], alphabet: &str) -> Result<Self> {
    for rect in bounds {
      if ![rect.x, rect.y, rect.width, rect.height].iter().all(|v| v.is_finite())
        || rect.width <= 0.0
        || rect.height <= 0.0
      {
        return Err(invalid("navigation bounds must be finite with positive size"));
      }
    }
    let targets = labels(bounds.len(), alphabet)?
      .into_iter()
      .zip(bounds)
      .map(|(label, bounds)| Target { label, bounds: *bounds })
      .collect();
    Ok(Self {
      targets,
      prefix: String::new(),
    })
  }

  pub fn grid(bounds: &[Rect], rows: usize, columns: usize, alphabet: &str) -> Result<Self> {
    if !(1..=100).contains(&rows)
      || !(1..=100).contains(&columns)
      || bounds.len().saturating_mul(rows).saturating_mul(columns) > 100_000
    {
      return Err(invalid("grid dimensions exceed limits"));
    }
    let mut cells = Vec::with_capacity(bounds.len() * rows * columns);
    for rect in bounds {
      for row in 0..rows {
        for column in 0..columns {
          let width = rect.width / columns as f64;
          let height = rect.height / rows as f64;
          cells.push(Rect {
            x: rect.x + column as f64 * width,
            y: rect.y + row as f64 * height,
            width,
            height,
          });
        }
      }
    }
    Self::from_rects(&cells, alphabet)
  }

  pub fn type_char(&mut self, character: char) -> Option<Target> {
    if !character.is_ascii_alphabetic() {
      return None;
    }
    let candidate = format!("{}{}", self.prefix, character.to_ascii_lowercase());
    if !self.targets.iter().any(|target| target.label.starts_with(&candidate)) {
      return None;
    }
    self.prefix = candidate;
    self.targets.iter().find(|target| target.label == self.prefix).cloned()
  }

  pub fn backspace(&mut self) {
    self.prefix.pop();
  }
  pub fn clear_prefix(&mut self) {
    self.prefix.clear();
  }
  pub fn visible(&self) -> Vec<Target> {
    self
      .targets
      .iter()
      .filter(|target| target.label.starts_with(&self.prefix))
      .cloned()
      .collect()
  }
}
