use anyhow::Result;
use clap::Parser;
use kact::config::Config;
use kact::runtime::{ConfigWatcher, Runtime};
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

  if cli.generate_config {
    generate_default_config(&cli.config)?;
    return Ok(());
  }

  // Load configuration
  let config = Config::load_or_default(&cli.config);
  tracing::info!("Configuration loaded from {:?}", cli.config);

  // start config watcher if enabled
  let watcher = if config.system.hot_reload {
    match ConfigWatcher::new(&cli.config) {
      Ok(w) => {
        tracing::info!("Hot-reload enabled");
        Some(w)
      }
      Err(e) => {
        tracing::warn!("Failed to start config watcher: {}", e);
        None
      }
    }
  } else {
    None
  };

  let runtime = Runtime::new(config)?;
  tracing::info!("Kact runtime started");
  tracing::info!("Press Ctrl+C to stop");

  loop {
    if let Some(ref w) = watcher {
      if let Some(new_config) = w.try_recv() {
        tracing::info!("Reloading configuration");
        if let Err(e) = runtime.update_config(new_config) {
          tracing::error!("Failed to update config: {}", e);
        }
      }
    }

    std::thread::sleep(std::time::Duration::from_millis(100));

    if runtime.get_state().emergency_stop {
      tracing::warn!("Emergency stop detected, shutting down");
      break;
    }
  }

  runtime.shutdown()?;
  tracing::info!("Kact stopped");

  Ok(())
}

fn generate_default_config(path: &PathBuf) -> Result<()> {
  let config = Config::default();
  let toml_str = toml::to_string_pretty(&config)?;
  std::fs::write(path, toml_str)?;
  println!("Generated default configuration at {:?}", path);
  Ok(())
}
