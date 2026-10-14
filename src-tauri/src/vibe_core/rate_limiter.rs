//! Rate limiting for VibeVisor Tauri commands
//!
//! Provides token bucket rate limiting to prevent resource exhaustion

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum number of requests allowed in the window
    pub max_requests: u32,
    /// Time window in seconds
    pub window_seconds: u64,
}

impl RateLimitConfig {
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            max_requests,
            window_seconds,
        }
    }
}

/// Rate limiter state for a single command
#[derive(Debug)]
struct RateLimitState {
    /// Timestamps of recent requests
    requests: Vec<Instant>,
    /// Configuration
    config: RateLimitConfig,
}

impl RateLimitState {
    fn new(config: RateLimitConfig) -> Self {
        Self {
            requests: Vec::new(),
            config,
        }
    }

    /// Check if a request is allowed and record it
    fn allow_request(&mut self) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_seconds);

        // Remove expired entries
        self.requests.retain(|&t| now.duration_since(t) < window);

        // Check if under limit
        if self.requests.len() < self.config.max_requests as usize {
            self.requests.push(now);
            true
        } else {
            false
        }
    }

    /// Get remaining requests in current window
    fn remaining(&self) -> u32 {
        let now = Instant::now();
        let window = Duration::from_secs(self.config.window_seconds);
        let active = self
            .requests
            .iter()
            .filter(|&&t| now.duration_since(t) < window)
            .count();
        self.config.max_requests.saturating_sub(active as u32)
    }
}

/// Global rate limiter for all Tauri commands.
///
/// Supports a generic default plus per-command overrides so that expensive
/// operations (VM creation, import/export) can be limited more strictly than
/// cheap state transitions.
pub struct CommandRateLimiter {
    /// Per-command rate limiting state
    limiters: Arc<Mutex<HashMap<String, RateLimitState>>>,
    /// Default configuration applied to commands without an override
    default_config: RateLimitConfig,
    /// Per-command configuration overrides
    overrides: HashMap<String, RateLimitConfig>,
}

impl CommandRateLimiter {
    pub fn new(default_config: RateLimitConfig) -> Self {
        Self {
            limiters: Arc::new(Mutex::new(HashMap::new())),
            default_config,
            overrides: Self::default_overrides(),
        }
    }

    /// Purpose-built per-command limits.
    ///
    /// These replace the previous unused preset constructors and are actually
    /// consulted on every `allow_command` call.
    fn default_overrides() -> HashMap<String, RateLimitConfig> {
        let mut m = HashMap::new();
        // VM creation: 5 per 5 minutes.
        m.insert("create_vm".to_string(), RateLimitConfig::new(5, 300));
        // Import/export (migration): 3 per 5 minutes.
        m.insert("import_vm".to_string(), RateLimitConfig::new(3, 300));
        m.insert("export_vm".to_string(), RateLimitConfig::new(3, 300));
        // Other expensive state changes: 10 per minute.
        for cmd in ["start_vm", "stop_vm", "delete_vm", "update_vm"] {
            m.insert(cmd.to_string(), RateLimitConfig::new(10, 60));
        }
        m
    }

    fn config_for(&self, command: &str) -> RateLimitConfig {
        self.overrides
            .get(command)
            .cloned()
            .unwrap_or_else(|| self.default_config.clone())
    }

    /// Check if a command is allowed
    pub async fn allow_command(&self, command: &str) -> bool {
        let config = self.config_for(command);
        let mut limiters = self.limiters.lock().await;
        let state = limiters
            .entry(command.to_string())
            .or_insert_with(|| RateLimitState::new(config));
        state.allow_request()
    }

    /// Get remaining requests for a command
    pub async fn remaining(&self, command: &str) -> u32 {
        let limiters = self.limiters.lock().await;
        if let Some(state) = limiters.get(command) {
            state.remaining()
        } else {
            self.config_for(command).max_requests
        }
    }
}

impl Default for CommandRateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::new(100, 60)) // Default: 100 requests per minute
    }
}

/// Rate limit error
#[derive(Debug, Clone)]
pub struct RateLimitError {
    pub command: String,
    pub retry_after_seconds: u64,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rate limit exceeded for '{}'. Please retry after {} seconds.",
            self.command, self.retry_after_seconds
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiting() {
        let limiter = CommandRateLimiter::new(RateLimitConfig::new(3, 60));

        // First 3 requests should be allowed
        assert!(limiter.allow_command("test").await);
        assert!(limiter.allow_command("test").await);
        assert!(limiter.allow_command("test").await);

        // 4th request should be rejected
        assert!(!limiter.allow_command("test").await);

        // Check remaining
        assert_eq!(limiter.remaining("test").await, 0);
    }

    #[tokio::test]
    async fn test_different_commands() {
        // Use command names WITHOUT per-command overrides so the configured
        // default applies deterministically.
        let limiter = CommandRateLimiter::new(RateLimitConfig::new(2, 60));

        assert!(limiter.allow_command("custom_a").await);
        assert!(limiter.allow_command("custom_a").await);
        assert!(!limiter.allow_command("custom_a").await);

        // Different command should still have full quota
        assert!(limiter.allow_command("custom_b").await);
    }

    #[tokio::test]
    async fn test_per_command_override_applies() {
        // create_vm has a stricter override (5 per 5 min) regardless of the
        // generic default (100 per min here).
        let limiter = CommandRateLimiter::new(RateLimitConfig::new(100, 60));
        for _ in 0..5 {
            assert!(limiter.allow_command("create_vm").await);
        }
        assert!(!limiter.allow_command("create_vm").await);

        // A command with no override uses the generic default.
        assert!(limiter.allow_command("list_vms").await);
    }
}
