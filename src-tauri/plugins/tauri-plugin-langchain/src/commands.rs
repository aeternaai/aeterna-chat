//! Tauri commands for the LangChain plugin

use std::collections::HashMap;
use tauri::{Runtime, State};
use serde::{Deserialize, Serialize};

use crate::error::{LangChainError, SerializableError};
use crate::protocol::{requests, responses};
use crate::service;
use crate::state::{LangChainState, ServiceConfig, ServiceInfo};

/// Result type for commands that can fail
type CommandResult<T> = Result<T, SerializableError>;

/// Convert LangChainError to SerializableError for Tauri
fn to_command_error(e: LangChainError) -> SerializableError {
    SerializableError::from(e)
}

/// Spawn the LangChain service
/// 
/// This starts the Python LangChain service subprocess.
/// The service will be ready to accept ingestion and query requests.
#[tauri::command]
pub async fn spawn_langchain_service<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: State<'_, LangChainState>,
    binary_path: Option<String>,
    env_vars: Option<HashMap<String, String>>,
    config: Option<ServiceConfig>,
) -> CommandResult<ServiceInfo> {
    log::info!("Command: spawn_langchain_service");

    // Update config if provided
    if let Some(cfg) = config {
        let mut state_config = state.config.lock().await;
        *state_config = cfg;
    }

    // Spawn service
    service::spawn_service(&state, binary_path.as_deref(), env_vars)
        .await
        .map_err(to_command_error)
}

/// Shutdown the LangChain service
/// 
/// Gracefully stops the Python subprocess.
#[tauri::command]
pub async fn shutdown_langchain_service<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: State<'_, LangChainState>,
) -> CommandResult<()> {
    log::info!("Command: shutdown_langchain_service");

    service::shutdown_service(&state)
        .await
        .map_err(to_command_error)
}

/// Check the health of the LangChain service
/// 
/// Returns health status including uptime and version.
#[tauri::command]
pub async fn health_check<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: State<'_, LangChainState>,
) -> CommandResult<responses::HealthResponse> {
    log::info!("Command: health_check");

    service::perform_health_check(&state)
        .await
        .map_err(to_command_error)
}

/// Request parameters for document ingestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestDocumentParams {
    /// Path to the document file
    pub file_path: String,
    /// Collection name for storage
    pub collection: String,
    /// Chunk size in characters (optional, uses default if not specified)
    pub chunk_size: Option<u32>,
    /// Chunk overlap in characters (optional, uses default if not specified)
    pub chunk_overlap: Option<u32>,
    /// Additional metadata to attach to chunks
    pub metadata: Option<serde_json::Value>,
}

/// Ingest a document into the RAG system
/// 
/// Processes the document, generates embeddings, and stores in the specified collection.
#[tauri::command]
pub async fn ingest_document<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: State<'_, LangChainState>,
    params: IngestDocumentParams,
) -> CommandResult<responses::IngestResponse> {
    log::info!("Command: ingest_document - file: {}, collection: {}", 
        params.file_path, params.collection);

    // Verify file exists
    if !std::path::Path::new(&params.file_path).exists() {
        return Err(to_command_error(LangChainError::IngestionFailed(
            format!("File not found: {}", params.file_path)
        )));
    }

    // Build request
    let request = requests::ingest(requests::IngestParams {
        file_path: params.file_path,
        collection: params.collection,
        chunk_size: params.chunk_size,
        chunk_overlap: params.chunk_overlap,
        metadata: params.metadata,
    });

    // Send to service
    let response = service::send_request(&state, request)
        .await
        .map_err(to_command_error)?;

    // Parse response
    let ingest_response: responses::IngestResponse = serde_json::from_value(response)
        .map_err(|e| to_command_error(LangChainError::IpcError(
            format!("Invalid ingest response: {}", e)
        )))?;

    log::info!("Ingestion complete: {} chunks in collection '{}'", 
        ingest_response.chunks_count, ingest_response.collection);

    Ok(ingest_response)
}

/// Request parameters for RAG query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRagParams {
    /// The question to answer
    pub query: String,
    /// Collection to search
    pub collection: String,
    /// Number of documents to retrieve (optional, uses default if not specified)
    pub k: Option<u32>,
    /// Minimum similarity score threshold
    pub score_threshold: Option<f32>,
    /// Optional metadata filter
    pub filter: Option<serde_json::Value>,
}

/// Query the RAG system
/// 
/// Retrieves relevant context and generates an answer using the LLM.
#[tauri::command]
pub async fn query_rag<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: State<'_, LangChainState>,
    params: QueryRagParams,
) -> CommandResult<responses::QueryResponse> {
    log::info!("Command: query_rag - query: '{}...' in collection: {}", 
        params.query.chars().take(50).collect::<String>(), params.collection);

    // Build request
    let request = requests::query(requests::QueryParams {
        query: params.query,
        collection: params.collection,
        k: params.k,
        score_threshold: params.score_threshold,
        filter: params.filter,
    });

    // Send to service
    let response = service::send_request(&state, request)
        .await
        .map_err(to_command_error)?;

    // Parse response
    let query_response: responses::QueryResponse = serde_json::from_value(response)
        .map_err(|e| to_command_error(LangChainError::IpcError(
            format!("Invalid query response: {}", e)
        )))?;

    log::info!("Query complete: {} sources, answer length: {}", 
        query_response.num_sources, query_response.answer.len());

    Ok(query_response)
}

/// Get information about the LangChain service
/// 
/// Returns configuration and status information.
#[tauri::command]
pub async fn get_service_info<R: Runtime>(
    _app: tauri::AppHandle<R>,
    state: State<'_, LangChainState>,
) -> CommandResult<ServiceInfoExtended> {
    log::info!("Command: get_service_info");

    // Get basic service info
    let service_info = state.get_info().await;
    let is_running = service_info.is_some();

    // If running, get detailed info from service
    let pipeline_info = if is_running {
        let request = requests::get_info();
        match service::send_request(&state, request).await {
            Ok(response) => {
                serde_json::from_value::<responses::InfoResponse>(response).ok()
            }
            Err(e) => {
                log::warn!("Failed to get pipeline info: {}", e);
                None
            }
        }
    } else {
        None
    };

    Ok(ServiceInfoExtended {
        is_running,
        service: service_info,
        pipeline: pipeline_info,
    })
}

/// Extended service info including pipeline configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfoExtended {
    pub is_running: bool,
    pub service: Option<ServiceInfo>,
    pub pipeline: Option<responses::InfoResponse>,
}
