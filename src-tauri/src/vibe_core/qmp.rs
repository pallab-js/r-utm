//! QEMU Machine Protocol (QMP) client for VibeVisor

use crate::vibe_core::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VmState {
    Starting,
    Running,
    Paused,
    Stopped,
    Saving,
    Restoring,
    Dead,
    NoState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmpResponse {
    pub return_type: QmpReturnType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QmpReturnType {
    Ok(Value),
    Error { error: QmpError },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QmpError {
    pub class: String,
    pub desc: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStatusResponse {
    pub running: bool,
    pub status: String,
    pub singlestep: bool,
    pub pad: Option<String>,
}

pub struct QmpClient {
    socket_path: PathBuf,
}

impl QmpClient {
    /// Create a new QMP client with a specific socket path.
    pub fn with_socket_path(socket_path: PathBuf) -> Self {
        Self { socket_path }
    }

    pub async fn connect(&self) -> Result<()> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?;

        let mut reader = BufReader::new(&mut stream).lines();
        let greeting = reader
            .next_line()
            .await
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?
            .ok_or_else(|| crate::vibe_core::Error::QmpComm("Empty greeting".to_string()))?;

        tracing::debug!("QMP greeting: {}", greeting);

        let handshake = r#"{"execute": "qmp_capabilities"}"#;
        stream
            .write_all(handshake.as_bytes())
            .await
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?;
        stream
            .write_all(b"\n")
            .await
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?;

        Ok(())
    }

    pub async fn query_status(&self) -> Result<VmState> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?;

        let cmd = r#"{"execute": "query-status"}"#;
        stream
            .write_all(cmd.as_bytes())
            .await
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?;
        stream
            .write_all(b"\n")
            .await
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?;

        let mut reader = BufReader::new(stream).lines();
        let response = reader
            .next_line()
            .await
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?
            .ok_or_else(|| crate::vibe_core::Error::QmpComm("No response".to_string()))?;

        let status: QueryStatusResponse = serde_json::from_str(&response)
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?;

        let vm_state = match status.status.as_str() {
            "running" => VmState::Running,
            "paused" => VmState::Paused,
            "starting" => VmState::Starting,
            "stopped" => VmState::Stopped,
            "saving" => VmState::Saving,
            "restoring" => VmState::Restoring,
            "dead" => VmState::Dead,
            _ => VmState::NoState,
        };

        Ok(vm_state)
    }

    pub async fn stop(&self) -> Result<()> {
        self.send_command("stop").await
    }

    pub async fn cont(&self) -> Result<()> {
        self.send_command("cont").await
    }

    pub async fn quit(&self) -> Result<()> {
        self.send_command("quit").await
    }

    async fn send_command(&self, cmd: &str) -> Result<()> {
        let mut stream = UnixStream::connect(&self.socket_path)
            .await
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?;

        let json_cmd = format!(r#"{{"execute": "{}"}}"#, cmd);
        stream
            .write_all(json_cmd.as_bytes())
            .await
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?;
        stream
            .write_all(b"\n")
            .await
            .map_err(|e| crate::vibe_core::Error::QmpComm(e.to_string()))?;

        Ok(())
    }
}
