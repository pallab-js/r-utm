//! QEMU process launcher for VibeVisor

use crate::vibe_core::config::{Architecture, NetworkMode, VmConfig};
use crate::vibe_core::security::{generate_secure_password, validate_command_path};
use crate::vibe_core::Result;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};

/// Maximum password length for SPICE (QEMU limit)
const MAX_SPICE_PASSWORD_LEN: usize = 64;

/// Result of launching a VM: the child process plus the runtime connection
/// details the UI/control plane needs to interact with it.
pub struct LaunchResult {
    pub child: Child,
    pub spice_port: u16,
    pub spice_password: String,
    pub qmp_socket_path: PathBuf,
}

pub struct QemuLauncher {
    qemu_path: PathBuf,
    /// Directory where QMP sockets are created. This must outlive the launched
    /// QEMU process (it is owned by `AppState`), otherwise control-plane
    /// commands (stop/pause/resume) cannot reach the running VM.
    socket_dir: PathBuf,
}

impl QemuLauncher {
    /// Create a launcher that writes QMP sockets into `socket_dir`.
    ///
    /// The directory is created with `0o700` permissions if it does not exist.
    pub fn new(qemu_path: PathBuf, socket_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&socket_dir).map_err(|e| {
            crate::vibe_core::Error::QemuProcess(format!(
                "Failed to create QMP socket directory: {}",
                e
            ))
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = socket_dir.metadata() {
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o700);
                let _ = std::fs::set_permissions(&socket_dir, permissions);
            }
        }

        Ok(Self {
            qemu_path,
            socket_dir,
        })
    }

    /// Get the QMP socket path for a given VM.
    pub fn qmp_socket_path(&self, vm_id: &str) -> PathBuf {
        self.socket_dir.join(format!("{}.sock", vm_id))
    }

    pub async fn launch(&self, config: &VmConfig) -> Result<LaunchResult> {
        let spice_port = Self::find_free_port()?;
        let spice_password = generate_secure_password(MAX_SPICE_PASSWORD_LEN)?;
        let qmp_socket_path = self.qmp_socket_path(&config.id);

        let args = self.build_args(config, spice_port, &spice_password, &qmp_socket_path)?;

        tracing::info!("Launching QEMU with {} arguments", args.len());

        let mut cmd = Command::new(&self.qemu_path);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| crate::vibe_core::Error::QemuProcess(e.to_string()))?;

        Ok(LaunchResult {
            child,
            spice_port,
            spice_password,
            qmp_socket_path,
        })
    }

    fn build_args(
        &self,
        config: &VmConfig,
        spice_port: u16,
        spice_password: &str,
        qmp_socket_path: &Path,
    ) -> Result<Vec<String>> {
        let mut args = Vec::new();

        args.extend([
            "-name".to_string(),
            config.name.clone(),
            "-uuid".to_string(),
            config.id.clone(),
        ]);

        match config.architecture {
            Architecture::Aarch64 => {
                args.extend(["-machine".to_string(), "virt".to_string()]);
                args.extend(["-accel".to_string(), "hvf".to_string()]);

                if let Some(cpu) = &config.hardware.cpu_model {
                    args.extend(["-cpu".to_string(), cpu.clone()]);
                } else {
                    args.extend(["-cpu".to_string(), "host".to_string()]);
                }
            }
        }

        args.extend([
            "-smp".to_string(),
            config.hardware.cpu_cores.to_string(),
            "-m".to_string(),
            config.hardware.memory_mb.to_string(),
        ]);

        // Securely handle disk paths - validate and use separate arguments
        let disk_path = validate_command_path(&config.storage.disk_path)?;
        let disk_path_str = disk_path.to_str().ok_or_else(|| {
            crate::vibe_core::Error::Storage("Disk path contains invalid characters".to_string())
        })?;

        args.extend([
            "-drive".to_string(),
            format!("file={},if=virtio,format=qcow2,aio=threads", disk_path_str),
        ]);

        if let Some(boot_iso) = &config.storage.boot_iso {
            let iso_path = validate_command_path(boot_iso)?;
            let iso_path_str = iso_path.to_str().ok_or_else(|| {
                crate::vibe_core::Error::Storage("ISO path contains invalid characters".to_string())
            })?;

            args.extend([
                "-drive".to_string(),
                format!("file={},media=cdrom", iso_path_str),
                "-boot".to_string(),
                "d".to_string(),
            ]);
        }

        if config.network.enabled {
            Self::validate_network_mode(&config.network.mode)?;
            match config.network.mode {
                NetworkMode::Nat => {
                    args.extend(["-net".to_string(), "nic,model=virtio".to_string()]);
                    args.extend(["-net".to_string(), "user".to_string()]);
                }
                NetworkMode::Bridged => {
                    args.extend(["-net".to_string(), "nic,model=virtio".to_string()]);
                    args.extend(["-net".to_string(), "bridge,br=bridge0".to_string()]);
                }
                NetworkMode::Shared => {
                    args.extend(["-net".to_string(), "nic,model=virtio".to_string()]);
                    args.extend(["-net".to_string(), "vmnet-shared".to_string()]);
                }
            }
        }

        // SPICE: run a headless QEMU exposing a localhost-only SPICE server on a
        // real, discoverable TCP port. The previous code combined `-display
        // spice-app` (QEMU's own embedded viewer) with a `-spice` TCP server,
        // which is contradictory and left the TCP port unusable by the UI's
        // SPICE client. We use `-display none` so the TCP server is the single
        // source of truth, and ticketing is enabled so the generated password
        // is actually enforced.
        args.extend([
            "-display".to_string(),
            "none".to_string(),
            "-spice".to_string(),
            format!(
                "addr=127.0.0.1,port={},password={},disable-ticketing=off",
                spice_port, spice_password
            ),
        ]);

        let qmp_path_str = qmp_socket_path.to_str().ok_or_else(|| {
            crate::vibe_core::Error::QmpComm("Invalid QMP socket path".to_string())
        })?;

        args.extend([
            "-qmp".to_string(),
            format!("unix:{},server,nowait", qmp_path_str),
        ]);

        Ok(args)
    }

    /// Validate that a requested network mode is usable on this host before
    /// handing it to QEMU, so we fail with a clear message instead of an
    /// opaque QEMU error.
    fn validate_network_mode(mode: &NetworkMode) -> Result<()> {
        match mode {
            // NAT (user networking) always works without host setup.
            NetworkMode::Nat => Ok(()),
            NetworkMode::Bridged => {
                // Bridged mode requires a pre-configured `bridge0` interface and
                // a qemu-bridge-helper allow-list entry. Verify the bridge exists.
                if cfg!(target_os = "macos") && !Self::interface_exists("bridge0") {
                    return Err(crate::vibe_core::Error::Network(
                        "Bridged networking requires a 'bridge0' interface. \
                         Create one (e.g. via `ifconfig bridge0 create`) and add it \
                         to QEMU's bridge allow-list, or use NAT networking."
                            .to_string(),
                    ));
                }
                Ok(())
            }
            NetworkMode::Shared => {
                // vmnet-shared requires the vmnet entitlement and typically root.
                // We can't fully verify entitlements here, but surface the
                // requirement rather than silently producing a broken VM.
                if !cfg!(target_os = "macos") {
                    return Err(crate::vibe_core::Error::Network(
                        "Shared (vmnet) networking is only available on macOS.".to_string(),
                    ));
                }
                Ok(())
            }
        }
    }

    /// Best-effort check for a named network interface on the host.
    fn interface_exists(name: &str) -> bool {
        std::process::Command::new("ifconfig")
            .arg(name)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    fn find_free_port() -> Result<u16> {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| {
            crate::vibe_core::Error::Network(format!("Failed to bind a free SPICE port: {}", e))
        })?;
        let port = listener
            .local_addr()
            .map_err(|e| {
                crate::vibe_core::Error::Network(format!("Failed to read local address: {}", e))
            })?
            .port();
        Ok(port)
    }
}

pub async fn watch_process(mut child: Child) -> Result<()> {
    if let Some(stdout) = child.stdout.take() {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            tracing::debug!("QEMU stdout: {}", line);
        }
    }

    if let Some(stderr) = child.stderr.take() {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            tracing::debug!("QEMU stderr: {}", line);
        }
    }

    let status = child
        .wait()
        .await
        .map_err(|e| crate::vibe_core::Error::QemuProcess(e.to_string()))?;

    tracing::info!("QEMU process exited with: {:?}", status);

    Ok(())
}
