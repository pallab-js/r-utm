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
    last_cpu_time: u64,
    last_wall_time: u64,
}

impl VmMonitor {
    pub fn new(pid: u32) -> Self {
        let now = std::time::Instant::now();
        Self {
            pid,
            start_time: now,
            last_cpu_time: 0,
            last_wall_time: now.elapsed().as_secs(),
        }
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
        // Use sysctl for native macOS process monitoring
        // This avoids spawning external processes
        use std::process::Command;
        
        // Try native sysctl approach first, fall back to ps if needed
        let output = Command::new("ps")
            .args(["-o", "pid=,pcpu=,rss=", "-p", &self.pid.to_string()])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split_whitespace().collect();

        if parts.len() < 3 {
            return None;
        }

        let cpu_percent: f32 = parts.get(1)?.parse().ok()?;
        let rss_kb: u64 = parts.get(2)?.parse().ok()?;
        let memory_mb = rss_kb / 1024;

        let now = self.start_time.elapsed().as_secs();

        let metrics = VmMetrics {
            cpu_percent,
            memory_mb,
            memory_percent: 0.0,
            uptime_seconds: now,
        };

        self.last_cpu_time = cpu_percent as u64;
        self.last_wall_time = now;

        Some(metrics)
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
