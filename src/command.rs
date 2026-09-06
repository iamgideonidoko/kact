use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "kact", version, about = "Control the cursor from your keyboard or shell")]
pub struct Cli {
  /// Override the configuration file
  #[arg(short, long, global = true)]
  pub config: Option<PathBuf>,
  /// Override the local control socket
  #[arg(long, global = true)]
  pub socket: Option<PathBuf>,
  /// Logging verbosity
  #[arg(long, global = true)]
  pub log_level: Option<String>,
  #[command(subcommand)]
  pub command: Option<Command>,
}

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "kebab-case", deny_unknown_fields)]
pub enum Command {
  /// Start the background service (safe to repeat)
  Start,
  /// Run the service in the foreground
  Daemon,
  /// Configure automatic startup at login (macOS)
  Service {
    #[command(subcommand)]
    command: ServiceCommand,
  },
  /// Show service and navigation state
  Status,
  /// Stop the background service and release held buttons
  Quit,
  /// Generate or validate configuration
  Config {
    #[command(subcommand)]
    command: ConfigCommand,
  },
  /// Activate a navigation mode
  Activate {
    #[arg(value_enum, default_value = "freestyle")]
    mode: NavigationMode,
  },
  /// Hide navigation and release held input
  Deactivate,
  /// Toggle navigation
  Toggle {
    #[arg(value_enum, default_value = "freestyle")]
    mode: NavigationMode,
  },
  /// Clear the label prefix, or leave navigation when it is empty
  Cancel,
  /// Stop motion and release every held mouse button
  Stop,
  /// Move by a pixel offset
  Move {
    #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
    dx: f64,
    #[arg(long, default_value_t = 0.0, allow_hyphen_values = true)]
    dy: f64,
  },
  /// Move to global screen coordinates
  MoveTo {
    #[arg(long, allow_hyphen_values = true)]
    x: f64,
    #[arg(long, allow_hyphen_values = true)]
    y: f64,
  },
  /// Start continuous movement; pair with move-stop on key release
  MoveStart {
    #[arg(value_enum)]
    direction: Heading,
  },
  /// Stop movement in one direction
  MoveStop {
    #[arg(value_enum)]
    direction: Heading,
  },
  /// Set the continuous movement speed
  Speed {
    #[arg(value_enum)]
    mode: Speed,
  },
  /// Jump to a screen edge, center, or successive corners
  Jump {
    #[arg(value_enum)]
    target: JumpTarget,
  },
  /// Click at the cursor; modifiers are comma-separated
  Click {
    #[arg(long, value_enum, default_value = "left")]
    button: Button,
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=3))]
    count: u8,
    #[arg(long, value_delimiter = ',')]
    modifiers: Vec<String>,
  },
  /// Hold a mouse button for dragging
  ButtonDown {
    #[arg(long, value_enum, default_value = "left")]
    button: Button,
    #[arg(long, value_delimiter = ',')]
    modifiers: Vec<String>,
  },
  /// Release a held mouse button
  ButtonUp {
    #[arg(long, value_enum, default_value = "left")]
    button: Button,
    #[arg(long, value_delimiter = ',')]
    modifiers: Vec<String>,
  },
  /// Scroll in pixels (positive dy scrolls up; positive dx scrolls left)
  Scroll {
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    dx: i32,
    #[arg(long, default_value_t = 0, allow_hyphen_values = true)]
    dy: i32,
  },
  /// Type a target label through the command interface
  Select { label: String },
  /// Remove the last typed label character
  Backspace,
  /// Subdivide the last selected grid cell
  Refine,
  /// Change overlay presentation for this session
  Show {
    #[arg(value_enum)]
    setting: Presentation,
  },
  /// Reload configuration without restarting
  Reload,
  /// Check permissions and configuration without moving the cursor
  Doctor,
}

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConfigCommand {
  /// Create a default config without overwriting an existing file
  Init,
  /// Validate the config and all shortcut actions
  Check,
  /// Print the resolved configuration path
  Path,
}

#[derive(Subcommand, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServiceCommand {
  /// Enable startup at the next login
  Install,
  /// Disable login startup and unload the managed service
  Uninstall,
}

macro_rules! values {
  ($name:ident { $($variant:ident),+ $(,)? }) => {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Serialize, Deserialize)]
    #[serde(rename_all = "kebab-case")]
    pub enum $name { $($variant),+ }
  };
}
values!(NavigationMode {
  Grid,
  Elements,
  Freestyle
});
values!(Heading { Up, Down, Left, Right });
values!(Speed { Normal, Precise, Fast });
values!(Button { Left, Right, Middle });
values!(JumpTarget {
  Top,
  Bottom,
  Left,
  Right,
  Center,
  Cycle
});
values!(Presentation {
  GridLines,
  Labels,
  Larger,
  Smaller,
  MoreContrast,
  LessContrast
});

impl Command {
  pub fn from_binding(binding: &str) -> Result<Self, String> {
    // Bindings are action arguments, never shell programs.
    let cli =
      Cli::try_parse_from(std::iter::once("kact").chain(binding.split_whitespace())).map_err(|e| e.to_string())?;
    let command = cli.command.ok_or("binding needs an action")?;
    if matches!(
      command,
      Self::Start | Self::Daemon | Self::Config { .. } | Self::Service { .. } | Self::Doctor
    ) || cli.config.is_some()
      || cli.socket.is_some()
      || cli.log_level.is_some()
    {
      return Err("binding must be a cursor or service-control action".into());
    }
    Ok(command)
  }

  pub fn validate(&self) -> Result<(), String> {
    match self {
      Self::Move { dx, dy } | Self::MoveTo { x: dx, y: dy } => {
        if !dx.is_finite() || !dy.is_finite() || dx.abs() > 1_000_000.0 || dy.abs() > 1_000_000.0 {
          return Err("coordinates must be finite and within ±1000000".into());
        }
      }
      Self::Click { count, modifiers, .. } => {
        if !(1..=3).contains(count) {
          return Err("click count must be 1–3".into());
        }
        validate_modifiers(modifiers)?;
      }
      Self::ButtonDown { modifiers, .. } | Self::ButtonUp { modifiers, .. } => validate_modifiers(modifiers)?,
      Self::Scroll { dx, dy } if dx.unsigned_abs() > 100_000 || dy.unsigned_abs() > 100_000 => {
        return Err("scroll amount must be within ±100000".into());
      }
      Self::Select { label } if label.is_empty() || label.len() > 64 || !label.is_ascii() => {
        return Err("label must contain 1–64 ASCII characters".into());
      }
      _ => {}
    }
    Ok(())
  }
}

fn validate_modifiers(modifiers: &[String]) -> Result<(), String> {
  if modifiers.iter().any(|s| {
    !matches!(
      s.as_str(),
      "shift" | "ctrl" | "control" | "alt" | "option" | "cmd" | "command" | "super"
    )
  }) {
    return Err("modifiers must be shift, ctrl, alt, or cmd".into());
  }
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn commands_validate_untrusted_input() {
    assert!(Command::from_binding("activate grid").is_ok());
    assert!(Command::from_binding("move --dx -20").is_ok());
    assert!(Command::from_binding("daemon").is_err());
    assert!(Command::Move { dx: f64::NAN, dy: 0.0 }.validate().is_err());
    assert!(
      Command::Click {
        button: Button::Left,
        count: 0,
        modifiers: vec![]
      }
      .validate()
      .is_err()
    );
    assert!(serde_json::from_str::<Command>(r#"{"action":"move","dx":1,"dy":0,"extra":1}"#).is_err());
  }
}
