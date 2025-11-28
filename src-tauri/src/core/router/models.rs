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
    
    /// Router model ID for LLM-based routing (optional, defaults to Phi-4-mini-instruct_Q4_K_M)
    pub router_model: Option<String>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8765,
            log_level: "info".to_string(),
            python_path: None,
            router_model: Some("Phi-4-mini-instruct_Q4_K_M".to_string()),
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
    #[serde(rename = "routerModel")]
    pub router_model: Option<Value>,
    #[serde(rename = "activeModels")]
    pub active_models: Vec<String>,
    #[serde(rename = "modelRoutingConfigs")]
    pub model_routing_configs: Option<Vec<Value>>,
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

/// Configuration payload for updating the LLM router
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterLLMConfig {
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub temperature: Option<f32>,
    #[serde(rename = "maxTokens")]
    pub max_tokens: Option<u32>,
    pub timeout: Option<f32>,
}
