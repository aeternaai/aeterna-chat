use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Configuration for the Python router service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    /// Host to bind the router service to
    pub host: String,
    
    /// Port to run the router service on
    pub port: u16,
    
    /// Log level for the router service
    pub log_level: String,
    
    /// Path to Python executable (optional, will use system python if not set)
    pub python_path: Option<String>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8765,
            log_level: "info".to_string(),
            python_path: None,
        }
    }
}

/// Request to route a query to a model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRequest {
    pub messages: Vec<Value>,
    #[serde(rename = "threadId")]
    pub thread_id: Option<String>,
    #[serde(rename = "availableModels")]
    pub available_models: Vec<Value>,
    #[serde(rename = "activeModels")]
    pub active_models: Vec<String>,
    pub attachments: Option<Value>,
    pub preferences: Option<Value>,
}

/// Response from routing request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResponse {
    #[serde(rename = "modelId")]
    pub model_id: String,
    #[serde(rename = "providerId")]
    pub provider_id: String,
    pub confidence: f64,
    pub reasoning: String,
    pub metadata: Option<Value>,
}

/// Health check response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub active_strategy: String,
}

/// Strategy information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyInfo {
    pub name: String,
    pub description: String,
}
