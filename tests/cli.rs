use std::path::PathBuf;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
  Command::new(env!("CARGO_BIN_EXE_kact")).args(args).output().unwrap()
}
fn temporary() -> PathBuf {
  std::env::temp_dir().join(format!(
    "kact-cli-{}-{}",
    std::process::id(),
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  ))
}
#[test]
fn config_init_is_safe_and_checks_binding_actions() {
  let dir = temporary();
  let path = dir.join("kact.toml");
  let path = path.to_str().unwrap();
  assert!(run(&["--config", path, "config", "init"]).status.success());
  assert!(run(&["--config", path, "config", "check"]).status.success());
  let original = std::fs::read(path).unwrap();
  assert!(!run(&["--config", path, "config", "init"]).status.success());
  assert_eq!(std::fs::read(path).unwrap(), original);
  std::fs::write(path, "[keybindings.global]\n'ctrl+g' = 'nonsense'\n").unwrap();
  assert!(!run(&["--config", path, "config", "check"]).status.success());
  std::fs::write(path, "[motion]\nmax_speed = nan\n").unwrap();
  assert!(!run(&["--config", path, "config", "check"]).status.success());
  std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn commands_are_discoverable_and_missing_service_is_actionable() {
  let help = run(&["--help"]);
  assert!(help.status.success());
  let text = String::from_utf8(help.stdout).unwrap();
  for name in [
    "setup",
    "uninstall",
    "update",
    "activate",
    "move-start",
    "click",
    "select",
    "doctor",
  ] {
    assert!(text.contains(name));
  }
  let missing = temporary().join("control.sock");
  let output = run(&["--socket", missing.to_str().unwrap(), "status"]);
  assert!(!output.status.success());
  assert!(String::from_utf8(output.stderr).unwrap().contains("kact start"));
}
