//! Error types for the LangChain plugin

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur in the LangChain plugin
#[derive(Debug, Error)]
pub enum LangChainError {
    #[error("Service not running: {0}")]
    ServiceNotRunning(String),

    #[error("Service spawn failed: {0}")]
    SpawnFailed(String),

    #[error("IPC communication error: {0}")]
    IpcError(String),

    #[error("JSON-RPC error: code={code}, message={message}")]
    JsonRpcError { code: i32, message: String },

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Python binary not found: {0}")]
    BinaryNotFound(String),

    #[error("Ingestion failed: {0}")]
    IngestionFailed(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for LangChain operations
pub type LangChainResult<T> = Result<T, LangChainError>;

/// Serializable error for Tauri commands
#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableError {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
}

impl From<LangChainError> for SerializableError {
    fn from(err: LangChainError) -> Self {
        let (code, message, details) = match &err {
            LangChainError::ServiceNotRunning(msg) => {
                ("SERVICE_NOT_RUNNING".to_string(), err.to_string(), Some(msg.clone()))
            }
            LangChainError::SpawnFailed(msg) => {
                ("SPAWN_FAILED".to_string(), err.to_string(), Some(msg.clone()))
            }
            LangChainError::IpcError(msg) => {
                ("IPC_ERROR".to_string(), err.to_string(), Some(msg.clone()))
            }
            LangChainError::JsonRpcError { code, message } => {
                ("JSONRPC_ERROR".to_string(), err.to_string(), Some(format!("code: {}, msg: {}", code, message)))
            }
            LangChainError::Timeout(msg) => {
                ("TIMEOUT".to_string(), err.to_string(), Some(msg.clone()))
            }
            LangChainError::BinaryNotFound(path) => {
                ("BINARY_NOT_FOUND".to_string(), err.to_string(), Some(path.clone()))
            }
            LangChainError::IngestionFailed(msg) => {
                ("INGESTION_FAILED".to_string(), err.to_string(), Some(msg.clone()))
            }
            LangChainError::QueryFailed(msg) => {
                ("QUERY_FAILED".to_string(), err.to_string(), Some(msg.clone()))
            }
            LangChainError::Io(e) => {
                ("IO_ERROR".to_string(), err.to_string(), Some(e.to_string()))
            }
            LangChainError::Json(e) => {
                ("JSON_ERROR".to_string(), err.to_string(), Some(e.to_string()))
            }
            LangChainError::Internal(msg) => {
                ("INTERNAL_ERROR".to_string(), err.to_string(), Some(msg.clone()))
            }
        };

        SerializableError {
            code,
            message,
            details,
        }
    }
}

impl std::fmt::Display for SerializableError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
