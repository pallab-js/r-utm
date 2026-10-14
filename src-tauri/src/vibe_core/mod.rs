//! VibeVisor Core - High-performance virtualization orchestration

pub mod config;
pub mod cores;
pub mod error;
pub mod kernel;
pub mod launcher;
pub mod monitor;
pub mod qmp;
pub mod rate_limiter;
pub mod security;
pub mod storage;

pub use cores::{CoreAllocation, CoreAllocationRequest, CoreAllocator, CoreType, CpuTopology};
pub use error::{Error, Result};
pub use kernel::{KernelBootManager, KernelConfig};
pub use monitor::{MonitoringManager, VmMetrics};
