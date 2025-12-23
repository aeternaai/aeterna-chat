/*!
   Thread Folder Commands

   Provides Tauri commands for thread folder/project management operations.
   These commands interface with the database to manage folder hierarchy.
*/

use tauri::Runtime;
use uuid::Uuid;
use serde_json::json;

use crate::core::db;

/// Create a new thread folder
#[tauri::command]
pub async fn create_folder<R: Runtime>(
    _app_handle: tauri::AppHandle<R>,
    name: String,
    workspace_id: Option<String>,
    parent_folder_id: Option<String>,
) -> Result<serde_json::Value, String> {
    // Generate UUID for new folder
    let id = Uuid::new_v4().to_string();
    let now = chrono::Utc::now().timestamp_millis();
    
    let folder = json!({
        "id": id,
        "name": name,
        "workspace_id": workspace_id,
        "parent_folder_id": parent_folder_id,
        "created_at": now,
        "updated_at": now,
    });
    
    db::folders::create_folder(folder)
        .await
        .map_err(|e| format!("Failed to create folder: {}", e))
}

/// Get a folder by ID
#[tauri::command]
pub async fn get_folder<R: Runtime>(
    _app_handle: tauri::AppHandle<R>,
    folder_id: String,
) -> Result<Option<serde_json::Value>, String> {
    db::folders::get_folder(&folder_id)
        .await
        .map_err(|e| format!("Failed to get folder: {}", e))
}

/// List all folders, optionally filtered by workspace
#[tauri::command]
pub async fn list_folders<R: Runtime>(
    _app_handle: tauri::AppHandle<R>,
    workspace_id: Option<String>,
) -> Result<Vec<serde_json::Value>, String> {
    db::folders::list_folders(workspace_id.as_deref())
        .await
        .map_err(|e| format!("Failed to list folders: {}", e))
}

/// Update a folder
#[tauri::command]
pub async fn update_folder<R: Runtime>(
    _app_handle: tauri::AppHandle<R>,
    folder: serde_json::Value,
) -> Result<(), String> {
    // Update timestamp
    let mut folder = folder;
    if let Some(obj) = folder.as_object_mut() {
        obj.insert("updated_at".to_string(), json!(chrono::Utc::now().timestamp_millis()));
    }
    
    db::folders::update_folder(folder)
        .await
        .map_err(|e| format!("Failed to update folder: {}", e))
}

/// Delete a folder and optionally its threads (CASCADE)
#[tauri::command]
pub async fn delete_folder<R: Runtime>(
    _app_handle: tauri::AppHandle<R>,
    folder_id: String,
) -> Result<(), String> {
    db::folders::delete_folder(&folder_id)
        .await
        .map_err(|e| format!("Failed to delete folder: {}", e))
}

/// Count folders, optionally filtered by workspace
#[tauri::command]
pub async fn count_folders<R: Runtime>(
    _app_handle: tauri::AppHandle<R>,
    workspace_id: Option<String>,
) -> Result<i64, String> {
    db::folders::count_folders(workspace_id.as_deref())
        .await
        .map_err(|e| format!("Failed to count folders: {}", e))
}
