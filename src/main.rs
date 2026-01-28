use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "kact")]
#[command(about = "Keyboard-driven cursor actuator", long_about = None)]
struct Cli {
  /// Path to configuration file
  #[arg(short, long, default_value = "kact.toml")]
  config: PathBuf,

  /// Generate default configuration file
  #[arg(short, long)]
  generate_config: bool,

  /// Log level (error, warn, info, debug, trace)
  #[arg(short, long, default_value = "info")]
  log_level: String,
}

fn main() -> Result<()> {
  let cli = Cli::parse();
  println!("options: {:?}", cli);
  Ok(())
}
