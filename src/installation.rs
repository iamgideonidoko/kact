use anyhow::{Context, Result, bail};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

struct Receipt {
  path: PathBuf,
  device: u64,
  inode: u64,
}

pub enum Ownership {
  Managed(Release),
  NotManaged(String),
}

pub struct Release {
  binary: PathBuf,
  receipt: PathBuf,
}

pub fn current_release() -> Result<Ownership> {
  let receipt_path = receipt_path();
  let receipt = match read_receipt(&receipt_path)? {
    Some(receipt) => receipt,
    None => {
      return Ok(Ownership::NotManaged(
        "No release-script installation receipt was found.".into(),
      ));
    }
  };
  let current = std::env::current_exe()?.canonicalize()?;
  if current != receipt.path {
    return Ok(Ownership::NotManaged(
      "This executable does not match the release-script installation receipt.".into(),
    ));
  }
  let metadata = fs::metadata(&current)?;
  if metadata.dev() != receipt.device || metadata.ino() != receipt.inode {
    return Ok(Ownership::NotManaged(
      "The installed binary changed after the release installer ran.".into(),
    ));
  }
  Ok(Ownership::Managed(Release {
    binary: current,
    receipt: receipt_path,
  }))
}

pub fn remove(release: Release) -> Result<PathBuf> {
  fs::remove_file(&release.binary)?;
  fs::remove_file(release.receipt)?;
  Ok(release.binary)
}

pub fn receipt_path() -> PathBuf {
  let state = std::env::var_os("XDG_STATE_HOME")
    .filter(|path| Path::new(path).is_absolute())
    .map(PathBuf::from)
    .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/state")))
    .unwrap_or_else(|| PathBuf::from(".local/state"));
  state.join("kact/install-receipt")
}

fn read_receipt(path: &Path) -> Result<Option<Receipt>> {
  let metadata = match fs::symlink_metadata(path) {
    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
    Err(error) => return Err(error.into()),
    Ok(metadata) => metadata,
  };
  if !metadata.is_file() || metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
    bail!("Refusing to trust an unsafe installation receipt at {}", path.display());
  }
  let mut location = None;
  let mut device = None;
  let mut inode = None;
  for line in fs::read_to_string(path)?.lines() {
    if let Some(value) = line.strip_prefix("path=") {
      location = Some(PathBuf::from(value));
    } else if let Some(value) = line.strip_prefix("device=") {
      device = Some(value.parse().context("invalid installation receipt device")?);
    } else if let Some(value) = line.strip_prefix("inode=") {
      inode = Some(value.parse().context("invalid installation receipt inode")?);
    }
  }
  let path = location.context("installation receipt has no path")?;
  if !path.is_absolute() {
    bail!("installation receipt path must be absolute");
  }
  Ok(Some(Receipt {
    path,
    device: device.context("installation receipt has no device")?,
    inode: inode.context("installation receipt has no inode")?,
  }))
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::fs::PermissionsExt;

  #[test]
  fn receipt_requires_a_complete_absolute_record() {
    let dir = std::env::temp_dir().join(format!("kact-installation-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let receipt = dir.join("receipt");
    fs::write(&receipt, "path=relative\ndevice=1\ninode=2\n").unwrap();
    fs::set_permissions(&receipt, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(read_receipt(&receipt).is_err());
    fs::write(&receipt, "path=/tmp/kact\ndevice=1\ninode=2\n").unwrap();
    let parsed = read_receipt(&receipt).unwrap().unwrap();
    assert_eq!(parsed.path, Path::new("/tmp/kact"));
    fs::remove_file(receipt).unwrap();
    fs::remove_dir(dir).unwrap();
  }
}
