pub mod vibe_core;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use vibe_core::config::VmConfig;
use vibe_core::qmp::VmState;
use vibe_core::launcher::QemuLauncher;
use vibe_core::qmp::QmpClient;
use vibe_core::storage::StorageManager;
use vibe_core::monitor::MonitoringManager;
use vibe_core::cores::{CpuTopology, CoreAllocator, CoreAllocationRequest, CoreAllocation};
use vibe_core::security::{
    create_secure_temp_dir, 
    inspect_archive, 
    is_path_safe,
};
use vibe_core::rate_limiter::CommandRateLimiter;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Manager,
    Emitter,
};

macro_rules! audit_log {
    ($($arg:tt)*) => {
        tracing::info!(target: "audit", $($arg)*)
    }
}

fn find_qemu_binary(name: &str) -> PathBuf {
    let search_paths = [
        PathBuf::from(&format!("/usr/local/bin/{}", name)),
        PathBuf::from(&format!("/opt/homebrew/bin/{}", name)),
        PathBuf::from(&format!("/opt/local/bin/{}", name)),
        PathBuf::from(format!("{}/sidecars/qemu/{}", env!("CARGO_MANIFEST_DIR"), name)),
        PathBuf::from(format!("./src-tauri/sidecars/qemu/{}", name)),
        PathBuf::from(name),
    ];

    for path in &search_paths {
        if path.exists() {
            tracing::info!("Found {} at {:?}", name, path);
            return path.clone();
        }
    }

    tracing::warn!("{} not found, using system PATH", name);
    PathBuf::from(name)
}

pub struct VmInstance {
    pub config: VmConfig,
    pub state: VmState,
    pub process: Option<tokio::process::Child>,
    pub pid: Option<u32>,
}

pub struct AppState {
    pub vms: RwLock<HashMap<String, Arc<RwLock<VmInstance>>>>,
    pub vm_dir: PathBuf,
    pub qemu_path: PathBuf,
    pub qemu_img_path: PathBuf,
    pub monitor: MonitoringManager,
    pub settings: RwLock<AppSettings>,
    pub rate_limiter: CommandRateLimiter,
}

impl AppState {
    pub fn new(vm_dir: PathBuf, qemu_path: PathBuf, qemu_img_path: PathBuf) -> Self {
        Self {
            vms: RwLock::new(HashMap::new()),
            vm_dir,
            qemu_path,
            qemu_img_path,
            monitor: MonitoringManager::new(),
            settings: RwLock::new(AppSettings::default()),
            rate_limiter: CommandRateLimiter::default(),
        }
    }

    pub async fn load_persisted_vms(&self) {
        let mut vms = self.vms.write().await;
        
        if let Ok(entries) = std::fs::read_dir(&self.vm_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |ext| ext == "toml") {
                    match VmConfig::load(&path) {
                        Ok(config) => {
                            let instance = VmInstance {
                                config,
                                state: VmState::Stopped,
                                process: None,
                                pid: None,
                            };
                            vms.insert(instance.config.id.clone(), Arc::new(RwLock::new(instance)));
                            tracing::info!("Loaded VM config: {:?}", path);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to load VM config {:?}: {}", path, e);
                        }
                    }
                }
            }
        }
    }

    pub fn get_config_path(&self, vm_id: &str) -> PathBuf {
        self.vm_dir.join(vm_id).with_extension("toml")
    }

    pub async fn save_vm_config(&self, vm_id: &str) -> Result<(), String> {
        let vms = self.vms.read().await;
        if let Some(vm) = vms.get(vm_id) {
            let config = vm.read().await.config.clone();
            let path = self.get_config_path(vm_id);
            config.save(&path).map_err(|e| e.to_string())?;
            tracing::info!("Saved VM config: {:?}", path);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppSettings {
    pub default_cpu_cores: u32,
    pub default_memory_mb: u64,
    pub default_disk_gb: u64,
    pub theme: String,
    pub qemu_custom_path: Option<String>,
    pub auto_start_last_vm: bool,
    pub minimize_to_tray: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_cpu_cores: 4,
            default_memory_mb: 4096,
            default_disk_gb: 20,
            theme: "dark".to_string(),
            qemu_custom_path: None,
            auto_start_last_vm: false,
            minimize_to_tray: false, // Default to false so close button actually quits
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("VibeVisor");

        let config_path = config_dir.join("settings.toml");

        if config_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(settings) = toml::from_str(&content) {
                    return settings;
                }
            }
        }

        Self::default()
    }

    pub fn save(&self) -> Result<(), String> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| "Cannot determine config directory".to_string())?
            .join("VibeVisor");

        std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

        let config_path = config_dir.join("settings.toml");
        let content = toml::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(config_path, content).map_err(|e| e.to_string())?;

        tracing::info!("Settings saved");
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VmInfo {
    pub config: VmConfig,
    pub state: String,
    pub pid: Option<u32>,
}

#[tauri::command]
async fn list_vms(state: tauri::State<'_, Arc<AppState>>) -> Result<Vec<VmInfo>, String> {
    let vms = state.vms.read().await;
    let mut result = Vec::new();
    for v in vms.values() {
        let vm = v.read().await;
        result.push(VmInfo {
            config: vm.config.clone(),
            state: format!("{:?}", vm.state),
            pid: vm.pid,
        });
    }
    Ok(result)
}

#[tauri::command]
async fn create_vm(
    name: String,
    cpu_cores: u32,
    memory_mb: u64,
    disk_size_gb: u64,
    iso_path: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<VmConfig, String> {
    // Rate limit VM creation
    if !state.rate_limiter.allow_command("create_vm").await {
        return Err("Rate limit exceeded. Please wait before creating another VM.".to_string());
    }
    
    let mut config = VmConfig::new(&name);
    
    config.hardware.cpu_cores = cpu_cores;
    config.hardware.memory_mb = memory_mb;
    config.storage.disk_size_gb = disk_size_gb;

    let disk_path = state.vm_dir.join(&config.id).with_extension("qcow2");
    config.storage.disk_path = disk_path.clone();

    let storage = StorageManager::new(state.qemu_img_path.clone());
    storage.create_disk(&disk_path, disk_size_gb)
        .map_err(|e: vibe_core::Error| e.to_string())?;

    if let Some(iso) = iso_path {
        if iso.is_empty() {
            return Err("ISO path cannot be empty".to_string());
        }
        let iso_path_buf = PathBuf::from(&iso);
        if !iso_path_buf.exists() {
            return Err("ISO file does not exist".to_string());
        }
        let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
        if !is_path_safe(&iso_path_buf, &home_dir) {
            return Err("ISO path must be within home directory and cannot contain traversal sequences".to_string());
        }
        config.storage.boot_iso = Some(iso_path_buf);
        audit_log!("Security audit: ISO path validated for VM creation: {}", iso);
    }

    let vm = VmInstance {
        config: config.clone(),
        state: VmState::Stopped,
        process: None,
        pid: None,
    };

    let mut vms = state.vms.write().await;
    vms.insert(config.id.clone(), Arc::new(RwLock::new(vm)));
    drop(vms);

    state.save_vm_config(&config.id).await?;

    audit_log!("Security: VM created - id={}, name={}", config.id, name);
    tracing::info!("Created VM: {} ({})", name, config.id);
    Ok(config)
}

#[tauri::command]
async fn start_vm(
    vm_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let vm = {
        let vms = state.vms.read().await;
        let vm = vms.get(&vm_id).ok_or_else(|| "VM not found".to_string())?;
        Arc::clone(vm)
    };

    let launcher = QemuLauncher::new(state.qemu_path.clone())
        .map_err(|e| format!("Failed to initialize VM launcher: {}", e))?;
    let config = vm.read().await.config.clone();

    let child = launcher.launch(&config).await
        .map_err(|e: vibe_core::Error| e.to_string())?;

    let pid = child.id().ok_or("Failed to get PID")?;

    let mut vm_guard = vm.write().await;
    vm_guard.process = Some(child);
    vm_guard.state = VmState::Running;
    vm_guard.pid = Some(pid);
    
    drop(vm_guard);
    state.monitor.add_vm(vm_id.clone(), pid).await;
    
    audit_log!("Security: VM started - id={}, pid={}", vm_id, pid);
    tracing::info!("Started VM: {} (PID: {})", vm_id, pid);
    Ok(())
}

#[tauri::command]
async fn stop_vm(
    vm_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let vms = state.vms.read().await;
    let vm = vms.get(&vm_id).ok_or_else(|| "VM not found".to_string())?;
    
    let qmp = QmpClient::new(&vm_id);
    let _ = qmp.quit().await;

    let mut vm_guard = vm.write().await;
    vm_guard.state = VmState::Stopped;
    vm_guard.pid = None;
    
    drop(vm_guard);
    state.monitor.remove_vm(&vm_id).await;
    
    tracing::info!("Stopped VM: {}", vm_id);
    Ok(())
}

#[tauri::command]
async fn get_vm_state(
    vm_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let vms = state.vms.read().await;
    let vm = vms.get(&vm_id).ok_or_else(|| "VM not found".to_string())?;
    let state_str = format!("{:?}", vm.read().await.state);
    Ok(state_str)
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct VmMetricsResponse {
    pub vm_id: String,
    pub cpu_percent: f32,
    pub memory_mb: u64,
    pub uptime_seconds: u64,
}

#[tauri::command]
async fn get_vm_metrics(
    vm_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Option<VmMetricsResponse>, String> {
    let metrics = state.monitor.get_metrics(&vm_id).await;
    Ok(metrics.map(|m| VmMetricsResponse {
        vm_id: vm_id.clone(),
        cpu_percent: m.cpu_percent,
        memory_mb: m.memory_mb,
        uptime_seconds: m.uptime_seconds,
    }))
}

#[tauri::command]
async fn get_all_metrics(
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<VmMetricsResponse>, String> {
    let metrics_list = state.monitor.get_all_metrics().await;
    let result: Vec<VmMetricsResponse> = metrics_list
        .into_iter()
        .filter_map(|(id, m)| {
            m.map(|metrics| VmMetricsResponse {
                vm_id: id,
                cpu_percent: metrics.cpu_percent,
                memory_mb: metrics.memory_mb,
                uptime_seconds: metrics.uptime_seconds,
            })
        })
        .collect();
    Ok(result)
}

#[tauri::command]
async fn delete_vm(
    vm_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let vms = state.vms.read().await;
    let vm = vms.get(&vm_id).ok_or_else(|| "VM not found".to_string())?;
    let vm_data = vm.read().await;
    
    if vm_data.state == VmState::Running {
        return Err("Cannot delete a running VM. Please stop it first.".to_string());
    }
    
    let disk_path = vm_data.config.storage.disk_path.clone();
    drop(vm_data);
    drop(vms);
    
    let mut vms = state.vms.write().await;
    let vm = vms.remove(&vm_id).ok_or_else(|| "VM not found".to_string())?;
    drop(vm);
    
    let _ = std::fs::remove_file(&disk_path);
    let _ = std::fs::remove_file(state.get_config_path(&vm_id));
    
    state.monitor.remove_vm(&vm_id).await;
    
    audit_log!("Security audit: VM deleted: {}", vm_id);
    tracing::info!("Deleted VM: {}", vm_id);
    Ok(())
}

const MIN_CPU_CORES: u32 = 1;
const MAX_CPU_CORES: u32 = 32;
const MIN_MEMORY_MB: u64 = 256;
const MAX_MEMORY_MB: u64 = 131072;
const MIN_DISK_GB: u64 = 1;
const MAX_DISK_GB: u64 = 1024;

#[tauri::command]
async fn update_vm(
    vm_id: String,
    name: String,
    cpu_cores: u32,
    memory_mb: u64,
    disk_size_gb: u64,
    network_enabled: bool,
    network_mode: String,
    display_width: u32,
    display_height: u32,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    if name.is_empty() || name.len() > 255 {
        return Err("Invalid VM name".to_string());
    }
    if name.chars().any(|c| c.is_control() || c == '/' || c == '\\' || c == ':') {
        return Err("VM name contains invalid characters".to_string());
    }
    if cpu_cores < MIN_CPU_CORES || cpu_cores > MAX_CPU_CORES {
        return Err(format!("CPU cores must be between {} and {}", MIN_CPU_CORES, MAX_CPU_CORES));
    }
    if memory_mb < MIN_MEMORY_MB || memory_mb > MAX_MEMORY_MB {
        return Err(format!("Memory must be between {} MB and {} MB", MIN_MEMORY_MB, MAX_MEMORY_MB));
    }
    if disk_size_gb < MIN_DISK_GB || disk_size_gb > MAX_DISK_GB {
        return Err(format!("Disk size must be between {} GB and {} GB", MIN_DISK_GB, MAX_DISK_GB));
    }
    if display_width < 640 || display_width > 7680 || display_height < 480 || display_height > 4320 {
        return Err("Invalid display resolution".to_string());
    }
    
    let vms = state.vms.read().await;
    let vm = vms.get(&vm_id).ok_or_else(|| "VM not found".to_string())?;
    
    let mut vm_guard = vm.write().await;
    vm_guard.config.name = name;
    vm_guard.config.hardware.cpu_cores = cpu_cores;
    vm_guard.config.hardware.memory_mb = memory_mb;
    vm_guard.config.storage.disk_size_gb = disk_size_gb;
    vm_guard.config.network.enabled = network_enabled;
    let normalized_mode = network_mode.to_lowercase();
    let network_mode_enum = match normalized_mode.as_str() {
        "bridged" => vibe_core::config::NetworkMode::Bridged,
        "shared" => vibe_core::config::NetworkMode::Shared,
        "nat" => vibe_core::config::NetworkMode::Nat,
        _ => return Err("Invalid network_mode. Must be 'nat', 'bridged', or 'shared'".to_string()),
    };
    vm_guard.config.network.mode = network_mode_enum;
    vm_guard.config.display.resolution.width = display_width;
    vm_guard.config.display.resolution.height = display_height;
    
    drop(vm_guard);
    drop(vms);
    
    state.save_vm_config(&vm_id).await?;
    
    tracing::info!("Updated VM: {}", vm_id);
    audit_log!("Security: VM updated - id={}", vm_id);
    Ok(())
}

#[tauri::command]
async fn export_vm(
    vm_id: String,
    export_path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    // Rate limit VM export
    if !state.rate_limiter.allow_command("export_vm").await {
        return Err("Rate limit exceeded. Please wait before exporting another VM.".to_string());
    }
    
    let vms = state.vms.read().await;
    let vm = vms.get(&vm_id).ok_or_else(|| "VM not found".to_string())?;
    let vm_data = vm.read().await;

    let export_dir = PathBuf::from(&export_path);

    let home_dir = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    if !export_dir.starts_with(&home_dir) {
        return Err("Export path must be within home directory".to_string());
    }

    std::fs::create_dir_all(&export_dir).map_err(|e| e.to_string())?;

    let disk_name = format!("{}.qcow2", vm_id);
    let config_name = format!("{}.toml", vm_id);
    let archive_name = format!("{}.tar.gz", vm_id);

    // Create secure temporary directory
    let temp_dir = create_secure_temp_dir("vibevisor-export-")
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    let disk_src = &vm_data.config.storage.disk_path;
    let disk_dst = temp_dir.path().join(&disk_name);
    if disk_src.exists() {
        std::fs::copy(disk_src, &disk_dst).map_err(|e| e.to_string())?;
    }

    let config_src = state.get_config_path(&vm_id);
    let config_dst = temp_dir.path().join(&config_name);
    if config_src.exists() {
        std::fs::copy(&config_src, &config_dst).map_err(|e| e.to_string())?;
    }

    let archive_path = export_dir.join(&archive_name);
    let archive_path_str = archive_path.to_str()
        .ok_or_else(|| "Invalid export path".to_string())?;
    let temp_dir_str = temp_dir.path().to_str()
        .ok_or_else(|| "Invalid temp directory path".to_string())?;

    let output = std::process::Command::new("tar")
        .args(["-czvf", archive_path_str, "-C", temp_dir_str, "."])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("Failed to create archive: Operation error".to_string());
    }

    // TempDir automatically cleans up when dropped
    
    tracing::info!("Exported VM {} to {}", vm_id, archive_path.display());
    audit_log!("Security: VM exported - id={}, path={}", vm_id, archive_path.display());
    Ok(archive_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn import_vm(
    archive_path: String,
    target_dir: Option<String>,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<VmConfig, String> {
    // Rate limit VM import
    if !state.rate_limiter.allow_command("import_vm").await {
        return Err("Rate limit exceeded. Please wait before importing another VM.".to_string());
    }
    
    let archive = PathBuf::from(&archive_path);
    if !archive.exists() {
        return Err("Archive file not found".to_string());
    }

    // Inspect archive before extraction for security
    let archive_info = inspect_archive(&archive)
        .map_err(|e| format!("Failed to inspect archive: {}", e))?;
    
    // Maximum archive size: 50 GB
    const MAX_ARCHIVE_SIZE: u64 = 50 * 1024 * 1024 * 1024;
    
    if !archive_info.is_safe(MAX_ARCHIVE_SIZE) {
        if !archive_info.suspicious_entries.is_empty() {
            return Err("Archive contains suspicious or unsafe paths".to_string());
        }
        return Err("Archive exceeds maximum allowed size (50 GB)".to_string());
    }
    
    tracing::info!(
        "Archive inspection: {} files, {} bytes",
        archive_info.file_count,
        archive_info.total_size
    );

    let import_dir = if let Some(dir) = target_dir {
        let dir_path = PathBuf::from(&dir);
        let vm_dir = state.vm_dir.clone();
        if !dir_path.starts_with(&vm_dir) {
            return Err("Invalid target directory".to_string());
        }
        dir_path
    } else {
        state.vm_dir.clone()
    };

    std::fs::create_dir_all(&import_dir).map_err(|e| e.to_string())?;

    // Create secure temporary directory for extraction
    let temp_dir = create_secure_temp_dir("vibevisor-import-")
        .map_err(|e| format!("Failed to create temp directory: {}", e))?;

    let archive_path_str = archive.to_str()
        .ok_or_else(|| "Invalid archive path".to_string())?;
    let temp_dir_str = temp_dir.path().to_str()
        .ok_or_else(|| "Invalid temp directory path".to_string())?;

    let output = std::process::Command::new("tar")
        .args(["-xzvf", archive_path_str, "-C", temp_dir_str])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("Failed to extract archive: Operation error".to_string());
    }

    // Verify extracted paths are safe
    let entries = std::fs::read_dir(temp_dir.path()).map_err(|e| e.to_string())?;
    let mut config_path: Option<PathBuf> = None;
    let mut disk_path: Option<PathBuf> = None;

    for entry in entries.flatten() {
        let path = entry.path();
        
        // Double-check path safety
        if !is_path_safe(&path, temp_dir.path()) {
            tracing::warn!("Skipping unsafe path: {:?}", path);
            continue;
        }
        
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.ends_with(".toml") {
            config_path = Some(path);
        } else if name_str.ends_with(".qcow2") {
            disk_path = Some(path);
        }
    }

    let config = match config_path {
        Some(path) => VmConfig::load(&path).map_err(|e| e.to_string())?,
        None => {
            return Err("No configuration file found in archive".to_string());
        }
    };

    let new_id = uuid::Uuid::new_v4().to_string();
    let mut new_config = config.clone();
    new_config.id = new_id.clone();

    let new_disk_path = import_dir.join(format!("{}.qcow2", new_id));
    if let Some(disk) = disk_path {
        if disk.exists() && is_path_safe(&disk, temp_dir.path()) {
            std::fs::copy(&disk, &new_disk_path).map_err(|e| e.to_string())?;
        }
    }
    new_config.storage.disk_path = new_disk_path.clone();

    let new_config_path = import_dir.join(format!("{}.toml", new_id));
    new_config.save(&new_config_path).map_err(|e| e.to_string())?;

    // TempDir automatically cleans up when dropped

    let vm = VmInstance {
        config: new_config.clone(),
        state: VmState::Stopped,
        process: None,
        pid: None,
    };

    let mut vms = state.vms.write().await;
    vms.insert(new_id.clone(), Arc::new(RwLock::new(vm)));

    tracing::info!("Imported VM as {}", new_id);
    audit_log!("Security: VM imported - old_id={}, new_id={}", new_id, new_id);
    Ok(new_config)
}

#[tauri::command]
async fn pause_vm(
    vm_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let vms = state.vms.read().await;
    let vm = vms.get(&vm_id).ok_or_else(|| "VM not found".to_string())?;
    
    let qmp = QmpClient::new(&vm_id);
    qmp.stop().await.map_err(|e| e.to_string())?;

    let mut vm_guard = vm.write().await;
    vm_guard.state = VmState::Paused;
    
    tracing::info!("Paused VM: {}", vm_id);
    Ok(())
}

#[tauri::command]
async fn resume_vm(
    vm_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let vms = state.vms.read().await;
    let vm = vms.get(&vm_id).ok_or_else(|| "VM not found".to_string())?;
    
    let qmp = QmpClient::new(&vm_id);
    qmp.cont().await.map_err(|e| e.to_string())?;

    let mut vm_guard = vm.write().await;
    vm_guard.state = VmState::Running;
    
    tracing::info!("Resumed VM: {}", vm_id);
    Ok(())
}

#[tauri::command]
async fn save_vm(
    vm_id: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.save_vm_config(&vm_id).await
}

#[tauri::command]
fn get_cpu_topology() -> Option<CpuTopology> {
    CoreAllocator::detect()
}

#[tauri::command]
fn suggest_core_allocation(
    total_cores: u32,
    prefer_performance: bool,
    memory_mb: u64,
) -> CoreAllocation {
    let request = CoreAllocationRequest {
        total_cores,
        prefer_performance,
        memory_mb,
    };
    
    CoreAllocator::new()
        .with_topology(CoreAllocator::detect().unwrap_or_default())
        .suggest_vm_cores(request)
}

#[tauri::command]
fn get_spice_port(vm_id: String) -> u16 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    vm_id.hash(&mut hasher);
    let hash = hasher.finish();
    
    let base_port: u16 = 5900;
    let offset = (hash % 100) as u16;
    base_port + offset
}

#[tauri::command]
async fn get_settings(state: tauri::State<'_, Arc<AppState>>) -> Result<AppSettings, String> {
    let settings = state.settings.read().await;
    Ok(settings.clone())
}

#[tauri::command]
async fn save_settings(
    settings: AppSettings,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let mut current = state.settings.write().await;
    *current = settings.clone();
    drop(current);
    settings.save()
}

#[tauri::command]
fn get_default_settings() -> AppSettings {
    AppSettings::load()
}

/// Gracefully shutdown all running VMs before exit
async fn graceful_shutdown(state: &Arc<AppState>) {
    tracing::info!("Starting graceful shutdown...");
    
    // Collect running VMs
    let vms = state.vms.read().await;
    let vm_ids: Vec<String> = vms.keys().cloned().collect();
    drop(vms);
    
    let mut running_vms = Vec::new();
    
    for vm_id in &vm_ids {
        let vms = state.vms.read().await;
        if let Some(vm) = vms.get(vm_id) {
            let vm_guard = vm.read().await;
            if vm_guard.state == VmState::Running {
                running_vms.push((vm_id.clone(), Arc::clone(vm)));
            }
        }
    }
    
    if running_vms.is_empty() {
        tracing::info!("No running VMs to shut down");
        return;
    }
    
    tracing::info!("Stopping {} running VM(s)...", running_vms.len());
    
    for (vm_id, vm) in &running_vms {
        tracing::info!("Stopping VM: {}", vm_id);
        
        // Try graceful shutdown via QMP first
        let qmp = QmpClient::new(vm_id);
        if qmp.quit().await.is_ok() {
            tracing::info!("Sent graceful shutdown to VM: {}", vm_id);
        }
        
        // Give VM time to shut down gracefully (500ms)
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        
        // Force kill if still running
        let mut vm_guard = vm.write().await;
        if let Some(ref mut process) = vm_guard.process {
            let _ = process.kill().await;
            tracing::info!("Force killed VM: {}", vm_id);
        }
        vm_guard.state = VmState::Stopped;
        vm_guard.process = None;
        vm_guard.pid = None;
        drop(vm_guard);
        
        state.monitor.remove_vm(vm_id).await;
    }
    
    tracing::info!("All VMs stopped. Shutdown complete.");
}

#[tauri::command]
async fn confirm_and_quit(state: tauri::State<'_, Arc<AppState>>) -> Result<(), String> {
    tracing::info!("User confirmed quit, performing graceful shutdown...");
    graceful_shutdown(&state).await;
    std::process::exit(0);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let settings = AppSettings::load();
    
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .init();

    let vm_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("VibeVisor")
        .join("vms");

    let qemu_path = find_qemu_binary("qemu-system-aarch64");
    let qemu_img_path = find_qemu_binary("qemu-img");

    std::fs::create_dir_all(&vm_dir).expect("Failed to create VM directory. Ensure you have write permissions to the data directory.");

    let app_state = Arc::new(AppState::new(
        vm_dir.clone(),
        qemu_path,
        qemu_img_path,
    ));

    let app_state_clone = Arc::clone(&app_state);
    let minimize_to_tray = settings.minimize_to_tray;
    
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state)
        .setup(move |app| {
            let state = Arc::clone(&app_state_clone);
            tauri::async_runtime::spawn(async move {
                state.load_persisted_vms().await;
            });

            let show_item = MenuItem::with_id(app, "show", "Show VibeVisor", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            
            let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

            let _tray = TrayIconBuilder::new()
                .menu(&menu)
                .tooltip("VibeVisor - Apple Silicon VM Manager")
                .on_menu_event(move |app, event| {
                    match event.id.as_ref() {
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::DoubleClick { .. } = event {
                        if let Some(window) = tray.app_handle().get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            if minimize_to_tray {
                if let Some(window) = app.get_webview_window("main") {
                    let app_handle = app.handle().clone();
                    window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            api.prevent_close();
                            if let Some(w) = app_handle.get_webview_window("main") {
                                let _ = w.hide();
                            }
                        }
                    });
                }
            } else {
                // When minimize_to_tray is disabled, close button triggers graceful shutdown with confirmation
                if let Some(window) = app.get_webview_window("main") {
                    let app_handle = app.handle().clone();
                    let state = Arc::clone(&app_state_clone);
                    
                    window.on_window_event(move |event| {
                        if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                            // Prevent immediate close so we can check for running VMs
                            api.prevent_close();
                            
                            let app_handle = app_handle.clone();
                            let state = Arc::clone(&state);
                            
                            // Check for running VMs and show confirmation if needed
                            tauri::async_runtime::spawn(async move {
                                let vms = state.vms.read().await;
                                let running_count = {
                                    let mut count = 0;
                                    for vm in vms.values() {
                                        let guard = vm.read().await;
                                        if guard.state == VmState::Running {
                                            count += 1;
                                        }
                                    }
                                    count
                                };
                                drop(vms);
                                
                                if running_count > 0 {
                                    // Send message to frontend to show confirmation dialog
                                    let _ = app_handle.emit("close-confirmation-needed", &running_count.to_string());
                                    // Window will be closed by frontend after user confirms
                                } else {
                                    // No running VMs, just exit
                                    graceful_shutdown(&state).await;
                                    std::process::exit(0);
                                }
                            });
                        }
                    });
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_vms,
            create_vm,
            start_vm,
            stop_vm,
            get_vm_state,
            get_vm_metrics,
            get_all_metrics,
            delete_vm,
            update_vm,
            export_vm,
            import_vm,
            pause_vm,
            resume_vm,
            save_vm,
            get_cpu_topology,
            suggest_core_allocation,
            get_spice_port,
            get_settings,
            save_settings,
            get_default_settings,
            confirm_and_quit,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
