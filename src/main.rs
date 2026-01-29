use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing_subscriber;

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

  // Initialize logging
  let log_level = match cli.log_level.as_str() {
    "error" => tracing::Level::ERROR,
    "warn" => tracing::Level::WARN,
    "info" => tracing::Level::INFO,
    "debug" => tracing::Level::DEBUG,
    "trace" => tracing::Level::TRACE,
    _ => tracing::Level::INFO,
  };

  tracing_subscriber::fmt()
    .with_max_level(log_level)
    .with_target(false)
    .init();

  // Generate config if requested
  if cli.generate_config {
    generate_default_config(&cli.config)?;
    return Ok(());
  }

  // Load configuration
  let _config = Config::load_or_default(&cli.config);
  tracing::info!("Configuration loaded from {:?}", cli.config);

  Ok(())
}

fn generate_default_config(path: &PathBuf) -> Result<()> {
  let config = Config::default();
  let toml_str = toml::to_string_pretty(&config)?;
  std::fs::write(path, toml_str)?;
  println!("Generated default configuration at {:?}", path);
  Ok(())
}
