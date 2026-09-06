use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
  // #[error("Configuration error: {0}")]
  // Config(String),
  //
  //
  // #[error("Motion engine error: {0}")]
  // Motion(String),
  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),

  #[error("TOML parse error: {0}")]
  TomlParse(#[from] toml::de::Error),

  #[error("Platform error: {0}")]
  Platform(String),

  #[error("Channel send error")]
  ChannelSend,
  // #[error("Channel receive error")]
  // ChannelRecv,
  //
  // #[error("Emergency stop triggered")]
  // EmergencyStop,
}

pub type Result<T> = std::result::Result<T, Error>;
