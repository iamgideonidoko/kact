use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
  #[error("IO error: {0}")]
  Io(#[from] std::io::Error),

  #[error("TOML parse error: {0}")]
  TomlParse(#[from] toml::de::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
