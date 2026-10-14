//! Apple Silicon P-core/E-core detection and allocation for VibeVisor

use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuTopology {
    pub total_cores: u32,
    pub performance_cores: u32,
    pub efficiency_cores: u32,
    pub clusters: Vec<CpuCluster>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCluster {
    pub cluster_id: u32,
    pub core_type: CoreType,
    pub core_indices: Vec<u32>,
    pub max_frequency_mhz: u32,
    pub min_frequency_mhz: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CoreType {
    Performance,
    Efficiency,
    Unknown,
}

impl CoreType {
    pub fn as_str(&self) -> &'static str {
        match self {
            CoreType::Performance => "P-core",
            CoreType::Efficiency => "E-core",
            CoreType::Unknown => "Unknown",
        }
    }
}

pub struct CoreAllocator {
    topology: Option<CpuTopology>,
}

impl CoreAllocator {
    pub fn new() -> Self {
        Self { topology: None }
    }

    pub fn detect() -> Option<CpuTopology> {
        #[cfg(target_os = "macos")]
        {
            Self::detect_macos()
        }
        #[cfg(not(target_os = "macos"))]
        {
            None
        }
    }

    #[cfg(target_os = "macos")]
    fn detect_macos() -> Option<CpuTopology> {
        let sysctl_output = Command::new("sysctl")
            .args([
                "-n",
                "hw.perflevel0.physicalcpu",
                "hw.perflevel1.physicalcpu",
                "hw.physicalcpu",
            ])
            .output()
            .ok()?;

        let output = String::from_utf8_lossy(&sysctl_output.stdout);
        let parts: Vec<&str> = output.split_whitespace().collect();

        if parts.len() < 2 {
            return None;
        }

        let p_cores: u32 = parts.first()?.parse().ok().unwrap_or(0);
        let e_cores: u32 = parts.get(1)?.parse().ok().unwrap_or(0);
        let total: u32 = parts
            .get(2)
            .and_then(|s| s.parse().ok())
            .unwrap_or(p_cores + e_cores);

        let mut clusters = Vec::new();

        if p_cores > 0 {
            clusters.push(CpuCluster {
                cluster_id: 0,
                core_type: CoreType::Performance,
                core_indices: (0..p_cores).collect(),
                max_frequency_mhz: 0,
                min_frequency_mhz: 0,
            });
        }

        if e_cores > 0 {
            clusters.push(CpuCluster {
                cluster_id: 1,
                core_type: CoreType::Efficiency,
                core_indices: (p_cores..p_cores + e_cores).collect(),
                max_frequency_mhz: 0,
                min_frequency_mhz: 0,
            });
        }

        Some(CpuTopology {
            total_cores: total,
            performance_cores: p_cores,
            efficiency_cores: e_cores,
            clusters,
        })
    }

    pub fn with_topology(mut self, topology: CpuTopology) -> Self {
        self.topology = Some(topology);
        self
    }

    pub fn suggest_vm_cores(&self, request: CoreAllocationRequest) -> CoreAllocation {
        let topo = match &self.topology {
            Some(t) => t,
            None => return CoreAllocation::default(),
        };

        let requested = request.total_cores.min(topo.total_cores);
        let (p_cores, e_cores) = if request.prefer_performance {
            let p = requested.min(topo.performance_cores);
            let e = requested.saturating_sub(p).min(topo.efficiency_cores);
            (p, e)
        } else {
            let e = requested.min(topo.efficiency_cores);
            let p = requested.saturating_sub(e).min(topo.performance_cores);
            (p, e)
        };

        CoreAllocation {
            performance_cores: p_cores,
            efficiency_cores: e_cores,
            total: p_cores + e_cores,
            affinity_hint: if request.prefer_performance {
                "performance".to_string()
            } else {
                "efficiency".to_string()
            },
        }
    }

    pub fn to_qemu_args(&self, allocation: &CoreAllocation) -> Vec<String> {
        let mut args = Vec::new();
        args.extend([
            "-smp".to_string(),
            format!(
                "{},cores={}",
                allocation.total,
                if allocation.performance_cores > 0 {
                    allocation.performance_cores
                } else {
                    allocation.efficiency_cores
                }
            ),
        ]);

        #[cfg(target_os = "macos")]
        {
            args.extend(["-cpu".to_string(), "host".to_string()]);
        }

        args
    }
}

impl Default for CoreAllocator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreAllocationRequest {
    pub total_cores: u32,
    pub prefer_performance: bool,
    pub memory_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CoreAllocation {
    pub performance_cores: u32,
    pub efficiency_cores: u32,
    pub total: u32,
    pub affinity_hint: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_topology(p_cores: u32, e_cores: u32) -> CpuTopology {
        let mut clusters = Vec::new();
        if p_cores > 0 {
            clusters.push(CpuCluster {
                cluster_id: 0,
                core_type: CoreType::Performance,
                core_indices: (0..p_cores).collect(),
                max_frequency_mhz: 3200,
                min_frequency_mhz: 600,
            });
        }
        if e_cores > 0 {
            clusters.push(CpuCluster {
                cluster_id: 1,
                core_type: CoreType::Efficiency,
                core_indices: (p_cores..p_cores + e_cores).collect(),
                max_frequency_mhz: 2064,
                min_frequency_mhz: 400,
            });
        }
        CpuTopology {
            total_cores: p_cores + e_cores,
            performance_cores: p_cores,
            efficiency_cores: e_cores,
            clusters,
        }
    }

    #[test]
    fn test_core_type_as_str() {
        assert_eq!(CoreType::Performance.as_str(), "P-core");
        assert_eq!(CoreType::Efficiency.as_str(), "E-core");
        assert_eq!(CoreType::Unknown.as_str(), "Unknown");
    }

    #[test]
    fn test_core_type_serialization() {
        assert_eq!(
            serde_json::to_string(&CoreType::Performance).unwrap(),
            "\"performance\""
        );
        assert_eq!(
            serde_json::to_string(&CoreType::Efficiency).unwrap(),
            "\"efficiency\""
        );
    }

    #[test]
    fn test_core_allocator_new() {
        let allocator = CoreAllocator::new();
        assert!(allocator.topology.is_none());
    }

    #[test]
    fn test_core_allocator_with_topology() {
        let topo = mock_topology(8, 4);
        let allocator = CoreAllocator::new().with_topology(topo.clone());
        assert!(allocator.topology.is_some());
        assert_eq!(allocator.topology.unwrap().total_cores, 12);
    }

    #[test]
    fn test_suggest_vm_cores_performance_preferred() {
        let topo = mock_topology(8, 4);
        let allocator = CoreAllocator::new().with_topology(topo);
        let request = CoreAllocationRequest {
            total_cores: 4,
            prefer_performance: true,
            memory_mb: 8192,
        };
        let allocation = allocator.suggest_vm_cores(request);
        assert_eq!(allocation.performance_cores, 4);
        assert_eq!(allocation.efficiency_cores, 0);
        assert_eq!(allocation.total, 4);
        assert_eq!(allocation.affinity_hint, "performance");
    }

    #[test]
    fn test_suggest_vm_cores_efficiency_preferred() {
        let topo = mock_topology(8, 4);
        let allocator = CoreAllocator::new().with_topology(topo);
        let request = CoreAllocationRequest {
            total_cores: 4,
            prefer_performance: false,
            memory_mb: 8192,
        };
        let allocation = allocator.suggest_vm_cores(request);
        assert_eq!(allocation.performance_cores, 0);
        assert_eq!(allocation.efficiency_cores, 4);
        assert_eq!(allocation.total, 4);
        assert_eq!(allocation.affinity_hint, "efficiency");
    }

    #[test]
    fn test_suggest_vm_cores_no_topology() {
        let allocator = CoreAllocator::new();
        let request = CoreAllocationRequest {
            total_cores: 4,
            prefer_performance: true,
            memory_mb: 8192,
        };
        let allocation = allocator.suggest_vm_cores(request);
        assert_eq!(allocation.total, 0);
        assert_eq!(allocation.affinity_hint, "");
    }

    #[test]
    fn test_suggest_vm_cores_exceeds_available() {
        let topo = mock_topology(4, 2);
        let allocator = CoreAllocator::new().with_topology(topo);
        let request = CoreAllocationRequest {
            total_cores: 8,
            prefer_performance: true,
            memory_mb: 16384,
        };
        let allocation = allocator.suggest_vm_cores(request);
        assert_eq!(allocation.performance_cores, 4);
        assert_eq!(allocation.efficiency_cores, 2);
        assert_eq!(allocation.total, 6);
    }

    #[test]
    fn test_to_qemu_args() {
        let allocation = CoreAllocation {
            performance_cores: 4,
            efficiency_cores: 2,
            total: 6,
            affinity_hint: "performance".to_string(),
        };
        let args = CoreAllocator::new().to_qemu_args(&allocation);
        assert!(args.contains(&"-smp".to_string()));
        assert!(args.contains(&"6,cores=4".to_string()));
    }

    #[test]
    fn test_cpu_topology_default() {
        let topo = CpuTopology::default();
        assert_eq!(topo.total_cores, 0);
        assert_eq!(topo.performance_cores, 0);
        assert_eq!(topo.efficiency_cores, 0);
        assert!(topo.clusters.is_empty());
    }

    #[test]
    fn test_core_allocation_default() {
        let allocation = CoreAllocation::default();
        assert_eq!(allocation.performance_cores, 0);
        assert_eq!(allocation.efficiency_cores, 0);
        assert_eq!(allocation.total, 0);
        assert!(allocation.affinity_hint.is_empty());
    }

    #[test]
    fn test_cpu_cluster_serialization() {
        let cluster = CpuCluster {
            cluster_id: 0,
            core_type: CoreType::Performance,
            core_indices: vec![0, 1, 2, 3],
            max_frequency_mhz: 3200,
            min_frequency_mhz: 600,
        };
        let json = serde_json::to_string(&cluster).unwrap();
        assert!(json.contains("\"performance\""));
        assert!(json.contains("\"core_indices\":[0,1,2,3]"));
    }
}
