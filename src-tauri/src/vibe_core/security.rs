//! Security utilities for VibeVisor
//!
//! This module provides secure helpers for:
//! - Cryptographically secure password generation
//! - Secure temporary file/directory management
//! - Path validation and sanitization
//! - Archive inspection

use std::path::Path;
use ring::rand::{SecureRandom, SystemRandom};
use tempfile::TempDir;
use crate::vibe_core::Result;

/// Generate a cryptographically secure password
/// 
/// Uses the `ring` crate's secure random number generator
/// to generate passwords with high entropy.
/// 
/// # Arguments
/// * `length` - Desired password length (minimum 16, recommended 32+)
/// 
/// # Returns
/// Base64-encoded password string
pub fn generate_secure_password(length: usize) -> String {
    let length = length.max(16); // Enforce minimum
    
    // Use alphanumeric character set (62 chars)
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    
    let mut bytes = vec![0u8; length];
    let rng = SystemRandom::new();
    
    // Fill with secure random bytes
    SecureRandom::fill(&rng, &mut bytes)
        .expect("Failed to generate secure random bytes");
    
    // Map to character set
    bytes.iter_mut().for_each(|b| {
        *b = CHARSET[*b as usize % CHARSET.len()];
    });
    
    String::from_utf8(bytes).expect("Generated password is not valid UTF-8")
}

/// Create a secure temporary directory with restrictive permissions
/// 
/// The directory is created with permissions that restrict access
/// to the current user only (0o700 on Unix systems).
/// 
/// # Returns
/// TempDir handle (automatically cleaned up when dropped)
pub fn create_secure_temp_dir(prefix: &str) -> Result<TempDir> {
    let temp_dir = tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .map_err(|e| crate::vibe_core::Error::Storage(
            format!("Failed to create secure temp directory: {}", e)
        ))?;
    
    // Set restrictive permissions on Unix systems
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = temp_dir.path().metadata() {
            let mut permissions = metadata.permissions();
            permissions.set_mode(0o700); // rwx------ (owner only)
            let _ = std::fs::set_permissions(temp_dir.path(), permissions);
        }
    }
    
    tracing::info!("Created secure temp directory: {:?}", temp_dir.path());
    Ok(temp_dir)
}

/// Validate that a path is safe and doesn't contain traversal attempts
/// 
/// Checks for:
/// - Path traversal sequences (..)
/// - Absolute paths when relative expected
/// - Symlink attacks
/// - Null bytes
/// 
/// # Arguments
/// * `path` - Path to validate
/// * `base_dir` - Expected base directory (path must be within this)
/// 
/// # Returns
/// true if path is safe, false otherwise
pub fn is_path_safe(path: &Path, base_dir: &Path) -> bool {
    // Check for null bytes
    if path.to_string_lossy().contains('\0') {
        return false;
    }
    
    // Check for path traversal components
    for component in path.components() {
        if let std::path::Component::ParentDir = component {
            return false;
        }
    }
    
    // Canonicalize and verify containment
    if let (Ok(canonical_path), Ok(canonical_base)) = (
        std::fs::canonicalize(path),
        std::fs::canonicalize(base_dir)
    ) {
        return canonical_path.starts_with(canonical_base);
    }
    
    false
}

/// Inspect an archive file before extraction
/// 
/// Returns information about the archive contents including:
/// - List of files
/// - Total uncompressed size
/// - Detection of suspicious paths
/// 
/// # Arguments
/// * `archive_path` - Path to the tar.gz archive
/// 
/// # Returns
/// Archive inspection result or error
pub fn inspect_archive(archive_path: &Path) -> Result<ArchiveInfo> {
    use std::fs::File;
    use std::io::BufReader;
    
    let file = File::open(archive_path)
        .map_err(|e| crate::vibe_core::Error::Storage(
            format!("Failed to open archive: {}", e)
        ))?;
    
    let decoder = flate2::read::GzDecoder::new(BufReader::new(file));
    let mut archive = tar::Archive::new(decoder);
    
    let mut total_size: u64 = 0;
    let mut file_count: usize = 0;
    let mut suspicious_entries: Vec<String> = Vec::new();
    let mut entries: Vec<String> = Vec::new();
    
    let entries_iter = archive.entries()
        .map_err(|e| crate::vibe_core::Error::Storage(
            format!("Failed to read archive entries: {}", e)
        ))?;

    for entry_result in entries_iter {
        let entry = entry_result.map_err(|e| crate::vibe_core::Error::Storage(
            format!("Failed to read archive entry: {}", e)
        ))?;

        let path = entry.path()
            .map_err(|e| crate::vibe_core::Error::Storage(
                format!("Invalid path in archive entry: {}", e)
            ))?
            .to_path_buf();
        
        let path_str = path.to_string_lossy().to_string();
        entries.push(path_str.clone());
        file_count += 1;
        
        // Check for suspicious paths
        if path_str.starts_with('/') || 
           path_str.starts_with("..") ||
           path_str.contains("..") ||
           path_str.contains('\0') {
            suspicious_entries.push(path_str);
        }
        
        // Accumulate size
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

/// Archive inspection result
#[derive(Debug, Clone)]
pub struct ArchiveInfo {
    pub file_count: usize,
    pub total_size: u64,
    pub suspicious_entries: Vec<String>,
    pub entries: Vec<String>,
}

impl ArchiveInfo {
    /// Check if archive is safe to extract
    pub fn is_safe(&self, max_size: u64) -> bool {
        // No suspicious entries
        if !self.suspicious_entries.is_empty() {
            tracing::warn!(
                "Archive contains suspicious entries: {:?}",
                self.suspicious_entries
            );
            return false;
        }
        
        // Size limit check
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
/// 
/// Returns the path as an OsString to avoid shell interpretation issues
pub fn validate_command_path(path: &Path) -> Result<std::ffi::OsString> {
    // Ensure path exists
    if !path.exists() {
        return Err(crate::vibe_core::Error::Storage(
            format!("Path does not exist: {:?}", path)
        ));
    }
    
    // Canonicalize to resolve symlinks
    let canonical = std::fs::canonicalize(path)
        .map_err(|e| crate::vibe_core::Error::Storage(
            format!("Failed to canonicalize path: {}", e)
        ))?;
    
    Ok(canonical.into_os_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_secure_password_generation() {
        let pwd1 = generate_secure_password(32);
        let pwd2 = generate_secure_password(32);
        
        // Passwords should be different (extremely unlikely to be same)
        assert_ne!(pwd1, pwd2);
        
        // Both should be 32 characters
        assert_eq!(pwd1.len(), 32);
        assert_eq!(pwd2.len(), 32);
        
        // Should only contain alphanumeric characters
        assert!(pwd1.chars().all(|c| c.is_alphanumeric()));
    }
    
    #[test]
    fn test_minimum_password_length() {
        let pwd = generate_secure_password(8);
        assert_eq!(pwd.len(), 16); // Should enforce minimum of 16
    }
    
    #[test]
    fn test_secure_temp_dir_creation() {
        let temp_dir = create_secure_temp_dir("vibevisor-test-");
        assert!(temp_dir.is_ok());
        
        let dir = temp_dir.unwrap();
        assert!(dir.path().exists());
        
        // Check permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = std::fs::metadata(dir.path()).unwrap();
            let mode = metadata.permissions().mode() & 0o777;
            assert_eq!(mode, 0o700, "Temp directory should have restrictive permissions");
        }
    }
    
    #[test]
    fn test_path_safety_validation() {
        let base = std::env::temp_dir();
        
        // Safe path
        let safe_path = base.join("subdir/file.txt");
        // Create it for canonicalization
        std::fs::create_dir_all(safe_path.parent().unwrap()).ok();
        std::fs::write(&safe_path, "test").ok();
        assert!(is_path_safe(&safe_path, &base));
        
        // Cleanup
        std::fs::remove_file(&safe_path).ok();
    }
}
