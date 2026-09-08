use anyhow::{Context, Result, bail};
use clap::{CommandFactory, Parser};
use kact::command::{Cli, Command, ConfigCommand};
use kact::config::Config;
use kact::ipc::{self, Reply, Server};
use kact::runtime::{ConfigWatcher, Runtime, bindings::Bindings};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

fn main() -> Result<()> {
  let cli = Cli::parse();
  let config_path = cli.config.clone().unwrap_or_else(Config::default_path);
  let socket = cli.socket.clone().unwrap_or_else(ipc::default_socket);
  let Some(command) = cli.command.clone() else {
    Cli::command().print_help()?;
    println!();
    return Ok(());
  };
  match command {
    Command::Config { command } => match command {
      ConfigCommand::Path => println!("{}", config_path.display()),
      ConfigCommand::Init => {
        initialize_config(&config_path)?;
        println!("Created {}", config_path.display());
      }
      ConfigCommand::Check => {
        load_config(&config_path, true)?;
        println!("Configuration is valid");
      }
    },
    Command::Doctor => {
      load_config(&config_path, false)?;
      println!("Config: {} (valid)", config_path.display());
      let trusted = kact::platform::accessibility_trusted(false);
      println!("Accessibility: {}", if trusted { "granted" } else { "not granted" });
      println!(
        "Service: {}",
        if ipc::send(&socket, &Command::Status).is_ok() {
          "running"
        } else {
          "not running"
        }
      );
      if !trusted {
        bail!("Enable Accessibility for Kact in System Settings > Privacy & Security");
      }
    }
    Command::Service { command } => kact::service::configure(command, &absolute(&config_path)?, &absolute(&socket)?)?,
    Command::Setup => setup(&cli, &config_path, &socket)?,
    Command::Uninstall { purge } => uninstall(&config_path, &socket, purge)?,
    Command::Start => start(&cli, &config_path, &socket)?,
    Command::Daemon => daemon(&cli, &config_path, &socket)?,
    command => print_reply(ipc::send(&socket, &command)?)?,
  }
  Ok(())
}

fn print_reply(reply: Reply) -> Result<()> {
  if !reply.ok {
    bail!("{}", reply.data.as_str().unwrap_or("command failed"));
  }
  println!("{}", serde_json::to_string_pretty(&reply.data)?);
  Ok(())
}

fn initialize_config(path: &Path) -> Result<()> {
  if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
    std::fs::create_dir_all(parent)?;
  }
  let mut file = OpenOptions::new()
    .write(true)
    .create_new(true)
    .open(path)
    .with_context(|| format!("cannot create {}; existing files are never overwritten", path.display()))?;
  file.write_all(toml::to_string_pretty(&Config::default())?.as_bytes())?;
  Ok(())
}

fn load_config(path: &Path, required: bool) -> Result<Config> {
  let config = match Config::load(path) {
    Ok(config) => config,
    Err(kact::Error::Io(e)) if !required && e.kind() == std::io::ErrorKind::NotFound => Config::default(),
    Err(error) => return Err(error).with_context(|| format!("invalid configuration at {}", path.display())),
  };
  Bindings::new(&config)?.alphabet(&config)?;
  Ok(config)
}

fn start(cli: &Cli, config_path: &Path, socket: &Path) -> Result<()> {
  if ipc::send(socket, &Command::Status).is_ok() {
    println!("Kact is already running");
    return Ok(());
  }
  load_config(config_path, false)?;
  if !config_path.exists() {
    initialize_config(config_path)?;
  }
  if !kact::platform::accessibility_trusted(true) {
    bail!("Enable Accessibility for Kact in System Settings > Privacy & Security, then run `kact start` again");
  }
  let log_path = config_path.with_extension("log");
  let log = OpenOptions::new().create(true).append(true).open(&log_path)?;
  let mut command = std::process::Command::new(std::env::current_exe()?);
  command
    .arg("daemon")
    .arg("--config")
    .arg(absolute(config_path)?)
    .arg("--socket")
    .arg(absolute(socket)?);
  if let Some(level) = &cli.log_level {
    command.arg("--log-level").arg(level);
  }
  command
    .stdin(Stdio::null())
    .stdout(Stdio::from(log.try_clone()?))
    .stderr(Stdio::from(log));
  // Detach from the invoking terminal without forking AppKit state in-process.
  use std::os::unix::process::CommandExt;
  unsafe {
    command.pre_exec(|| {
      if libc::setsid() < 0 {
        Err(std::io::Error::last_os_error())
      } else {
        Ok(())
      }
    });
  }
  let mut child = command.spawn()?;
  let deadline = Instant::now() + Duration::from_secs(5);
  loop {
    if let Some(status) = child.try_wait()? {
      bail!("Kact exited ({status}); see {}", log_path.display());
    }
    if let Ok(reply) = ipc::send(socket, &Command::Status)
      && reply.ok
    {
      println!("Kact started. Use `kact activate grid`. Logs: {}", log_path.display());
      return Ok(());
    }
    if Instant::now() >= deadline {
      let _ = child.kill();
      let _ = child.wait();
      bail!("Kact did not become ready; see {}", log_path.display());
    }
    std::thread::sleep(Duration::from_millis(50));
  }
}

fn setup(cli: &Cli, config_path: &Path, socket: &Path) -> Result<()> {
  if !config_path.exists() {
    initialize_config(config_path)?;
    println!("Created {}", config_path.display());
  }
  load_config(config_path, true)?;
  if !kact::platform::accessibility_trusted(true) {
    open_accessibility_settings();
    println!("Grant Accessibility to Kact in System Settings, then run `kact setup` again.");
    return Ok(());
  }
  start(cli, config_path, socket)?;
  println!("Ready. Run `kact activate grid` to place the cursor with labels.");
  Ok(())
}

fn uninstall(config_path: &Path, socket: &Path, purge: bool) -> Result<()> {
  let ownership = kact::installation::current_release()?;
  #[cfg(target_os = "macos")]
  kact::service::configure(
    kact::command::ServiceCommand::Uninstall,
    &absolute(config_path)?,
    &absolute(socket)?,
  )?;
  let _ = ipc::send(socket, &Command::Quit);
  match ownership {
    kact::installation::Ownership::Managed(release) => {
      println!("Removed {}", kact::installation::remove(release)?.display());
    }
    kact::installation::Ownership::NotManaged(message) => println!("{message} Left the binary unchanged."),
  }
  if purge {
    remove_if_present(config_path)?;
    remove_if_present(&config_path.with_extension("log"))?;
    remove_if_present(socket)?;
    println!("Removed configuration, logs, and socket.");
  } else {
    println!("Configuration and logs were kept. Run `kact uninstall --purge` to remove them.");
  }
  Ok(())
}

fn remove_if_present(path: &Path) -> Result<()> {
  match std::fs::remove_file(path) {
    Ok(()) => Ok(()),
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(error) => Err(error.into()),
  }
}

#[cfg(target_os = "macos")]
fn open_accessibility_settings() {
  let _ = std::process::Command::new("open")
    .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
    .status();
}

#[cfg(not(target_os = "macos"))]
fn open_accessibility_settings() {}

fn absolute(path: &Path) -> Result<PathBuf> {
  Ok(if path.is_absolute() {
    path.into()
  } else {
    std::env::current_dir()?.join(path)
  })
}

fn daemon(cli: &Cli, config_path: &Path, socket: &Path) -> Result<()> {
  if !config_path.exists() {
    initialize_config(config_path)?;
  }
  let config = load_config(config_path, false)?;
  let level = cli
    .log_level
    .as_deref()
    .unwrap_or(&config.system.log_level)
    .parse::<tracing::level_filters::LevelFilter>()
    .context("invalid log level")?;
  use tracing_subscriber::prelude::*;
  let (filter, log_handle) = tracing_subscriber::reload::Layer::new(level);
  tracing_subscriber::registry()
    .with(filter)
    .with(tracing_subscriber::fmt::layer().with_target(false))
    .init();
  let mut applied_log_level = level;
  let server = Server::bind(socket)?;
  let stopped = Arc::new(AtomicBool::new(false));
  signal_hook::flag::register(signal_hook::consts::SIGINT, Arc::clone(&stopped))?;
  signal_hook::flag::register(signal_hook::consts::SIGTERM, Arc::clone(&stopped))?;
  if !kact::platform::accessibility_trusted(true) {
    bail!("Enable Accessibility for Kact in System Settings > Privacy & Security, then reopen Kact");
  }
  let mut runtime = Runtime::new(config, config_path.into())?;
  let watcher = ConfigWatcher::new(config_path)
    .map_err(|error| tracing::warn!(%error, "Automatic config reload unavailable"))
    .ok();
  tracing::info!(socket = %socket.display(), "Kact ready");
  while !stopped.load(Ordering::Relaxed) && !runtime.quitting {
    if server.failed() {
      bail!("command transport stopped");
    }
    for request in server.requests.try_iter().take(32) {
      let reply = if Instant::now() >= request.expires {
        Reply::error("command expired before execution")
      } else {
        match runtime.execute(request.command) {
          Ok(value) => Reply::success(value),
          Err(error) => Reply::error(error),
        }
      };
      let _ = request.reply.send(reply);
      if runtime.quitting {
        break;
      }
    }
    let pending_config = watcher.as_ref().and_then(ConfigWatcher::try_recv);
    if runtime.config.system.hot_reload
      && let Some(config) = pending_config
      && let Err(error) = runtime.update_config(config)
    {
      tracing::error!(%error, "Keeping previous config");
    }
    let desired_level = cli
      .log_level
      .as_deref()
      .unwrap_or(&runtime.config.system.log_level)
      .parse::<tracing::level_filters::LevelFilter>()?;
    if desired_level != applied_log_level {
      log_handle.reload(desired_level)?;
      applied_log_level = desired_level;
    }
    if runtime.quitting {
      break;
    }
    runtime.poll()?;
    std::thread::sleep(runtime.sleep_duration());
  }
  // Dropping runtime releases held buttons and stops input before socket removal.
  drop(runtime);
  drop(server);
  Ok(())
}
