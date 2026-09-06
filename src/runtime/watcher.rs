use crate::config::Config;
use crate::{Error, Result};
use crossbeam_channel::{Receiver, bounded};
use notify::{RecursiveMode, Watcher};
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

pub struct ConfigWatcher {
  _watcher: notify::RecommendedWatcher,
  config_rx: Receiver<Config>,
}
impl ConfigWatcher {
  pub fn new(config_path: &Path) -> Result<Self> {
    let path = if config_path.is_absolute() {
      config_path.to_path_buf()
    } else {
      std::env::current_dir()?.join(config_path)
    };
    let parent = path
      .parent()
      .ok_or_else(|| Error::Platform("config path needs a parent".into()))?
      .canonicalize()?;
    let path = parent.join(
      path
        .file_name()
        .ok_or_else(|| Error::Platform("config path needs a filename".into()))?,
    );
    let (events_tx, events_rx) = bounded(1);
    let (config_tx, config_rx) = bounded(1);
    let old_config = config_rx.clone();
    let watched = path.clone();
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| match event {
      Ok(event)
        if (event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove())
          && event.paths.iter().any(|p| p == &watched) =>
      {
        let _ = events_tx.try_send(());
      }
      Err(error) => tracing::warn!(%error, "Config watcher error"),
      _ => {}
    })
    .map_err(|e| Error::Platform(e.to_string()))?;
    // Watch the parent to survive editors' atomic rename/replacement saves.
    watcher
      .watch(&parent, RecursiveMode::NonRecursive)
      .map_err(|e| Error::Platform(e.to_string()))?;
    thread::Builder::new().name("kact-config".into()).spawn(move || {
      while events_rx.recv().is_ok() {
        let mut quiet_until = Instant::now() + Duration::from_millis(150);
        while let Ok(()) = events_rx.recv_timeout(quiet_until.saturating_duration_since(Instant::now())) {
          quiet_until = Instant::now() + Duration::from_millis(150);
        }
        match Config::load(&path) {
          Ok(config) => {
            let _ = old_config.try_recv();
            if config_tx.try_send(config).is_err() {
              break;
            }
          }
          Err(error) => tracing::warn!(%error, "Keeping previous config"),
        }
      }
    })?;
    Ok(Self {
      _watcher: watcher,
      config_rx,
    })
  }
  pub fn try_recv(&self) -> Option<Config> {
    self.config_rx.try_recv().ok()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn atomic_replacement_through_symlinked_directory_reloads() {
    let directory = std::env::temp_dir().join(format!("kact-watch-{}", std::process::id()));
    std::fs::create_dir_all(directory.join("real")).unwrap();
    std::os::unix::fs::symlink(directory.join("real"), directory.join("link")).unwrap();
    let path = directory.join("link/config.toml");
    std::fs::write(&path, "").unwrap();
    let watcher = ConfigWatcher::new(&path).unwrap();
    let replacement = directory.join("real/new.toml");
    std::fs::write(&replacement, "[motion]\ntarget_fps = 60\n").unwrap();
    std::fs::rename(replacement, &path).unwrap();
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
      if let Some(config) = watcher.try_recv() {
        assert_eq!(config.motion.target_fps, 60);
        break;
      }
      assert!(Instant::now() < deadline, "atomic save was not detected");
      std::thread::sleep(Duration::from_millis(20));
    }
    drop(watcher);
    std::fs::remove_dir_all(directory).unwrap();
  }
}
