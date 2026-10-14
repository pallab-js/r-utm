//! Storage management for VibeVisor (QCOW2, snapshots)

use crate::vibe_core::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct StorageManager {
    qemu_img_path: PathBuf,
}

impl StorageManager {
    pub fn new(qemu_img_path: PathBuf) -> Self {
        Self { qemu_img_path }
    }

    pub fn create_disk(&self, path: &Path, size_gb: u64) -> Result<()> {
        let path_str = path.to_str().ok_or_else(|| {
            crate::vibe_core::Error::Storage("Disk path contains invalid characters".to_string())
        })?;

        let output = Command::new(&self.qemu_img_path)
            .args([
                "create",
                "-f",
                "qcow2",
                "-o",
                &format!("size={}G", size_gb),
                path_str,
            ])
            .output()
            .map_err(|e| crate::vibe_core::Error::Storage(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::vibe_core::Error::Storage(stderr.to_string()));
        }

        tracing::info!("Created disk image at: {}", path.display());
        Ok(())
    }

    pub fn create_snapshot(&self, disk_path: &Path, snapshot_name: &str) -> Result<()> {
        let disk_path_str = disk_path.to_str().ok_or_else(|| {
            crate::vibe_core::Error::Storage("Disk path contains invalid characters".to_string())
        })?;

        let output = Command::new(&self.qemu_img_path)
            .args(["snapshot", "-c", snapshot_name, disk_path_str])
            .output()
            .map_err(|e| crate::vibe_core::Error::Storage(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::vibe_core::Error::Storage(stderr.to_string()));
        }

        tracing::info!(
            "Created snapshot '{}' for disk: {}",
            snapshot_name,
            disk_path.display()
        );
        Ok(())
    }

    pub fn list_snapshots(&self, disk_path: &Path) -> Result<Vec<SnapshotInfo>> {
        let disk_path_str = disk_path.to_str().ok_or_else(|| {
            crate::vibe_core::Error::Storage("Disk path contains invalid characters".to_string())
        })?;

        let output = Command::new(&self.qemu_img_path)
            .args(["snapshot", "-l", disk_path_str])
            .output()
            .map_err(|e| crate::vibe_core::Error::Storage(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::vibe_core::Error::Storage(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut snapshots = Vec::new();

        for line in stdout.lines().skip(2) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                snapshots.push(SnapshotInfo {
                    id: parts[0].to_string(),
                    name: parts[1].to_string(),
                    vm_clock: parts.get(2).map(|s| s.to_string()).unwrap_or_default(),
                });
            }
        }

        Ok(snapshots)
    }

    pub fn delete_snapshot(&self, disk_path: &Path, snapshot_name: &str) -> Result<()> {
        let disk_path_str = disk_path.to_str().ok_or_else(|| {
            crate::vibe_core::Error::Storage("Disk path contains invalid characters".to_string())
        })?;

        let output = Command::new(&self.qemu_img_path)
            .args(["snapshot", "-d", snapshot_name, disk_path_str])
            .output()
            .map_err(|e| crate::vibe_core::Error::Storage(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::vibe_core::Error::Storage(stderr.to_string()));
        }

        Ok(())
    }

    pub fn get_disk_info(&self, disk_path: &Path) -> Result<DiskInfo> {
        let disk_path_str = disk_path.to_str().ok_or_else(|| {
            crate::vibe_core::Error::Storage("Disk path contains invalid characters".to_string())
        })?;

        let output = Command::new(&self.qemu_img_path)
            .args(["info", "--output=json", disk_path_str])
            .output()
            .map_err(|e| crate::vibe_core::Error::Storage(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::vibe_core::Error::Storage(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let info: serde_json::Value = serde_json::from_str(&stdout)
            .map_err(|e| crate::vibe_core::Error::Storage(e.to_string()))?;

        Ok(DiskInfo {
            virtual_size: info["virtual-size"].as_u64().unwrap_or(0),
            disk_size: info
                .get("actual-size")
                .and_then(|v| v.as_u64())
                .unwrap_or(0),
            format: info["format"].as_str().unwrap_or("unknown").to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct SnapshotInfo {
    pub id: String,
    pub name: String,
    pub vm_clock: String,
}

#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub virtual_size: u64,
    pub disk_size: u64,
    pub format: String,
}
