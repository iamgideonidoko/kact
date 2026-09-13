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
  focused: Option<usize>,
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
      .map(|(label, bounds)| Target {
        label,
        bounds: *bounds,
        focused: false,
      })
      .collect();
    Ok(Self {
      targets,
      prefix: String::new(),
      focused: None,
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
  pub fn cycle(&mut self, backwards: bool) -> Option<Target> {
    if self.targets.is_empty() {
      return None;
    }
    let next = match (self.focused, backwards) {
      (Some(index), false) => (index + 1) % self.targets.len(),
      (Some(0), true) => self.targets.len() - 1,
      (Some(index), true) => index - 1,
      (None, false) => 0,
      (None, true) => self.targets.len() - 1,
    };
    self.focused = Some(next);
    self.clear_prefix();
    self.targets.get(next).cloned()
  }
  pub fn visible(&self) -> Vec<Target> {
    self
      .targets
      .iter()
      .enumerate()
      .filter(|(_, target)| target.label.starts_with(&self.prefix))
      .map(|(index, target)| Target {
        focused: self.focused == Some(index),
        ..target.clone()
      })
      .collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn cycling_wraps_and_marks_only_current_target() {
    let bounds = [
      Rect {
        x: 0.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
      },
      Rect {
        x: 1.0,
        y: 0.0,
        width: 1.0,
        height: 1.0,
      },
    ];
    let mut navigation = Navigation::from_rects(&bounds, "ab").unwrap();
    assert_eq!(navigation.cycle(false).unwrap().bounds, bounds[0]);
    assert_eq!(navigation.visible().iter().filter(|target| target.focused).count(), 1);
    assert_eq!(navigation.cycle(true).unwrap().bounds, bounds[1]);
    assert_eq!(navigation.cycle(false).unwrap().bounds, bounds[0]);
    assert_eq!(navigation.cycle(false).unwrap().bounds, bounds[1]);
    assert_eq!(navigation.cycle(true).unwrap().bounds, bounds[0]);

    let mut navigation = Navigation::from_rects(&bounds, "ab").unwrap();
    assert_eq!(navigation.cycle(true).unwrap().bounds, bounds[1]);
    assert!(Navigation::from_rects(&[], "ab").unwrap().cycle(false).is_none());
  }
}
