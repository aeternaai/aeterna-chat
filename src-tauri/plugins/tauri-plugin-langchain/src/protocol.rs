//! JSON-RPC 2.0 protocol implementation for LangChain service communication

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
    pub id: String,
}

impl JsonRpcRequest {
    pub fn new(method: &str, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Uuid::new_v4().to_string(),
        }
    }
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
    pub id: String,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    /// Check if the response indicates success
    pub fn is_success(&self) -> bool {
        self.error.is_none() && self.result.is_some()
    }

    /// Get the result value, returning an error if the response was an error
    pub fn into_result(self) -> Result<Value, JsonRpcError> {
        if let Some(error) = self.error {
            Err(error)
        } else {
            Ok(self.result.unwrap_or(Value::Null))
        }
    }
}

/// Standard JSON-RPC error codes
pub mod error_codes {
    pub const PARSE_ERROR: i32 = -32700;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    
    // Custom error codes (application-specific)
    pub const SERVICE_NOT_READY: i32 = -32000;
    pub const INGESTION_ERROR: i32 = -32001;
    pub const QUERY_ERROR: i32 = -32002;
    pub const EMBEDDING_ERROR: i32 = -32003;
    pub const COLLECTION_ERROR: i32 = -32004;
}

/// Request types for the LangChain service
pub mod requests {
    use super::*;

    /// Health check request
    pub fn health() -> JsonRpcRequest {
        JsonRpcRequest::new("health", serde_json::json!({}))
    }

    /// Shutdown request
    pub fn shutdown() -> JsonRpcRequest {
        JsonRpcRequest::new("shutdown", serde_json::json!({}))
    }

    /// Document ingestion request
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct IngestParams {
        pub file_path: String,
        pub collection: String,
        pub chunk_size: Option<u32>,
        pub chunk_overlap: Option<u32>,
        pub metadata: Option<Value>,
    }

    pub fn ingest(params: IngestParams) -> JsonRpcRequest {
        JsonRpcRequest::new("ingest", serde_json::to_value(params).unwrap())
    }

    /// RAG query request
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct QueryParams {
        pub query: String,
        pub collection: String,
        pub k: Option<u32>,
        pub score_threshold: Option<f32>,
        pub filter: Option<Value>,
    }

    pub fn query(params: QueryParams) -> JsonRpcRequest {
        JsonRpcRequest::new("query", serde_json::to_value(params).unwrap())
    }

    /// Get service info request
    pub fn get_info() -> JsonRpcRequest {
        JsonRpcRequest::new("get_info", serde_json::json!({}))
    }
}

/// Response types from the LangChain service
pub mod responses {
    use super::*;

    /// Health check response
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct HealthResponse {
        pub status: String,
        pub version: String,
        pub uptime_seconds: f64,
    }

    /// Document ingestion response
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct IngestResponse {
        pub collection: String,
        pub file_name: String,
        pub chunks_count: u32,
        pub doc_ids: Vec<String>,
        pub processing_time_ms: u64,
    }

    /// RAG query response
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct QueryResponse {
        pub answer: String,
        pub sources: Vec<SourceDocument>,
        pub query: String,
        pub collection: String,
        pub num_sources: u32,
        pub processing_time_ms: u64,
    }

    /// Source document in query response
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SourceDocument {
        pub file: String,
        pub chunk: String,
        pub score: f32,
        pub chunk_index: u32,
        pub metadata: Option<Value>,
    }

    /// Service info response
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct InfoResponse {
        pub version: String,
        pub embeddings_model: String,
        pub chunk_size: u32,
        pub chunk_overlap: u32,
        pub retrieval_top_k: u32,
        pub llm_server_url: Option<String>,
    }
}
