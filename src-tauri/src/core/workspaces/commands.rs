use crate::core::db;
use serde_json::Value;
use tauri::Runtime;
use uuid::Uuid;

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
    app_handle: tauri::AppHandle<R>,
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
