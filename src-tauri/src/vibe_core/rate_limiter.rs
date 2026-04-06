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
        let active = self.requests.iter().filter(|&&t| now.duration_since(t) < window).count();
        self.config.max_requests.saturating_sub(active as u32)
    }
}

/// Global rate limiter for all Tauri commands
pub struct CommandRateLimiter {
    /// Per-command rate limiting state
    limiters: Arc<Mutex<HashMap<String, RateLimitState>>>,
    /// Default configuration
    default_config: RateLimitConfig,
}

impl CommandRateLimiter {
    pub fn new(default_config: RateLimitConfig) -> Self {
        Self {
            limiters: Arc::new(Mutex::new(HashMap::new())),
            default_config,
        }
    }

    /// Check if a command is allowed
    pub async fn allow_command(&self, command: &str) -> bool {
        let mut limiters = self.limiters.lock().await;
        let state = limiters
            .entry(command.to_string())
            .or_insert_with(|| RateLimitState::new(self.default_config.clone()));
        state.allow_request()
    }

    /// Get remaining requests for a command
    pub async fn remaining(&self, command: &str) -> u32 {
        let limiters = self.limiters.lock().await;
        if let Some(state) = limiters.get(command) {
            state.remaining()
        } else {
            self.default_config.max_requests
        }
    }

    /// Create a rate limiter for expensive operations
    pub fn expensive_operations() -> Self {
        Self::new(RateLimitConfig::new(10, 60)) // 10 requests per minute
    }

    /// Create a rate limiter for VM creation
    pub fn vm_creation() -> Self {
        Self::new(RateLimitConfig::new(5, 300)) // 5 requests per 5 minutes
    }

    /// Create a rate limiter for VM import/export
    pub fn vm_migration() -> Self {
        Self::new(RateLimitConfig::new(3, 300)) // 3 requests per 5 minutes
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
        let limiter = CommandRateLimiter::new(RateLimitConfig::new(2, 60));

        assert!(limiter.allow_command("create_vm").await);
        assert!(limiter.allow_command("create_vm").await);
        assert!(!limiter.allow_command("create_vm").await);

        // Different command should still have full quota
        assert!(limiter.allow_command("delete_vm").await);
    }
}
