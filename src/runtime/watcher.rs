use crate::config::Config;
use crate::{Error, Result};
use crossbeam_channel::{Receiver, Sender, bounded};
use notify::{Event, RecursiveMode, Watcher};
use std::path::Path;
use std::thread;
use std::time::Duration;

pub struct ConfigWatcher {
  _watcher: notify::RecommendedWatcher,
  config_rx: Receiver<Config>,
}

impl ConfigWatcher {
  pub fn new(config_path: &Path) -> Result<Self> {
    let (tx, rx) = bounded::<notify::Result<Event>>(10);
    let (config_tx, config_rx) = bounded::<Config>(1);

    let mut watcher = notify::recommended_watcher(move |res| {
      let _ = tx.send(res);
    })
    .map_err(|e| Error::Platform(format!("Failed to create file watcher: {}", e)))?;

    watcher
      .watch(config_path, RecursiveMode::NonRecursive)
      .map_err(|e| Error::Platform(format!("Failed to watch config file: {}", e)))?;

    let path = config_path.to_path_buf();
    thread::Builder::new()
      .name("kact-config-watcher".to_string())
      .spawn(move || {
        Self::watch_loop(rx, config_tx, &path);
      })
      .map_err(|e| Error::Platform(format!("Failed to spawn watcher thread: {}", e)))?;

    Ok(Self {
      _watcher: watcher,
      config_rx,
    })
  }

  fn watch_loop(event_rx: Receiver<notify::Result<Event>>, config_tx: Sender<Config>, path: &Path) {
    while let Ok(Ok(event)) = event_rx.recv() {
      if event.kind.is_modify() {
        tracing::info!("Config file changed, reloading...");

        // Small delay to ensure file is fully written
        thread::sleep(Duration::from_millis(100));

        match Config::load(path) {
          Ok(config) => {
            if config_tx.send(config).is_err() {
              tracing::warn!("Config channel closed, stopping watcher");
              break;
            }
          }
          Err(e) => {
            tracing::error!("Failed to reload config: {}", e);
          }
        }
      }
    }
  }

  pub fn try_recv(&self) -> Option<Config> {
    self.config_rx.try_recv().ok()
  }
}
