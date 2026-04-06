//! Direct kernel boot support for VibeVisor
//! Enables sub-2-second boot times by bypassing UEFI/BIOS

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelConfig {
    pub kernel_path: PathBuf,
    pub initrd_path: Option<PathBuf>,
    pub kernel_cmdline: Vec<String>,
    pub dtb_path: Option<PathBuf>,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            kernel_path: PathBuf::new(),
            initrd_path: None,
            kernel_cmdline: vec![
                "console=hvc0".to_string(),
                "root=/dev/vda".to_string(),
                "rw".to_string(),
                "quiet".to_string(),
            ],
            dtb_path: None,
        }
    }
}

impl KernelConfig {
    pub fn new(kernel_path: PathBuf) -> Self {
        Self {
            kernel_path,
            ..Default::default()
        }
    }

    pub fn with_initrd(mut self, initrd_path: PathBuf) -> Self {
        self.initrd_path = Some(initrd_path);
        self
    }

    pub fn with_cmdline(mut self, cmdline: Vec<String>) -> Self {
        self.kernel_cmdline.extend(cmdline);
        self
    }

    pub fn with_dtb(mut self, dtb_path: PathBuf) -> Self {
        self.dtb_path = Some(dtb_path);
        self
    }

    pub fn to_qemu_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        args.extend([
            "-kernel".to_string(),
            self.kernel_path.to_string_lossy().to_string(),
        ]);

        if let Some(ref initrd) = self.initrd_path {
            args.extend(["-initrd".to_string(), initrd.to_string_lossy().to_string()]);
        }

        if let Some(ref dtb) = self.dtb_path {
            args.extend(["-dtb".to_string(), dtb.to_string_lossy().to_string()]);
        }

        args.extend(["-append".to_string(), self.kernel_cmdline.join(" ")]);

        args
    }
}

pub struct KernelBootManager {
    common_cmdline: Vec<String>,
}

impl KernelBootManager {
    pub fn new() -> Self {
        Self {
            common_cmdline: vec![
                "console=hvc0".to_string(),
                "loglevel=3".to_string(),
                "systemd.log_target=quiet".to_string(),
                "panic=1".to_string(),
            ],
        }
    }

    pub fn linux_standard_cmdline(&self) -> Vec<String> {
        let mut cmdline = self.common_cmdline.clone();
        cmdline.extend([
            "root=/dev/vda2".to_string(),
            "rw".to_string(),
            "quiet".to_string(),
            "splash".to_string(),
        ]);
        cmdline
    }

    pub fn minimal_cmdline(&self) -> Vec<String> {
        let mut cmdline = self.common_cmdline.clone();
        cmdline.extend(["root=/dev/vda2".to_string(), "rw".to_string()]);
        cmdline
    }

    pub fn build_config(
        &self,
        kernel_path: PathBuf,
        initrd: Option<PathBuf>,
        minimal: bool,
    ) -> KernelConfig {
        let config = KernelConfig::new(kernel_path).with_cmdline(if minimal {
            self.minimal_cmdline()
        } else {
            self.linux_standard_cmdline()
        });

        if let Some(initrd) = initrd {
            config.with_initrd(initrd)
        } else {
            config
        }
    }
}

impl Default for KernelBootManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_config_to_args() {
        let config = KernelConfig::new(PathBuf::from("/boot/vmlinuz"))
            .with_initrd(PathBuf::from("/boot/initrd.img"))
            .with_cmdline(vec![
                "console=hvc0".to_string(),
                "root=/dev/vda".to_string(),
            ]);

        let args = config.to_qemu_args();
        assert!(args.contains(&"-kernel".to_string()));
        assert!(args.contains(&"-initrd".to_string()));
        assert!(args.contains(&"-append".to_string()));
    }
}
