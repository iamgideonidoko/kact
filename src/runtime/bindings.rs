use crate::command::Command;
use crate::config::Config;
use crate::platform::canonical_shortcut;
use anyhow::{Result, bail};
use std::collections::HashMap;

pub struct Bindings {
  pub global: HashMap<String, Command>,
  pub local: HashMap<String, Command>,
}

impl Bindings {
  pub fn new(config: &Config) -> Result<Self> {
    let mut result = Self {
      global: HashMap::new(),
      local: HashMap::new(),
    };
    if config.keybindings.navigation_enabled {
      for (key, action) in [
        ("escape", "cancel"),
        ("cmd+period", "cancel"),
        ("ctrl+g", "cancel"),
        ("cmd+h", "deactivate"),
        ("backspace", "backspace"),
        ("enter", "click"),
        ("equal", "button-down"),
        ("backslash", "click --count 2"),
        ("leftbracket", "click --button middle"),
        ("rightbracket", "click --button right"),
        ("ctrl+equal", "show grid-lines"),
        ("ctrl+shift+equal", "show labels"),
        ("cmd+shift+equal", "show larger"),
        ("cmd+shift+minus", "show smaller"),
        ("cmd+equal", "show more-contrast"),
        ("cmd+minus", "show less-contrast"),
      ] {
        result.insert_local(key, action)?;
      }
      for (key, axis, sign, jump) in [
        ("up", "dy", -1, "top"),
        ("down", "dy", 1, "bottom"),
        ("left", "dx", -1, "left"),
        ("right", "dx", 1, "right"),
      ] {
        result.insert_local(key, &format!("move --{axis} {}", sign * 10))?;
        result.insert_local(&format!("alt+{key}"), &format!("move --{axis} {}", sign * 100))?;
        result.insert_local(&format!("cmd+{key}"), &format!("jump {jump}"))?;
        result.insert_local(&format!("shift+{key}"), &format!("scroll --{axis} {}", -sign * 40))?;
      }
      if config.keybindings.preset == "vi" {
        for (key, axis, sign, edge) in [
          ("k", "dy", -1, "top"),
          ("j", "dy", 1, "bottom"),
          ("h", "dx", -1, "left"),
          ("l", "dx", 1, "right"),
        ] {
          result.insert_local(key, &format!("move --{axis} {}", sign * 10))?;
          result.insert_local(&format!("ctrl+{key}"), &format!("move --{axis} {}", sign * 100))?;
          result.insert_local(&format!("shift+{key}"), &format!("jump {edge}"))?;
        }
        for (key, action) in [
          ("shift+m", "jump cycle"),
          ("ctrl+b", "scroll --dy 40"),
          ("ctrl+f", "scroll --dy -40"),
          ("ctrl+i", "scroll --dx 40"),
          ("ctrl+a", "scroll --dx -40"),
        ] {
          result.insert_local(key, action)?;
        }
      } else {
        for (key, action) in [
          ("ctrl+p", "move --dy -10"),
          ("ctrl+n", "move --dy 10"),
          ("ctrl+b", "move --dx -10"),
          ("ctrl+f", "move --dx 10"),
          ("alt+a", "move --dy -100"),
          ("alt+e", "move --dy 100"),
          ("alt+b", "move --dx -100"),
          ("alt+f", "move --dx 100"),
          ("alt+shift+comma", "jump top"),
          ("alt+shift+period", "jump bottom"),
          ("ctrl+a", "jump left"),
          ("ctrl+e", "jump right"),
          ("ctrl+l", "jump cycle"),
          ("shift+p", "scroll --dy 40"),
          ("shift+n", "scroll --dy -40"),
          ("shift+b", "scroll --dx 40"),
          ("shift+f", "scroll --dx -40"),
        ] {
          result.insert_local(key, action)?;
        }
      }
      for bits in 1..16 {
        let mods: Vec<_> = ["ctrl", "alt", "shift", "cmd"]
          .into_iter()
          .enumerate()
          .filter_map(|(i, name)| (bits & (1 << i) != 0).then_some(name))
          .collect();
        result.insert_local(
          &format!("{}+enter", mods.join("+")),
          &format!("click --modifiers {}", mods.join(",")),
        )?;
      }
    }
    // Validate disabled bindings too, so enabling them cannot expose latent errors.
    let mut global = HashMap::new();
    for (key, action) in &config.keybindings.global {
      let key = canonical_shortcut(key)?;
      if !key.contains('+') {
        bail!("global shortcut `{key}` needs a modifier");
      }
      let action = Command::from_binding(action).map_err(anyhow::Error::msg)?;
      action.validate().map_err(anyhow::Error::msg)?;
      if global.insert(key.clone(), action).is_some() {
        bail!("duplicate global shortcut `{key}`");
      }
    }
    let mut local = HashMap::new();
    for (key, action) in &config.keybindings.local {
      let key = canonical_shortcut(key)?;
      let action = Command::from_binding(action).map_err(anyhow::Error::msg)?;
      action.validate().map_err(anyhow::Error::msg)?;
      if local.insert(key.clone(), action).is_some() {
        bail!("duplicate local shortcut `{key}`");
      }
    }
    if config.keybindings.enabled {
      result.global = global;
      result.local.extend(local);
    }
    Ok(result)
  }

  fn insert_local(&mut self, key: &str, action: &str) -> Result<()> {
    self.local.insert(
      canonical_shortcut(key)?,
      Command::from_binding(action).map_err(anyhow::Error::msg)?,
    );
    Ok(())
  }

  pub fn alphabet(&self, config: &Config) -> Result<String> {
    let alphabet: String = config
      .navigation
      .alphabet
      .chars()
      .filter(|c| !self.local.contains_key(&c.to_string()))
      .collect();
    if alphabet.len() < 2 {
      bail!("local bindings leave fewer than two available label characters");
    }
    Ok(alphabet)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn shortcuts_are_opt_in_and_labels_remain_reachable() {
    let mut config = Config::default();
    assert!(Bindings::new(&config).unwrap().global.is_empty());
    config.keybindings.global.insert("space".into(), "toggle".into());
    assert!(Bindings::new(&config).is_err());
    config.keybindings.global.clear();
    config.keybindings.preset = "vi".into();
    let bindings = Bindings::new(&config).unwrap();
    assert!(!bindings.alphabet(&config).unwrap().contains('h'));
    config.keybindings.navigation_enabled = false;
    assert!(Bindings::new(&config).unwrap().local.is_empty());
  }
}
