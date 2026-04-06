//! QEMU process launcher for VibeVisor

use crate::vibe_core::config::{VmConfig, Architecture, NetworkMode};
use crate::vibe_core::security::{generate_secure_password, validate_command_path};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::io::{AsyncBufReadExt, BufReader};
use crate::vibe_core::Result;

/// Maximum password length for SPICE (QEMU limit)
const MAX_SPICE_PASSWORD_LEN: usize = 64;

pub struct QemuLauncher {
    qemu_path: PathBuf,
    qmp_socket_dir: tempfile::TempDir,
}

impl QemuLauncher {
    pub fn new(qemu_path: PathBuf) -> Result<Self> {
        // Create secure temporary directory for QMP sockets
        let qmp_socket_dir = tempfile::Builder::new()
            .prefix("vibevisor-qmp-")
            .tempdir()
            .map_err(|e| crate::vibe_core::Error::QemuProcess(
                format!("Failed to create secure temp directory: {}", e)
            ))?;
        
        // Set restrictive permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = qmp_socket_dir.path().metadata() {
                let mut permissions = metadata.permissions();
                permissions.set_mode(0o700);
                let _ = std::fs::set_permissions(qmp_socket_dir.path(), permissions);
            }
        }
        
        Ok(Self { 
            qemu_path,
            qmp_socket_dir,
        })
    }
    
    /// Get the QMP socket path for this launcher
    pub fn qmp_socket_path(&self, vm_id: &str) -> PathBuf {
        self.qmp_socket_dir.path().join(format!("{}.sock", vm_id))
    }

    pub async fn launch(&self, config: &VmConfig) -> Result<Child> {
        let args = self.build_args(config)?;

        tracing::info!("Launching QEMU with {} arguments", args.len());

        let mut cmd = Command::new(&self.qemu_path);
        cmd.args(&args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let child = cmd.spawn()
            .map_err(|e| crate::vibe_core::Error::QemuProcess(e.to_string()))?;

        Ok(child)
    }

    fn build_args(&self, config: &VmConfig) -> Result<Vec<String>> {
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

        // Build drive argument safely
        args.extend([
            "-drive".to_string(),
            format!(
                "file={},if=virtio,format=qcow2,aio=threads",
                disk_path_str
            ),
        ]);

        if let Some(boot_iso) = &config.storage.boot_iso {
            let iso_path = validate_command_path(boot_iso)?;
            let iso_path_str = iso_path.to_str().ok_or_else(|| {
                crate::vibe_core::Error::Storage("ISO path contains invalid characters".to_string())
            })?;
            
            args.extend([
                "-drive".to_string(),
                format!(
                    "file={},media=cdrom",
                    iso_path_str
                ),
                "-boot".to_string(),
                "d".to_string(),
            ]);
        }

        if config.network.enabled {
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

        // Generate cryptographically secure SPICE password
        let spice_port = self.find_free_port();
        let spice_password = generate_secure_password(MAX_SPICE_PASSWORD_LEN);

        args.extend([
            "-display".to_string(),
            "spice-app".to_string(),
            "-spice".to_string(),
            format!(
                "addr=127.0.0.1:{},password={},disable-ticketing=off",
                spice_port, spice_password
            ),
        ]);

        // Use secure QMP socket path
        let qmp_socket_path = self.qmp_socket_path(&config.id);
        let qmp_path_str = qmp_socket_path.to_str().ok_or_else(|| {
            crate::vibe_core::Error::QmpComm("Invalid QMP socket path".to_string())
        })?;
        
        args.extend([
            "-qmp".to_string(),
            format!("{},server,nowait", qmp_path_str)
        ]);

        Ok(args)
    }

    fn find_free_port(&self) -> u16 {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0")
            .expect("Failed to bind to port for SPICE");
        listener.local_addr().expect("Failed to get local address").port()
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

    let status = child.wait().await
        .map_err(|e| crate::vibe_core::Error::QemuProcess(e.to_string()))?;
    
    tracing::info!("QEMU process exited with: {:?}", status);
    
    Ok(())
}
