//! VibeVisor Core - High-performance virtualization orchestration

pub mod launcher;
pub mod qmp;
pub mod config;
pub mod storage;
pub mod error;
pub mod monitor;
pub mod kernel;
pub mod cores;
pub mod security;
pub mod rate_limiter;

pub use error::{Error, Result};
pub use monitor::{VmMetrics, MonitoringManager};
pub use kernel::{KernelConfig, KernelBootManager};
pub use cores::{CpuTopology, CoreType, CoreAllocator, CoreAllocation, CoreAllocationRequest};
