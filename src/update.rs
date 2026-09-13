use crate::installation::Release;
use anyhow::{Context, Result, bail, ensure};
use serde::Deserialize;
use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const REPOSITORY: &str = "iamgideonidoko/kact";
static TEMPORARY_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Deserialize)]
struct ReleaseResponse {
  tag_name: String,
  draft: bool,
  prerelease: bool,
}

pub struct LatestRelease {
  pub tag: String,
}

pub fn latest_release() -> Result<LatestRelease> {
  let output = Command::new("curl")
    .args([
      "--proto",
      "=https",
      "--tlsv1.2",
      "--fail",
      "--location",
      "--silent",
      "--show-error",
      "--header",
      "Accept: application/vnd.github+json",
      "--header",
      concat!("User-Agent: kact/", env!("CARGO_PKG_VERSION")),
      &format!("https://api.github.com/repos/{REPOSITORY}/releases/latest"),
    ])
    .output()
    .context("cannot run curl; install curl to check for updates")?;
  ensure!(
    output.status.success(),
    "Could not check for updates: {}",
    String::from_utf8_lossy(&output.stderr).trim()
  );
  parse_latest_release(&output.stdout)
}

fn parse_latest_release(body: &[u8]) -> Result<LatestRelease> {
  let release: ReleaseResponse = serde_json::from_slice(body).context("GitHub returned an invalid release response")?;
  ensure!(
    !release.draft && !release.prerelease,
    "GitHub did not return a stable release"
  );
  ensure!(valid_tag(&release.tag_name), "GitHub returned an invalid release tag");
  Ok(LatestRelease { tag: release.tag_name })
}

pub fn install(release: &Release, latest: &LatestRelease) -> Result<()> {
  let asset = format!("kact-{}.tar.gz", platform()?);
  let directory = TemporaryDirectory::new()?;
  let checksums = directory.path.join("SHA256SUMS");
  let archive = directory.path.join(&asset);
  let download_base = format!("https://github.com/{REPOSITORY}/releases/download/{}", latest.tag);
  download(&format!("{download_base}/SHA256SUMS"), &checksums)?;
  download(&format!("{download_base}/{asset}"), &archive)?;
  verify_checksum(&archive, &checksums, &asset)?;
  let extraction = directory.path.join("extract");
  fs::create_dir(&extraction)?;
  run(
    Command::new("tar")
      .arg("-xzf")
      .arg(&archive)
      .arg("--strip-components=1")
      .arg("-C")
      .arg(&extraction),
    "cannot extract release archive",
  )?;
  let binary = extraction.join("kact");
  ensure!(binary.is_file(), "Release archive does not contain a kact binary");
  verify_binary(&binary, &latest.tag)?;
  replace_binary(release.binary(), &binary)?;
  release.refresh_receipt()?;
  Ok(())
}

fn platform() -> Result<&'static str> {
  match (std::env::consts::OS, std::env::consts::ARCH) {
    ("macos", "aarch64") => Ok("macos-arm64"),
    ("macos", "x86_64") => Ok("macos-x86_64"),
    ("linux", "x86_64") => Ok("linux-x86_64"),
    (os, arch) => bail!("No release archive is available for {os} {arch}"),
  }
}

fn download(url: &str, destination: &Path) -> Result<()> {
  run(
    Command::new("curl")
      .args([
        "--proto",
        "=https",
        "--tlsv1.2",
        "--fail",
        "--location",
        "--silent",
        "--show-error",
      ])
      .arg(url)
      .arg("--output")
      .arg(destination),
    "release download failed",
  )
}

fn verify_checksum(archive: &Path, checksums: &Path, asset: &str) -> Result<()> {
  let contents = fs::read_to_string(checksums)?;
  let expected = contents
    .lines()
    .find_map(|line| {
      line
        .split_once("  ")
        .filter(|(_, name)| *name == asset)
        .map(|(hash, _)| hash)
    })
    .filter(|hash| valid_sha256(hash))
    .context("No valid checksum for the release archive")?;
  let mut command = if cfg!(target_os = "macos") {
    let mut command = Command::new("shasum");
    command.args(["-a", "256"]);
    command
  } else {
    Command::new("sha256sum")
  };
  let output = command.arg(archive).output().context("cannot run a SHA-256 tool")?;
  ensure!(output.status.success(), "Could not calculate release checksum");
  let checksum = String::from_utf8_lossy(&output.stdout);
  let actual = checksum.split_whitespace().next().unwrap_or("");
  ensure!(actual == expected, "Checksum verification failed for {asset}");
  Ok(())
}

fn verify_binary(binary: &Path, tag: &str) -> Result<()> {
  let output = Command::new(binary)
    .arg("--version")
    .output()
    .context("cannot execute release binary")?;
  ensure!(output.status.success(), "Release binary could not report its version");
  let version = String::from_utf8_lossy(&output.stdout);
  ensure!(
    version.trim_end().ends_with(&tag[1..]),
    "Release binary version does not match {tag}"
  );
  Ok(())
}

fn replace_binary(destination: &Path, source: &Path) -> Result<()> {
  let temporary = temporary_path(destination, "update");
  let mut input = File::open(source)?;
  let mut output = OpenOptions::new()
    .write(true)
    .create_new(true)
    .mode(0o755)
    .open(&temporary)
    .with_context(|| format!("cannot create {}", temporary.display()))?;
  std::io::copy(&mut input, &mut output)?;
  output.sync_all()?;
  drop(output);
  fs::rename(&temporary, destination).with_context(|| format!("cannot replace {}", destination.display()))?;
  Ok(())
}

fn run(command: &mut Command, context: &str) -> Result<()> {
  let output = command.output().with_context(|| context.to_string())?;
  ensure!(
    output.status.success(),
    "{context}: {}",
    String::from_utf8_lossy(&output.stderr).trim()
  );
  Ok(())
}

fn temporary_path(path: &Path, suffix: &str) -> PathBuf {
  path.with_file_name(format!(
    ".kact-{suffix}-{}-{}",
    std::process::id(),
    TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
  ))
}

fn valid_sha256(value: &str) -> bool {
  value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_tag(value: &str) -> bool {
  let Some(version) = value.strip_prefix('v') else {
    return false;
  };
  let core = version.split(['-', '+']).next().unwrap_or_default();
  core.split('.').count() == 3
    && core
      .split('.')
      .all(|part| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit()))
    && version
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

struct TemporaryDirectory {
  path: PathBuf,
}

impl TemporaryDirectory {
  fn new() -> Result<Self> {
    for _ in 0..100 {
      let path = std::env::temp_dir().join(format!(
        "kact-update-{}-{}",
        std::process::id(),
        TEMPORARY_ID.fetch_add(1, Ordering::Relaxed)
      ));
      match fs::create_dir(&path) {
        Ok(()) => return Ok(Self { path }),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
        Err(error) => return Err(error.into()),
      }
    }
    bail!("Could not create a temporary update directory")
  }
}

impl Drop for TemporaryDirectory {
  fn drop(&mut self) {
    let _ = fs::remove_dir_all(&self.path);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::os::unix::fs::PermissionsExt;

  #[test]
  fn only_accepts_stable_versioned_release_metadata() {
    assert_eq!(
      parse_latest_release(br#"{"tag_name":"v1.2.3","draft":false,"prerelease":false}"#)
        .unwrap()
        .tag,
      "v1.2.3"
    );
    assert!(parse_latest_release(br#"{"tag_name":"v1.2.3-rc.1","draft":false,"prerelease":true}"#).is_err());
    assert!(parse_latest_release(br#"{"tag_name":"main","draft":false,"prerelease":false}"#).is_err());
    assert!(parse_latest_release(br#"{"tag_name":"v1.2","draft":false,"prerelease":false}"#).is_err());
  }

  #[test]
  fn validates_sha256_values() {
    assert!(valid_sha256(&"a".repeat(64)));
    assert!(valid_sha256(&"A1".repeat(32)));
    assert!(!valid_sha256("not-a-checksum"));
  }

  #[test]
  fn tags_and_checksum_records_reject_ambiguous_input() {
    for tag in ["v0.0.0", "v12.34.56+build.7", "v1.2.3-rc.1"] {
      assert!(valid_tag(tag), "{tag}");
    }
    for tag in ["1.2.3", "v1.2", "v1.2.3/evil", "v1.2.3_evil", "v..1.2.3"] {
      assert!(!valid_tag(tag), "{tag}");
    }
    let directory = std::env::temp_dir().join(format!("kact-update-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir(&directory).unwrap();
    let archive = directory.join("kact.tar.gz");
    let checksums = directory.join("SHA256SUMS");
    fs::write(&archive, b"contents").unwrap();
    fs::write(&checksums, format!("{}  other.tar.gz\n", "a".repeat(64))).unwrap();
    assert!(verify_checksum(&archive, &checksums, "kact.tar.gz").is_err());
    fs::write(&checksums, format!("{}  kact.tar.gz\n", "z".repeat(64))).unwrap();
    assert!(verify_checksum(&archive, &checksums, "kact.tar.gz").is_err());
    let mut command = if cfg!(target_os = "macos") {
      let mut command = std::process::Command::new("shasum");
      command.args(["-a", "256"]);
      command
    } else {
      std::process::Command::new("sha256sum")
    };
    let checksum = command.arg(&archive).output().unwrap();
    let checksum = String::from_utf8(checksum.stdout).unwrap();
    fs::write(
      &checksums,
      format!("{}  kact.tar.gz\n", checksum.split_whitespace().next().unwrap()),
    )
    .unwrap();
    assert!(verify_checksum(&archive, &checksums, "kact.tar.gz").is_ok());
    let _ = fs::remove_dir_all(directory);
  }

  #[test]
  fn replacement_is_atomic_and_keeps_executable_mode() {
    let directory = std::env::temp_dir().join(format!("kact-replace-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir(&directory).unwrap();
    let source = directory.join("source");
    let destination = directory.join("kact");
    fs::write(&source, b"new binary").unwrap();
    fs::write(&destination, b"old binary").unwrap();
    replace_binary(&destination, &source).unwrap();
    assert_eq!(fs::read(&destination).unwrap(), b"new binary");
    assert_eq!(fs::metadata(&destination).unwrap().permissions().mode() & 0o777, 0o755);
    assert!(!temporary_path(&destination, "update").exists());
    let _ = fs::remove_dir_all(directory);
  }
}
