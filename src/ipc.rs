//! A private, bounded local command transport. No shell evaluation or network listener.
use crate::command::Command;
use crossbeam_channel::{Receiver, Sender, bounded};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

const MAX_MESSAGE: u64 = 16 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
  pub ok: bool,
  pub data: Value,
}
impl Reply {
  pub fn success(data: impl Into<Value>) -> Self {
    Self {
      ok: true,
      data: data.into(),
    }
  }
  pub fn error(error: impl ToString) -> Self {
    Self {
      ok: false,
      data: Value::String(error.to_string()),
    }
  }
}

pub struct Request {
  pub command: Command,
  pub reply: Sender<Reply>,
  pub expires: Instant,
}

pub struct Server {
  pub requests: Receiver<Request>,
  stopped: Arc<AtomicBool>,
  worker: Option<JoinHandle<()>>,
  path: PathBuf,
  _lock: File,
  socket_identity: (u64, u64),
}

pub fn default_socket() -> PathBuf {
  let uid = unsafe { libc::geteuid() };
  let base = std::env::var_os("XDG_RUNTIME_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(std::env::temp_dir);
  base.join(format!("kact-{uid}")).join("control.sock")
}

fn private_directory(path: &Path) -> io::Result<()> {
  use std::os::unix::fs::DirBuilderExt;
  match fs::DirBuilder::new().mode(0o700).create(path) {
    Ok(()) => {}
    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
    Err(e) => return Err(e),
  }
  let metadata = fs::symlink_metadata(path)?;
  if !metadata.is_dir() || metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o077 != 0 {
    return Err(io::Error::new(
      io::ErrorKind::PermissionDenied,
      "socket directory must be owned by you, not a symlink, and mode 0700",
    ));
  }
  Ok(())
}

impl Server {
  pub fn bind(path: &Path) -> io::Result<Self> {
    let parent = path
      .parent()
      .filter(|p| !p.as_os_str().is_empty())
      .ok_or_else(|| io::Error::other("socket needs a private parent directory"))?;
    private_directory(parent)?;
    let lock = OpenOptions::new()
      .read(true)
      .write(true)
      .create(true)
      .truncate(false)
      .mode(0o600)
      .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
      .open(path.with_extension("lock"))?;
    let lock_metadata = lock.metadata()?;
    if !lock_metadata.is_file() || lock_metadata.uid() != unsafe { libc::geteuid() } || lock_metadata.nlink() != 1 {
      return Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        "invalid socket lock file",
      ));
    }
    if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
      return Err(io::Error::new(
        io::ErrorKind::AddrInUse,
        "Kact is already running on this socket",
      ));
    }
    if let Ok(metadata) = fs::symlink_metadata(path) {
      if !metadata.file_type().is_socket() || metadata.uid() != unsafe { libc::geteuid() } {
        return Err(io::Error::new(
          io::ErrorKind::PermissionDenied,
          "refusing to replace a non-socket or foreign socket",
        ));
      }
      match UnixStream::connect(path) {
        Ok(_) => {
          return Err(io::Error::new(
            io::ErrorKind::AddrInUse,
            "socket already has a listener",
          ));
        }
        Err(error) if matches!(error.kind(), io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound) => {}
        Err(error) => return Err(error),
      }
      fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    let metadata = fs::symlink_metadata(path)?;
    let socket_identity = (metadata.dev(), metadata.ino());
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    listener.set_nonblocking(true)?;
    let (tx, requests) = bounded::<Request>(32);
    let stopped = Arc::new(AtomicBool::new(false));
    let stop = Arc::clone(&stopped);
    let worker = thread::Builder::new().name("kact-commands".into()).spawn(move || {
      while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
          Ok((mut stream, _)) => {
            let result = serve(&mut stream, &tx, &stop);
            if let Err(error) = result {
              tracing::debug!(%error, "Command connection ended");
            }
          }
          Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
            let mut ready = libc::pollfd {
              fd: listener.as_raw_fd(),
              events: libc::POLLIN,
              revents: 0,
            };
            // Sleep until a client arrives, with a bounded shutdown check.
            unsafe {
              libc::poll(&mut ready, 1, 50);
            }
          }
          Err(error) => {
            tracing::error!(%error, "Command listener failed");
            stop.store(true, Ordering::Relaxed);
          }
        }
      }
    })?;
    Ok(Self {
      requests,
      stopped,
      worker: Some(worker),
      path: path.into(),
      _lock: lock,
      socket_identity,
    })
  }

  pub fn failed(&self) -> bool {
    self.stopped.load(Ordering::Relaxed)
  }
}

fn serve(stream: &mut UnixStream, tx: &Sender<Request>, stopped: &AtomicBool) -> io::Result<()> {
  // Darwin inherits the listener's nonblocking flag on accepted sockets.
  stream.set_nonblocking(false)?;
  stream.set_read_timeout(Some(Duration::from_millis(250)))?;
  stream.set_write_timeout(Some(Duration::from_millis(250)))?;
  let reply = match read_message::<Value>(stream, Duration::from_millis(250)).and_then(decode_command) {
    Ok(command) => {
      if let Err(error) = command.validate() {
        Reply::error(error)
      } else {
        let (reply_tx, reply_rx) = bounded(1);
        let expires = Instant::now() + Duration::from_secs(4);
        if tx
          .try_send(Request {
            command,
            reply: reply_tx,
            expires,
          })
          .is_err()
        {
          Reply::error("command queue is full")
        } else {
          let deadline = expires;
          loop {
            match reply_rx.recv_timeout(Duration::from_millis(50)) {
              Ok(reply) => break reply,
              Err(_) if stopped.load(Ordering::Relaxed) || std::time::Instant::now() >= deadline => {
                break Reply::error("service did not respond");
              }
              Err(crossbeam_channel::RecvTimeoutError::Disconnected) => break Reply::error("service stopped"),
              Err(_) => {}
            }
          }
        }
      }
    }
    Err(error) => Reply::error(error),
  };
  write_message(stream, &reply)
}

fn decode_command(value: Value) -> io::Result<Command> {
  let command: Command =
    serde_json::from_value(value.clone()).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
  // Serde ignores unknown fields on internally tagged unit variants; check their envelope too.
  let canonical = serde_json::to_value(&command)?;
  if let (Some(input), Some(fields)) = (value.as_object(), canonical.as_object())
    && let Some(unknown) = input.keys().find(|key| !fields.contains_key(*key))
  {
    return Err(io::Error::new(
      io::ErrorKind::InvalidData,
      format!("unknown command field: {unknown}"),
    ));
  }
  Ok(command)
}

fn read_message<T: serde::de::DeserializeOwned>(mut stream: &UnixStream, timeout: Duration) -> io::Result<T> {
  let deadline = Instant::now() + timeout;
  let mut bytes = Vec::new();
  loop {
    let remaining = deadline
      .checked_duration_since(Instant::now())
      .filter(|remaining| !remaining.is_zero())
      .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "command frame deadline exceeded"))?;
    stream.set_read_timeout(Some(remaining))?;
    let mut chunk = [0u8; 1024];
    let count = match stream.read(&mut chunk) {
      Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
      result => result?,
    };
    if count == 0 {
      return Err(io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "unterminated command frame",
      ));
    }
    let newline = chunk[..count].iter().position(|byte| *byte == b'\n');
    let end = newline.map_or(count, |index| index + 1);
    if bytes.len() + end > MAX_MESSAGE as usize {
      return Err(io::Error::new(io::ErrorKind::InvalidData, "oversized command frame"));
    }
    bytes.extend_from_slice(&chunk[..end]);
    if newline.is_some() {
      return serde_json::from_slice(&bytes).map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error));
    }
  }
}

fn write_message(stream: &mut UnixStream, value: &impl Serialize) -> io::Result<()> {
  let mut bytes = serde_json::to_vec(value)?;
  bytes.push(b'\n');
  if bytes.len() > MAX_MESSAGE as usize {
    return Err(io::Error::new(io::ErrorKind::InvalidData, "oversized command frame"));
  }
  stream.write_all(&bytes)
}

pub fn send(path: &Path, command: &Command) -> io::Result<Reply> {
  command
    .validate()
    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
  let mut stream = UnixStream::connect(path).map_err(|e| {
    io::Error::new(
      e.kind(),
      format!("cannot reach Kact at {}: {e}; run `kact start`", path.display()),
    )
  })?;
  stream.set_read_timeout(Some(Duration::from_secs(6)))?;
  stream.set_write_timeout(Some(Duration::from_secs(1)))?;
  write_message(&mut stream, command)?;
  read_message(&stream, Duration::from_secs(6))
}

impl Drop for Server {
  fn drop(&mut self) {
    self.stopped.store(true, Ordering::Relaxed);
    if let Some(worker) = self.worker.take() {
      let _ = worker.join();
    }
    if fs::symlink_metadata(&self.path).is_ok_and(|metadata| (metadata.dev(), metadata.ino()) == self.socket_identity) {
      let _ = fs::remove_file(&self.path);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn exclusive_socket_and_roundtrip() {
    let directory = std::env::temp_dir().join(format!(
      "kact-test-{}-{}",
      std::process::id(),
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    let path = directory.join("control.sock");
    let server = Server::bind(&path).unwrap();
    assert!(Server::bind(&path).is_err());
    let client = thread::spawn(move || send(&path, &Command::Status).unwrap());
    let request = server.requests.recv_timeout(Duration::from_secs(2)).unwrap();
    assert!(request.expires > Instant::now());
    assert!(request.expires <= Instant::now() + Duration::from_secs(4));
    request.reply.send(Reply::success("ready")).unwrap();
    assert_eq!(client.join().unwrap().data, "ready");
    drop(server);
    let _ = fs::remove_dir_all(directory);
  }
  #[test]
  fn rejects_unterminated_frame() {
    let (mut writer, reader) = UnixStream::pair().unwrap();
    writer.write_all(b"{}").unwrap();
    drop(writer);
    assert!(read_message::<Command>(&reader, Duration::from_millis(250)).is_err());
  }

  fn socket_path() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    std::env::temp_dir()
      .join(format!(
        "kact-ipc-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
          .duration_since(std::time::UNIX_EPOCH)
          .unwrap()
          .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
      ))
      .join("control.sock")
  }

  #[test]
  fn slow_frames_have_total_deadline() {
    let (mut writer, reader) = UnixStream::pair().unwrap();
    let writer = thread::spawn(move || {
      for _ in 0..100 {
        if writer.write_all(b" ").is_err() {
          break;
        }
        thread::sleep(Duration::from_millis(20));
      }
    });
    let start = Instant::now();
    assert!(read_message::<Command>(&reader, Duration::from_millis(100)).is_err());
    assert!(start.elapsed() < Duration::from_secs(1));
    drop(reader);
    writer.join().unwrap();
  }

  #[test]
  fn rejects_oversized_and_invalid_frames_without_dispatch() {
    let path = socket_path();
    let server = Server::bind(&path).unwrap();
    for frame in [
      b"{invalid}\n".to_vec(),
      b"{\"action\":\"stop\",\"extra\":1}\n".to_vec(),
      vec![b'x'; MAX_MESSAGE as usize + 1],
    ] {
      let mut stream = UnixStream::connect(&path).unwrap();
      stream.write_all(&frame).unwrap();
      let reply: Reply = read_message(&stream, Duration::from_secs(1)).unwrap();
      assert!(!reply.ok);
      assert!(server.requests.is_empty());
    }
    drop(server);
    fs::remove_dir_all(path.parent().unwrap()).unwrap();
  }

  #[test]
  fn stale_socket_recovery_and_live_socket_protection() {
    let path = socket_path();
    private_directory(path.parent().unwrap()).unwrap();
    let listener = UnixListener::bind(&path).unwrap();
    assert!(Server::bind(&path).is_err());
    assert!(path.exists());
    drop(listener);
    let server = Server::bind(&path).unwrap();
    drop(server);
    assert!(!path.exists());
    let server = Server::bind(&path).unwrap();
    // Cleanup must never unlink a socket that replaced ours.
    fs::remove_file(&path).unwrap();
    let replacement = UnixListener::bind(&path).unwrap();
    drop(server);
    assert!(path.exists());
    drop(replacement);
    fs::remove_dir_all(path.parent().unwrap()).unwrap();
  }

  #[test]
  fn private_directory_and_non_socket_are_protected() {
    let path = socket_path();
    private_directory(path.parent().unwrap()).unwrap();
    fs::write(&path, "keep").unwrap();
    assert!(Server::bind(&path).is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "keep");
    fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o755)).unwrap();
    assert!(Server::bind(&path).is_err());
    fs::remove_dir_all(path.parent().unwrap()).unwrap();
  }
}
