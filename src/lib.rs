pub mod config;
pub mod core;
pub mod error;
pub mod platform;
pub mod runtime;

pub use error::{Error, Result};
pub mod command;
pub mod desktop;
pub mod ipc;
pub mod service;
