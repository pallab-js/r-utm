//! VM configuration management with TOML support

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub name: String,
    pub id: String,
    pub architecture: Architecture,
    pub hardware: HardwareConfig,
    pub storage: StorageConfig,
    pub network: NetworkConfig,
    pub display: DisplayConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Architecture {
    Aarch64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfig {
    pub cpu_cores: u32,
    pub memory_mb: u64,
    pub cpu_model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub disk_path: PathBuf,
    pub disk_size_gb: u64,
    pub disk_type: DiskType,
    pub snapshot_enabled: bool,
    pub boot_iso: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DiskType {
    Qcow2,
    Raw,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub enabled: bool,
    pub mode: NetworkMode,
    pub mac_address: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    Nat,
    Bridged,
    Shared,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayConfig {
    pub spice_enabled: bool,
    pub vnc_enabled: bool,
    pub resolution: Resolution,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            name: String::from("New VM"),
            id: uuid::Uuid::new_v4().to_string(),
            architecture: Architecture::Aarch64,
            hardware: HardwareConfig {
                cpu_cores: 4,
                memory_mb: 4096,
                cpu_model: Some(String::from("host")),
            },
            storage: StorageConfig {
                disk_path: PathBuf::new(),
                disk_size_gb: 20,
                disk_type: DiskType::Qcow2,
                snapshot_enabled: true,
                boot_iso: None,
            },
            network: NetworkConfig {
                enabled: true,
                mode: NetworkMode::Nat,
                mac_address: None,
            },
            display: DisplayConfig {
                spice_enabled: true,
                vnc_enabled: false,
                resolution: Resolution {
                    width: 1920,
                    height: 1080,
                },
            },
        }
    }
}

impl VmConfig {
    pub fn new(name: &str) -> Self {
        let mut config = Self::default();
        config.name = name.to_string();
        config
    }

    pub fn load(path: &PathBuf) -> crate::vibe_core::Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::vibe_core::Error::Config(e.to_string()))?;
        toml::from_str(&content).map_err(|e| crate::vibe_core::Error::Config(e.to_string()))
    }

    pub fn save(&self, path: &PathBuf) -> crate::vibe_core::Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| crate::vibe_core::Error::Config(e.to_string()))?;
        std::fs::write(path, content).map_err(|e| crate::vibe_core::Error::Config(e.to_string()))
    }

    pub fn update_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn update_hardware(&mut self, cpu_cores: u32, memory_mb: u64) {
        self.hardware.cpu_cores = cpu_cores;
        self.hardware.memory_mb = memory_mb;
    }

    pub fn update_network(&mut self, enabled: bool, mode: NetworkMode) {
        self.network.enabled = enabled;
        self.network.mode = mode;
    }

    pub fn update_display(&mut self, width: u32, height: u32) {
        self.display.resolution.width = width;
        self.display.resolution.height = height;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn temp_toml_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().expect("Failed to create temp file");
        file.write_all(content.as_bytes()).expect("Failed to write to temp file");
        file
    }

    #[test]
    fn test_vm_config_new() {
        let config = VmConfig::new("Test VM");
        assert_eq!(config.name, "Test VM");
        assert_eq!(config.architecture, Architecture::Aarch64);
        assert_eq!(config.hardware.cpu_cores, 4);
        assert_eq!(config.hardware.memory_mb, 4096);
        assert!(!config.id.is_empty());
    }

    #[test]
    fn test_vm_config_default() {
        let config = VmConfig::default();
        assert_eq!(config.name, "New VM");
        assert!(config.id.len() == 36);
    }

    #[test]
    fn test_vm_config_save_load() {
        let mut config = VmConfig::new("Save Test");
        config.hardware.cpu_cores = 8;
        config.hardware.memory_mb = 8192;

        let temp_file = temp_toml_file("");
        let path = temp_file.path().to_path_buf();

        config.save(&path).expect("Failed to save config");
        let loaded = VmConfig::load(&path).expect("Failed to load config");

        assert_eq!(loaded.name, "Save Test");
        assert_eq!(loaded.hardware.cpu_cores, 8);
        assert_eq!(loaded.hardware.memory_mb, 8192);
    }

    #[test]
    fn test_vm_config_update_name() {
        let mut config = VmConfig::new("Original");
        config.update_name("Updated".to_string());
        assert_eq!(config.name, "Updated");
    }

    #[test]
    fn test_vm_config_update_hardware() {
        let mut config = VmConfig::default();
        config.update_hardware(16, 16384);
        assert_eq!(config.hardware.cpu_cores, 16);
        assert_eq!(config.hardware.memory_mb, 16384);
    }

    #[test]
    fn test_vm_config_update_network() {
        let mut config = VmConfig::default();
        config.update_network(false, NetworkMode::Bridged);
        assert!(!config.network.enabled);
        assert_eq!(config.network.mode, NetworkMode::Bridged);
    }

    #[test]
    fn test_vm_config_update_display() {
        let mut config = VmConfig::default();
        config.update_display(2560, 1440);
        assert_eq!(config.display.resolution.width, 2560);
        assert_eq!(config.display.resolution.height, 1440);
    }

    #[test]
    fn test_architecture_serialization() {
        let arch = Architecture::Aarch64;
        let serialized = serde_json::to_string(&arch).unwrap();
        assert_eq!(serialized, "\"aarch64\"");
    }

    #[test]
    fn test_disk_type_serialization() {
        assert_eq!(
            serde_json::to_string(&DiskType::Qcow2).unwrap(),
            "\"qcow2\""
        );
        assert_eq!(serde_json::to_string(&DiskType::Raw).unwrap(), "\"raw\"");
    }

    #[test]
    fn test_network_mode_serialization() {
        assert_eq!(serde_json::to_string(&NetworkMode::Nat).unwrap(), "\"nat\"");
        assert_eq!(
            serde_json::to_string(&NetworkMode::Bridged).unwrap(),
            "\"bridged\""
        );
        assert_eq!(
            serde_json::to_string(&NetworkMode::Shared).unwrap(),
            "\"shared\""
        );
    }

    #[test]
    fn test_toml_roundtrip() {
        let config = VmConfig::new("TOML Test");
        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: VmConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config.name, deserialized.name);
        assert_eq!(config.architecture, deserialized.architecture);
    }

    #[test]
    fn test_invalid_toml_load() {
        let temp_file = temp_toml_file("invalid toml {{{{");
        let result = VmConfig::load(&temp_file.path().to_path_buf());
        assert!(result.is_err());
    }
}
