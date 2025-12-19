use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// OAuth configuration for MCP servers requiring authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthConfig {
    /// OAuth 2.0 authorization endpoint
    pub auth_url: String,
    /// OAuth 2.0 token endpoint
    pub token_url: String,
    /// Client ID for the OAuth application
    pub client_id: String,
    /// Optional client secret (for confidential clients)
    pub client_secret: Option<String>,
    /// OAuth scopes to request
    pub scopes: Vec<String>,
    /// Redirect URI for OAuth callback (defaults to http://localhost:PORT/callback)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_uri: Option<String>,
}

/// OAuth token storage
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub token_type: String,
    pub expires_at: i64, // Unix timestamp
    pub scopes: Vec<String>,
}

impl OAuthToken {
    /// Check if the token is expired or about to expire (within 5 minutes)
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        
        // Consider expired if less than 5 minutes remaining
        self.expires_at < now + 300
    }
}

/// OAuth authentication status for an MCP server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStatus {
    pub server_name: String,
    pub authenticated: bool,
    pub expires_at: Option<i64>,
    pub scopes: Vec<String>,
}

/// Configuration parameters extracted from MCP server config
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub transport_type: Option<String>,
    pub url: Option<String>,
    pub command: String,
    pub args: Vec<Value>,
    pub envs: serde_json::Map<String, Value>,
    pub timeout: Option<Duration>,
    pub headers: serde_json::Map<String, Value>,
    pub oauth_config: Option<OAuthConfig>,
}

fn default_tool_call_timeout_seconds() -> u64 {
    super::constants::DEFAULT_MCP_TOOL_CALL_TIMEOUT_SECS
}

fn default_base_restart_delay_ms() -> u64 {
    super::constants::DEFAULT_MCP_BASE_RESTART_DELAY_MS
}

fn default_max_restart_delay_ms() -> u64 {
    super::constants::DEFAULT_MCP_MAX_RESTART_DELAY_MS
}

fn default_backoff_multiplier() -> f64 {
    super::constants::DEFAULT_MCP_BACKOFF_MULTIPLIER
}

fn default_cache_ttl_seconds() -> u64 {
    3600 // 1 hour default
}

fn default_cache_cleanup_interval_seconds() -> u64 {
    600 // 10 minutes default
}

/// Runtime MCP settings that can be adjusted via UI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct McpSettings {
    #[serde(default = "default_tool_call_timeout_seconds")]
    pub tool_call_timeout_seconds: u64,
    #[serde(default = "default_base_restart_delay_ms")]
    pub base_restart_delay_ms: u64,
    #[serde(default = "default_max_restart_delay_ms")]
    pub max_restart_delay_ms: u64,
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,
    /// TTL for cached tool outputs in seconds (default: 3600 = 1 hour)
    #[serde(default = "default_cache_ttl_seconds")]
    pub cache_ttl_seconds: u64,
    /// Interval for automatic cache cleanup in seconds (default: 600 = 10 minutes)
    #[serde(default = "default_cache_cleanup_interval_seconds")]
    pub cache_cleanup_interval_seconds: u64,
}

impl Default for McpSettings {
    fn default() -> Self {
        Self {
            tool_call_timeout_seconds: super::constants::DEFAULT_MCP_TOOL_CALL_TIMEOUT_SECS,
            base_restart_delay_ms: super::constants::DEFAULT_MCP_BASE_RESTART_DELAY_MS,
            max_restart_delay_ms: super::constants::DEFAULT_MCP_MAX_RESTART_DELAY_MS,
            backoff_multiplier: super::constants::DEFAULT_MCP_BACKOFF_MULTIPLIER,
            cache_ttl_seconds: 3600,
            cache_cleanup_interval_seconds: 600,
        }
    }
}

impl McpSettings {
    /// Returns the tool call timeout duration, enforcing a minimum of 1 second to avoid zero-duration timeouts.
    pub fn tool_call_timeout_duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.tool_call_timeout_seconds.max(1))
    }
}

/// Tool with server information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolWithServer {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    pub server: String,
}
