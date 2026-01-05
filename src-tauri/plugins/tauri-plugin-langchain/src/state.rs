//! State management for the LangChain plugin

use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::Mutex;
use tokio::io::{BufReader, BufWriter};
use tokio::process::{ChildStdin, ChildStdout};
use serde::{Deserialize, Serialize};

/// Information about the running LangChain service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub pid: u32,
    pub binary_path: String,
    pub started_at: i64,
    pub is_healthy: bool,
    pub last_health_check: Option<i64>,
    pub request_count: u64,
    pub error_count: u64,
}

/// Internal service handle with process and I/O handles
pub struct ServiceHandle {
    pub child: Child,
    pub stdin: BufWriter<ChildStdin>,
    pub stdout: BufReader<ChildStdout>,
    pub info: ServiceInfo,
}

/// Plugin state managing the LangChain service lifecycle
pub struct LangChainState {
    /// The running service handle (None if not started)
    pub service: Arc<Mutex<Option<ServiceHandle>>>,
    /// Path to the Python binary/service
    pub binary_path: Arc<Mutex<Option<String>>>,
    /// Configuration for the service
    pub config: Arc<Mutex<ServiceConfig>>,
}

/// Configuration for the LangChain service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    /// Timeout for IPC calls in milliseconds
    pub call_timeout_ms: u64,
    /// Whether to auto-restart on crash
    pub auto_restart: bool,
    /// Maximum restart attempts
    pub max_restart_attempts: u32,
    /// Current restart attempt count
    pub restart_count: u32,
    /// Delay between restart attempts in milliseconds
    pub restart_delay_ms: u64,
    /// Health check interval in milliseconds
    pub health_check_interval_ms: u64,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            call_timeout_ms: 30000,       // 30 seconds
            auto_restart: true,
            max_restart_attempts: 3,
            restart_count: 0,
            restart_delay_ms: 1000,       // 1 second
            health_check_interval_ms: 30000, // 30 seconds
        }
    }
}

impl LangChainState {
    pub fn new() -> Self {
        Self {
            service: Arc::new(Mutex::new(None)),
            binary_path: Arc::new(Mutex::new(None)),
            config: Arc::new(Mutex::new(ServiceConfig::default())),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: ServiceConfig) -> Self {
        Self {
            service: Arc::new(Mutex::new(None)),
            binary_path: Arc::new(Mutex::new(None)),
            config: Arc::new(Mutex::new(config)),
        }
    }

    /// Check if the service is currently running
    pub async fn is_running(&self) -> bool {
        let service = self.service.lock().await;
        service.is_some()
    }

    /// Get the current service info if running
    pub async fn get_info(&self) -> Option<ServiceInfo> {
        let service = self.service.lock().await;
        service.as_ref().map(|s| s.info.clone())
    }

    /// Increment request counter
    pub async fn increment_request_count(&self) {
        let mut service = self.service.lock().await;
        if let Some(ref mut handle) = *service {
            handle.info.request_count += 1;
        }
    }

    /// Increment error counter
    pub async fn increment_error_count(&self) {
        let mut service = self.service.lock().await;
        if let Some(ref mut handle) = *service {
            handle.info.error_count += 1;
        }
    }

    /// Update health status
    pub async fn set_health_status(&self, is_healthy: bool) {
        let mut service = self.service.lock().await;
        if let Some(ref mut handle) = *service {
            handle.info.is_healthy = is_healthy;
            handle.info.last_health_check = Some(chrono::Utc::now().timestamp());
        }
    }
}

impl Default for LangChainState {
    fn default() -> Self {
        Self::new()
    }
}
