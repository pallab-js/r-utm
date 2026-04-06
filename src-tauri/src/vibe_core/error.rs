//! Error types for VibeVisor

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("QEMU process error: {0}")]
    QemuProcess(String),

    #[error("QMP communication error: {0}")]
    QmpComm(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("VM not found: {0}")]
    VmNotFound(String),

    #[error("Invalid VM state transition from {from} to {to}")]
    InvalidStateTransition { from: String, to: String },
}

pub type Result<T> = std::result::Result<T, Error>;
