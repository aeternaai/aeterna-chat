use crate::core::db;
use serde_json::Value;
use tauri::{Runtime, Manager, Emitter};
use uuid::Uuid;
use tokio_util::sync::CancellationToken;

/// Creates a new workspace in the database
#[tauri::command]
pub async fn create_workspace<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    mut workspace: Value,
) -> Result<Value, String> {
    // Generate ID if not provided
    if workspace.get("id").is_none() {
        let uuid = Uuid::new_v4().to_string();
        workspace["id"] = Value::String(uuid);
    }
    
    // Set timestamps
    let now = chrono::Utc::now().timestamp();
    if workspace.get("created_at").is_none() {
        workspace["created_at"] = Value::Number(now.into());
    }
    workspace["updated_at"] = Value::Number(now.into());
    
    db::workspaces::create_workspace(workspace).await
        .map_err(|e| e.to_string())
}

/// Gets a workspace by ID
#[tauri::command]
pub async fn get_workspace<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    workspace_id: String,
) -> Result<Option<Value>, String> {
    db::workspaces::get_workspace(&workspace_id).await
        .map_err(|e| e.to_string())
}

/// Lists all workspaces
#[tauri::command]
pub async fn list_workspaces<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<Vec<Value>, String> {
    db::workspaces::list_workspaces().await
        .map_err(|e| e.to_string())
}

/// Updates an existing workspace
#[tauri::command]
pub async fn update_workspace<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    mut workspace: Value,
) -> Result<(), String> {
    // Update timestamp
    let now = chrono::Utc::now().timestamp();
    workspace["updated_at"] = Value::Number(now.into());
    
    db::workspaces::update_workspace(workspace).await
        .map_err(|e| e.to_string())
}

/// Deletes a workspace and all its related data (CASCADE)
#[tauri::command]
pub async fn delete_workspace<R: Runtime>(
    _app_handle: tauri::AppHandle<R>,
    workspace_id: String,
) -> Result<(), String> {
    db::workspaces::delete_workspace(&workspace_id).await
        .map_err(|e| e.to_string())
}

/// Adds a file to a workspace
#[tauri::command]
pub async fn add_workspace_file<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    workspace_id: String,
    file_path: String,
    name: Option<String>,
) -> Result<Value, String> {
    let file_id = Uuid::new_v4().to_string();
    
    // Get file info
    let path = std::path::Path::new(&file_path);
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    
    let file_name = name.unwrap_or_else(|| {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    });
    
    // Get file size
    let size = std::fs::metadata(&file_path)
        .ok()
        .map(|m| m.len() as i64);
    
    // Create file record in database
    let now = chrono::Utc::now().timestamp();
    let file_record = serde_json::json!({
        "id": file_id,
        "workspace_id": workspace_id,
        "file_path": file_path,
        "name": file_name,
        "extension": extension,
        "size": size,
        "added_at": now,
        "updated_at": now,
        "is_valid": true,
        "status": "ready"
    });
    
    // Insert into database (this would need a new db function)
    // For now, store in metadata
    db::workspace_files::add_file(&workspace_id, &file_record).await
        .map_err(|e| e.to_string())?;
    
    Ok(file_record)
}

/// Lists all files in a workspace
#[tauri::command]
pub async fn list_workspace_files<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    workspace_id: String,
) -> Result<Vec<Value>, String> {
    db::workspace_files::list_files(&workspace_id).await
        .map_err(|e| e.to_string())
}

/// Removes a file from a workspace
#[tauri::command]
pub async fn remove_workspace_file<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    file_id: String,
) -> Result<(), String> {
    db::workspace_files::delete_file(&file_id).await
        .map_err(|e| e.to_string())
}

/// Updates a workspace file
#[tauri::command]
pub async fn update_workspace_file<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    mut file: Value,
) -> Result<(), String> {
    // Update timestamp
    let now = chrono::Utc::now().timestamp();
    file["updated_at"] = Value::Number(now.into());
    
    db::workspace_files::update_file(&file).await
        .map_err(|e| e.to_string())
}

/// Validates a workspace file (checks if the original file exists)
#[tauri::command]
pub async fn validate_workspace_file<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    file_id: String,
) -> Result<Value, String> {
    let file = db::workspace_files::get_file(&file_id).await
        .map_err(|e| e.to_string())?
        .ok_or("File not found")?;
    
    let file_path = file.get("file_path")
        .and_then(|v| v.as_str())
        .ok_or("File path not found")?;
    
    let now = chrono::Utc::now().timestamp();
    
    // Check if file exists
    let exists = std::path::Path::new(file_path).exists();
    
    let status = serde_json::json!({
        "file_id": file_id,
        "exists": exists,
        "readable": exists,
        "checked_at": now,
        "error": if !exists { Some("File not found") } else { None }
    });
    
    // Update file validity
    if let Ok(mut updated_file) = serde_json::from_value::<Value>(file.clone()) {
        updated_file["is_valid"] = Value::Bool(exists);
        updated_file["updated_at"] = Value::Number(now.into());
        let _ = db::workspace_files::update_file(&updated_file).await;
    }
    
    Ok(status)
}

// ============================================================================
// Workspace File RAG Commands
// ============================================================================

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceFileRagStatus {
    pub file_id: String,
    pub status: String,
    pub chunks: i64,
    pub indexed_at: Option<i64>,
    pub error: Option<String>,
}

/// Update RAG status for a workspace file
#[tauri::command]
pub async fn update_workspace_file_rag_status<R: Runtime>(
    _app: tauri::AppHandle<R>,
    file_id: String,
    status: String,
    chunks: Option<i64>,
    indexed_at: Option<i64>,
    error: Option<String>,
) -> Result<(), String> {
    db::workspace_files::update_rag_status(
        &file_id,
        &status,
        chunks,
        indexed_at,
        error.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

/// Get RAG status for a workspace file
#[tauri::command]
pub async fn get_workspace_file_rag_status<R: Runtime>(
    _app: tauri::AppHandle<R>,
    file_id: String,
) -> Result<Option<WorkspaceFileRagStatus>, String> {
    let status = db::workspace_files::get_rag_status(&file_id)
        .await
        .map_err(|e| e.to_string())?;

    Ok(status.map(|(status, chunks, indexed_at, error)| {
        WorkspaceFileRagStatus {
            file_id,
            status,
            chunks,
            indexed_at,
            error,
        }
    }))
}

/// Ingest a workspace file into the vector database with background processing
/// This command spawns a background task that will emit progress events via the app handle
#[tauri::command]
pub async fn ingest_workspace_file<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    workspace_id: String,
    file_id: String,
    file_path: String,
) -> Result<(), String> {
    // Clone app_handle and get state first
    let app_clone = app_handle.clone();
    let app_for_state = app_handle.clone();
    let state = app_handle.state::<crate::core::state::AppState>();
    
    // Store the cancellation token for this file
    let cancel_token = CancellationToken::new();
    state
        .workspace_indexing_tokens
        .lock()
        .await
        .insert(file_id.clone(), cancel_token.clone());
    
    // Clone values for the background task
    let _workspace_id_clone = workspace_id.clone();
    let file_id_clone = file_id.clone();
    let _file_path_clone = file_path.clone();
    
    // Spawn background task for processing
    // This will emit events that the frontend listens to
    tokio::spawn(async move {
        // Update status to processing
        let _ = db::workspace_files::update_rag_status(&file_id_clone, "processing", None, None, None).await;
        
        // Emit event that indexing has started
        let _ = app_clone.emit("workspace-file-indexing-start", serde_json::json!({
            "file_id": file_id_clone,
        }));
        
        // The actual processing will be handled by the RAG extension
        // which will emit progress events and update the status
        // For now, just clean up and wait for the extension to complete
        let start_time = std::time::Instant::now();
        let timeout_duration = std::time::Duration::from_secs(300); // 5 minute timeout
        
        // Wait for indexing to complete (extension will update the status)
        // This is a placeholder - the extension will handle the actual work
        loop {
            if cancel_token.is_cancelled() {
                let _ = db::workspace_files::update_rag_status(
                    &file_id_clone,
                    "failed",
                    None,
                    None,
                    Some("Cancelled by user"),
                )
                .await;
                
                let _ = app_clone.emit("workspace-file-indexing-cancelled", serde_json::json!({
                    "file_id": file_id_clone,
                }));
                break;
            }
            
            // Check if status has been updated by the extension
            if let Ok(Some((status, _, _, _))) = db::workspace_files::get_rag_status(&file_id_clone).await {
                if status == "indexed" || status == "failed" {
                    break;
                }
            }
            
            // Check timeout
            if start_time.elapsed() > timeout_duration {
                let _ = db::workspace_files::update_rag_status(
                    &file_id_clone,
                    "failed",
                    None,
                    None,
                    Some("Indexing timeout"),
                )
                .await;
                break;
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
        
        // Clean up cancellation token
        app_for_state
            .state::<crate::core::state::AppState>()
            .workspace_indexing_tokens
            .lock()
            .await
            .remove(&file_id_clone);
    });
    
    Ok(())
}

/// Cancel an in-progress workspace file indexing operation
#[tauri::command]
pub async fn cancel_workspace_file_indexing<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    file_id: String,
) -> Result<(), String> {
    let state = app_handle.state::<crate::core::state::AppState>();
    
    let token = {
        state
            .workspace_indexing_tokens
            .lock()
            .await
            .remove(&file_id)
    };
    
    if let Some(token) = token {
        token.cancel();
        
        // Update file status to failed with cancellation message
        let _ = db::workspace_files::update_rag_status(
            &file_id,
            "failed",
            None,
            None,
            Some("Cancelled by user"),
        )
        .await;
        
        // Emit cancellation event
        let _ = app_handle.emit("workspace-file-indexing-cancelled", serde_json::json!({
            "file_id": file_id,
        }));
        
        Ok(())
    } else {
        Err(format!("No indexing operation found for file {}", file_id))
    }
}


