//! Process monitoring for VibeVisor VMs

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize)]
pub struct VmMetrics {
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub memory_percent: f32,
    pub uptime_seconds: u64,
}

pub struct VmMonitor {
    pid: u32,
    start_time: std::time::Instant,
    /// Total physical RAM in bytes, used to compute memory_percent.
    total_memory_bytes: u64,
}

impl VmMonitor {
    pub fn new(pid: u32) -> Self {
        Self {
            pid,
            start_time: std::time::Instant::now(),
            total_memory_bytes: Self::total_physical_memory().unwrap_or(0),
        }
    }

    /// Query total physical memory (bytes) on macOS via `sysctl hw.memsize`.
    #[cfg(target_os = "macos")]
    fn total_physical_memory() -> Option<u64> {
        use std::process::Command;
        let output = Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()?;
        String::from_utf8_lossy(&output.stdout).trim().parse().ok()
    }

    #[cfg(not(target_os = "macos"))]
    fn total_physical_memory() -> Option<u64> {
        None
    }

    pub fn collect(&mut self) -> Option<VmMetrics> {
        #[cfg(target_os = "macos")]
        {
            self.collect_macos()
        }

        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    #[cfg(target_os = "macos")]
    fn collect_macos(&mut self) -> Option<VmMetrics> {
        // Sample the process's CPU% and resident set size via `ps`. (A future
        // improvement would be to use libproc's proc_pidinfo to avoid spawning
        // a child process on every poll.)
        use std::process::Command;

        let output = Command::new("ps")
            .args(["-o", "pid=,pcpu=,rss=", "-p", &self.pid.to_string()])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.split_whitespace().collect();

        if parts.len() < 3 {
            return None;
        }

        let cpu_percent: f32 = parts.get(1)?.parse().ok()?;
        let rss_kb: u64 = parts.get(2)?.parse().ok()?;
        let memory_mb = rss_kb / 1024;

        // Compute memory as a percentage of total physical RAM.
        let memory_percent = if self.total_memory_bytes > 0 {
            ((rss_kb * 1024) as f64 / self.total_memory_bytes as f64 * 100.0) as f32
        } else {
            0.0
        };

        Some(VmMetrics {
            cpu_percent,
            memory_mb,
            memory_percent,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        })
    }
}

pub struct MonitoringManager {
    monitors: Arc<RwLock<std::collections::HashMap<String, VmMonitor>>>,
}

impl MonitoringManager {
    pub fn new() -> Self {
        Self {
            monitors: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn add_vm(&self, vm_id: String, pid: u32) {
        let mut monitors = self.monitors.write().await;
        monitors.insert(vm_id, VmMonitor::new(pid));
    }

    pub async fn remove_vm(&self, vm_id: &str) {
        let mut monitors = self.monitors.write().await;
        monitors.remove(vm_id);
    }

    pub async fn get_metrics(&self, vm_id: &str) -> Option<VmMetrics> {
        let mut monitors = self.monitors.write().await;
        monitors.get_mut(vm_id)?.collect()
    }

    pub async fn get_all_metrics(&self) -> Vec<(String, Option<VmMetrics>)> {
        let mut monitors = self.monitors.write().await;
        monitors
            .iter_mut()
            .map(|(id, m)| (id.clone(), m.collect()))
            .collect()
    }
}

impl Default for MonitoringManager {
    fn default() -> Self {
        Self::new()
    }
}
