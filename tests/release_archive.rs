use std::path::PathBuf;
use std::process::Command;

fn temporary(name: &str) -> PathBuf {
  std::env::temp_dir().join(format!(
    "kact-{name}-{}-{}",
    std::process::id(),
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos()
  ))
}

#[test]
fn release_archive_is_reproducible_and_normalized() {
  let directory = temporary("archive");
  let source = directory.join("kact-v1-test");
  let one = directory.join("one.tar.gz");
  let two = directory.join("two.tar.gz");
  std::fs::create_dir_all(source.join("nested")).unwrap();
  std::fs::write(source.join("README.md"), "read me\n").unwrap();
  std::fs::write(source.join("nested/kact"), "binary\n").unwrap();
  let run = |archive: &std::path::Path| {
    let status = Command::new("python3")
      .args([
        "scripts/create_archive.py",
        source.to_str().unwrap(),
        archive.to_str().unwrap(),
      ])
      .env("SOURCE_DATE_EPOCH", "1234567890")
      .status()
      .unwrap();
    assert!(status.success());
  };
  run(&one);
  run(&two);
  assert_eq!(std::fs::read(&one).unwrap(), std::fs::read(&two).unwrap());
  let output = Command::new("tar")
    .args(["-tzf", one.to_str().unwrap()])
    .output()
    .unwrap();
  assert!(output.status.success());
  assert_eq!(
    String::from_utf8(output.stdout).unwrap(),
    "kact-v1-test/\nkact-v1-test/README.md\nkact-v1-test/nested/\nkact-v1-test/nested/kact\n"
  );
  std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn release_scripts_reject_unknown_arguments_without_building() {
  for script in ["scripts/install.sh", "scripts/package-release.sh"] {
    let output = Command::new("bash").args([script, "--unknown"]).output().unwrap();
    assert_eq!(output.status.code(), Some(2), "{script}");
  }
  let invalid_version = Command::new("bash")
    .args(["scripts/install.sh", "--version", "not-a-version"])
    .output()
    .unwrap();
  assert_eq!(invalid_version.status.code(), Some(2));
  let unsupported_target = Command::new("bash")
    .args(["scripts/package-release.sh", "--target", "unsupported"])
    .output()
    .unwrap();
  assert_eq!(unsupported_target.status.code(), Some(1));
}
