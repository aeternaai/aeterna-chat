//! # Tauri Plugin: LangChain
//! 
//! This plugin provides integration with LangChain for advanced RAG capabilities.
//! It manages a Python subprocess that runs LangChain orchestration, communicating
//! via JSON-RPC 2.0 over stdio.
//! 
//! ## Features
//! 
//! - **Document Ingestion**: Process documents, split into chunks, generate embeddings
//! - **RAG Query**: Retrieve relevant context and generate answers
//! - **Service Management**: Spawn, monitor, and shutdown the Python service
//! 
//! ## Architecture
//! 
//! ```text
//! Tauri Commands ──► LangChain Plugin ──► Python Subprocess
//!                          │                    │
//!                    JSON-RPC 2.0          LangChain
//!                    over stdio            Orchestration
//! ```
//! 
//! ## Usage
//! 
//! Add the plugin to your Tauri app:
//! 
//! ```rust,ignore
//! fn main() {
//!     tauri::Builder::default()
//!         .plugin(tauri_plugin_langchain::init())
//!         .run(tauri::generate_context!())
//!         .expect("error while running tauri application");
//! }
//! ```

pub mod commands;
pub mod error;
pub mod protocol;
pub mod service;
pub mod state;

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

use state::{LangChainState, ServiceConfig};

/// Default name for the plugin
const PLUGIN_NAME: &str = "langchain";

/// Plugin builder for customizing initialization
pub struct LangChainPluginBuilder {
    config: ServiceConfig,
}

impl Default for LangChainPluginBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl LangChainPluginBuilder {
    /// Create a new plugin builder with default configuration
    pub fn new() -> Self {
        Self {
            config: ServiceConfig::default(),
        }
    }

    /// Set the timeout for health checks (in milliseconds)
    pub fn health_check_interval(mut self, ms: u64) -> Self {
        self.config.health_check_interval_ms = ms;
        self
    }

    /// Set the timeout for IPC calls (in milliseconds)
    pub fn call_timeout(mut self, ms: u64) -> Self {
        self.config.call_timeout_ms = ms;
        self
    }

    /// Set the maximum number of restart attempts
    pub fn max_restart_attempts(mut self, attempts: u32) -> Self {
        self.config.max_restart_attempts = attempts;
        self
    }

    /// Enable or disable auto-restart on failure
    pub fn auto_restart(mut self, enabled: bool) -> Self {
        self.config.auto_restart = enabled;
        self
    }

    /// Build the plugin
    pub fn build<R: Runtime>(self) -> TauriPlugin<R> {
        build_plugin(self.config)
    }
}

/// Build the plugin with the given configuration
fn build_plugin<R: Runtime>(config: ServiceConfig) -> TauriPlugin<R> {
    Builder::new(PLUGIN_NAME)
        .invoke_handler(tauri::generate_handler![
            commands::spawn_langchain_service,
            commands::shutdown_langchain_service,
            commands::health_check,
            commands::ingest_document,
            commands::query_rag,
            commands::get_service_info,
        ])
        .setup(move |app, _api| {
            log::info!("Setting up LangChain plugin");
            
            // Create plugin state with configuration
            let state = LangChainState::with_config(config.clone());
            
            // Manage state
            app.manage(state);
            
            log::info!("LangChain plugin initialized successfully");
            Ok(())
        })
        .on_drop(|app| {
            log::info!("LangChain plugin dropping, shutting down service");
            
            // Try to get state and shutdown service
            if let Some(state) = app.try_state::<LangChainState>() {
                // Use a blocking call since we're in drop
                let rt = tokio::runtime::Handle::current();
                rt.block_on(async {
                    if let Err(e) = service::shutdown_service(&state).await {
                        log::warn!("Error shutting down LangChain service: {}", e);
                    }
                });
            }
        })
        .build()
}

/// Initialize the plugin with default configuration
/// 
/// # Example
/// 
/// ```rust,ignore
/// tauri::Builder::default()
///     .plugin(tauri_plugin_langchain::init())
///     .run(tauri::generate_context!())
///     .expect("error while running tauri application");
/// ```
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    LangChainPluginBuilder::new().build()
}

/// Create a plugin builder for custom configuration
/// 
/// # Example
/// 
/// ```rust,ignore
/// tauri::Builder::default()
///     .plugin(
///         tauri_plugin_langchain::builder()
///             .health_check_interval(10000)
///             .call_timeout(60000)
///             .auto_restart(true)
///             .build()
///     )
///     .run(tauri::generate_context!())
///     .expect("error while running tauri application");
/// ```
pub fn builder() -> LangChainPluginBuilder {
    LangChainPluginBuilder::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_default() {
        let builder = LangChainPluginBuilder::new();
        assert_eq!(builder.config.health_check_interval_ms, 30000);
        assert_eq!(builder.config.call_timeout_ms, 30000);
        assert_eq!(builder.config.max_restart_attempts, 3);
        assert!(builder.config.auto_restart);
    }

    #[test]
    fn test_builder_customization() {
        let builder = LangChainPluginBuilder::new()
            .health_check_interval(10000)
            .call_timeout(120000)
            .max_restart_attempts(5)
            .auto_restart(false);

        assert_eq!(builder.config.health_check_interval_ms, 10000);
        assert_eq!(builder.config.call_timeout_ms, 120000);
        assert_eq!(builder.config.max_restart_attempts, 5);
        assert!(!builder.config.auto_restart);
    }
}
