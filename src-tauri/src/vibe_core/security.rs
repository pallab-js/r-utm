//! Security utilities for VibeVisor
//!
//! This module provides secure helpers for:
//! - Cryptographically secure password generation
//! - Secure temporary file/directory management
//! - Path validation and sanitization
//! - Archive inspection

use crate::vibe_core::Result;
use ring::rand::{SecureRandom, SystemRandom};
use std::path::Path;
use tempfile::TempDir;

/// Generate a cryptographically secure password
///
/// Uses the `ring` crate's secure random number generator
/// to generate passwords with high entropy.
///
/// # Arguments
/// * `length` - Desired password length (minimum 16, recommended 32+)
///
/// # Returns
/// An alphanumeric password string, or an error if the system RNG fails.
pub fn generate_secure_password(length: usize) -> Result<String> {
    let length = length.max(16); // Enforce minimum

    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";

    let mut bytes = vec![0u8; length];
    let rng = SystemRandom::new();

    SecureRandom::fill(&rng, &mut bytes).map_err(|_| {
        crate::vibe_core::Error::Config("Failed to generate secure random bytes".to_string())
    })?;

    // Map each byte into the charset. Because CHARSET is pure ASCII, the
    // resulting bytes are always valid UTF-8, so this never fails in practice.
    bytes.iter_mut().for_each(|b| {
        *b = CHARSET[*b as usize % CHARSET.len()];
    });

    String::from_utf8(bytes).map_err(|_| {
        crate::vibe_core::Error::Config("Generated password is not valid UTF-8".to_string())
    })
}

/// Create a secure temporary directory with restrictive permissions
pub fn create_secure_temp_dir(prefix: &str) -> Result<TempDir> {
    let temp_dir = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .map_err(|e| {
            crate::vibe_core::Error::Storage(format!(
                "Failed to create secure temp directory: {}",
                e
            ))
        })?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = temp_dir.path().metadata() {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o700);
            let _ = std::fs::set_permissions(temp_dir.path(), permissions);
        }
    }

    tracing::info!("Created secure temp directory: {:?}", temp_dir.path());
    Ok(temp_dir)
}

/// Validate that a path is safe and doesn't contain traversal attempts
pub fn is_path_safe(path: &Path, base_dir: &Path) -> bool {
    if path.to_string_lossy().contains('\0') {
        return false;
    }

    for component in path.components() {
        if let std::path::Component::ParentDir = component {
            return false;
        }
    }

    if let (Ok(canonical_path), Ok(canonical_base)) =
        (std::fs::canonicalize(path), std::fs::canonicalize(base_dir))
    {
        return canonical_path.starts_with(canonical_base);
    }

    false
}

/// Validate a string path against a base directory
pub fn is_path_safe_str(path_str: &str, base_dir: &Path) -> bool {
    let path = Path::new(path_str);
    is_path_safe(path, base_dir)
}

/// Inspect an archive file before extraction
pub fn inspect_archive(archive_path: &Path) -> Result<ArchiveInfo> {
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open(archive_path)
        .map_err(|e| crate::vibe_core::Error::Storage(format!("Failed to open archive: {}", e)))?;

    let decoder = flate2::read::GzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(decoder);

    let mut total_size: u64 = 0;
    let mut file_count: usize = 0;
    let mut suspicious_entries: Vec<String> = Vec::new();
    let mut entries: Vec<String> = Vec::new();

    let entries_iter = archive.entries().map_err(|e| {
        crate::vibe_core::Error::Storage(format!("Failed to read archive entries: {}", e))
    })?;

    for entry_result in entries_iter {
        let entry = entry_result.map_err(|e| {
            crate::vibe_core::Error::Storage(format!("Failed to read archive entry: {}", e))
        })?;

        let path = entry
            .path()
            .map_err(|e| {
                crate::vibe_core::Error::Storage(format!("Invalid path in archive entry: {}", e))
            })?
            .to_path_buf();

        let path_str = path.to_string_lossy().to_string();
        entries.push(path_str.clone());
        file_count += 1;

        if path_str.starts_with('/')
            || path_str.starts_with("..")
            || path_str.contains("..")
            || path_str.contains('\0')
        {
            suspicious_entries.push(path_str);
        }

        if let Ok(size) = entry.header().size() {
            total_size += size;
        }
    }

    Ok(ArchiveInfo {
        file_count,
        total_size,
        suspicious_entries,
        entries,
    })
}

#[derive(Debug, Clone)]
pub struct ArchiveInfo {
    pub file_count: usize,
    pub total_size: u64,
    pub suspicious_entries: Vec<String>,
    pub entries: Vec<String>,
}

impl ArchiveInfo {
    pub fn is_safe(&self, max_size: u64) -> bool {
        if !self.suspicious_entries.is_empty() {
            tracing::warn!(
                "Archive contains suspicious entries: {:?}",
                self.suspicious_entries
            );
            return false;
        }

        if self.total_size > max_size {
            tracing::warn!(
                "Archive size {} exceeds limit {}",
                self.total_size,
                max_size
            );
            return false;
        }

        true
    }
}

/// Validate and sanitize a file path for use in command arguments
pub fn validate_command_path(path: &Path) -> Result<std::ffi::OsString> {
    if !path.exists() {
        return Err(crate::vibe_core::Error::Storage(format!(
            "Path does not exist: {:?}",
            path
        )));
    }

    let canonical = std::fs::canonicalize(path).map_err(|e| {
        crate::vibe_core::Error::Storage(format!("Failed to canonicalize path: {}", e))
    })?;

    Ok(canonical.into_os_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_password_generation() {
        let pwd1 = generate_secure_password(32).expect("rng should succeed");
        let pwd2 = generate_secure_password(32).expect("rng should succeed");

        assert_ne!(pwd1, pwd2);
        assert_eq!(pwd1.len(), 32);
        assert_eq!(pwd2.len(), 32);
        assert!(pwd1.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn test_minimum_password_length() {
        let pwd = generate_secure_password(8).expect("rng should succeed");
        assert_eq!(pwd.len(), 16);
    }

    #[test]
    fn test_secure_temp_dir_creation() {
        let temp_dir = create_secure_temp_dir("vibevisor-test-");
        assert!(temp_dir.is_ok());

        let dir = temp_dir.unwrap();
        assert!(dir.path().exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(dir.path()).unwrap();
            let mode = metadata.permissions().mode() & 0o777;
            assert_eq!(mode, 0o700);
        }
    }

    #[test]
    fn test_path_safety_validation() {
        let base = std::env::temp_dir();

        let safe_path = base.join("subdir/file.txt");
        std::fs::create_dir_all(safe_path.parent().unwrap()).ok();
        std::fs::write(&safe_path, "test").ok();
        assert!(is_path_safe(&safe_path, &base));

        std::fs::remove_file(&safe_path).ok();
    }
}
