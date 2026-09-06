use crate::command::ServiceCommand;
use anyhow::Result;
use std::path::Path;

pub fn configure(command: ServiceCommand, config: &Path, socket: &Path) -> Result<()> {
  #[cfg(target_os = "macos")]
  return macos(command, config, socket);
  #[cfg(not(target_os = "macos"))]
  {
    let _ = (command, config, socket);
    anyhow::bail!(
      "Automatic startup configuration currently requires macOS; run `kact daemon` from your session manager"
    )
  }
}

#[cfg(target_os = "macos")]
fn macos(command: ServiceCommand, config: &Path, socket: &Path) -> Result<()> {
  use anyhow::{Context, bail};
  use std::io::Write;
  use std::os::unix::fs::OpenOptionsExt;
  let home = std::env::var_os("HOME").context("HOME is not set")?;
  let path = Path::new(&home).join("Library/LaunchAgents/io.kact.agent.plist");
  match command {
    ServiceCommand::Install => {
      let executable = std::env::current_exe()?;
      let log = config.with_extension("log");
      let contents = launch_agent(&executable, config, socket, &log)?;
      std::fs::create_dir_all(path.parent().unwrap())?;
      if let Some(parent) = config.parent() {
        std::fs::create_dir_all(parent)?;
      }
      if let Ok(metadata) = std::fs::symlink_metadata(&path)
        && (!metadata.is_file() || !std::fs::read_to_string(&path)?.contains("<!-- Managed by kact -->"))
      {
        bail!("Refusing to overwrite unmanaged {}", path.display());
      }
      let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)?;
      file.write_all(contents.as_bytes())?;
      println!("Enabled startup at the next login. Run `kact start` for this session.");
    }
    ServiceCommand::Uninstall => {
      match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
          println!("Login startup is already disabled");
          return Ok(());
        }
        Err(error) => return Err(error.into()),
        Ok(metadata) if !metadata.is_file() => bail!("Refusing to remove a non-file launch agent"),
        Ok(_) => {}
      }
      if !std::fs::read_to_string(&path)?.contains("<!-- Managed by kact -->") {
        bail!("Refusing to remove an unmanaged launch agent");
      }
      let domain = format!("gui/{}/io.kact.agent", unsafe { libc::geteuid() });
      let loaded = std::process::Command::new("/bin/launchctl")
        .args(["print", &domain])
        .output()?;
      if loaded.status.success() {
        let output = std::process::Command::new("/bin/launchctl")
          .args(["bootout", &domain])
          .output()?;
        if !output.status.success() {
          bail!(
            "Cannot unload login service: {}",
            String::from_utf8_lossy(&output.stderr)
          );
        }
      }
      std::fs::remove_file(path)?;
      println!("Login startup disabled");
    }
  }
  Ok(())
}

#[cfg(any(target_os = "macos", test))]
fn launch_agent(executable: &Path, config: &Path, socket: &Path, log: &Path) -> Result<String> {
  fn escape(path: &Path) -> Result<String> {
    let text = path
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("service paths must be UTF-8"))?;
    anyhow::ensure!(
      !text.chars().any(char::is_control),
      "service paths cannot contain control characters"
    );
    Ok(
      text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;"),
    )
  }
  let args = [
    executable,
    Path::new("daemon"),
    Path::new("--config"),
    config,
    Path::new("--socket"),
    socket,
  ]
  .into_iter()
  .map(|path| escape(path).map(|text| format!("<string>{text}</string>")))
  .collect::<Result<Vec<_>>>()?
  .join("\n");
  let log = escape(log)?;
  Ok(format!(
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<!-- Managed by kact -->
<plist version="1.0"><dict>
<key>Label</key><string>io.kact.agent</string>
<key>ProgramArguments</key><array>{args}</array>
<key>RunAtLoad</key><true/>
<key>LimitLoadToSessionType</key><string>Aqua</string>
<key>StandardOutPath</key><string>{log}</string>
<key>StandardErrorPath</key><string>{log}</string>
</dict></plist>
"#
  ))
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn launch_arguments_are_xml_escaped_not_shell_interpolated() {
    let plist = launch_agent(
      Path::new("/Applications/A & B/kact"),
      Path::new("/tmp/config.toml"),
      Path::new("/tmp/control.sock"),
      Path::new("/tmp/log"),
    )
    .unwrap();
    assert!(plist.contains("<string>/Applications/A &amp; B/kact</string>"));
    assert!(plist.contains("<key>RunAtLoad</key><true/>"));
    assert!(launch_agent(Path::new("/tmp/\n"), Path::new("a"), Path::new("b"), Path::new("c")).is_err());
  }
}
