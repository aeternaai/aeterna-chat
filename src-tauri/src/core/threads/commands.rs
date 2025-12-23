use std::fs::{self, File};
use std::io::Write;
use tauri::Runtime;
use uuid::Uuid;

use crate::core::db;
use super::helpers::{
    get_lock_for_thread, read_messages_from_file, should_use_sqlite, update_thread_metadata,
    write_messages_to_file,
};
use super::{
    constants::THREADS_FILE,
    utils::{
        ensure_data_dirs, ensure_thread_dir_exists, get_data_dir, get_messages_path,
        get_thread_dir, get_thread_metadata_path,
    },
};

/// Lists all threads by reading their metadata from the database.
/// Returns a vector of thread metadata as JSON values.
#[tauri::command]
pub async fn list_threads<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<Vec<serde_json::Value>, String> {
    // Use database for all platforms
    db::threads::list_threads(None, None).await
        .map_err(|e| e.to_string())
}

/// Creates a new thread, assigns it a unique ID, and persists its metadata to the database.
#[tauri::command]
pub async fn create_thread<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    mut thread: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Use database for all platforms
    let uuid = Uuid::new_v4().to_string();
    thread["id"] = serde_json::Value::String(uuid.clone());
    
    db::threads::create_thread(thread).await
        .map_err(|e| e.to_string())
}

/// Modifies an existing thread's metadata in the database.
/// Returns an error if the thread does not exist.
#[tauri::command]
pub async fn modify_thread<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    thread: serde_json::Value,
) -> Result<(), String> {
    // Use database for all platforms
    db::threads::update_thread(thread).await
        .map_err(|e| e.to_string())
}

/// Deletes a thread and all its associated messages from the database.
/// CASCADE deletion ensures all messages are automatically removed.
#[tauri::command]
pub async fn delete_thread<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    thread_id: String,
) -> Result<(), String> {
    // Use database for all platforms (CASCADE handles message deletion)
    db::threads::delete_thread(&thread_id).await
        .map_err(|e| e.to_string())
}

/// Lists all messages for a given thread from the database.
/// Returns a vector of message JSON values.
#[tauri::command]
pub async fn list_messages<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    thread_id: String,
) -> Result<Vec<serde_json::Value>, String> {
    // Use database for all platforms
    db::messages::list_messages(&thread_id).await
        .map_err(|e| e.to_string())
}

/// Creates a new message in a thread in the database.
/// Database handles concurrency, no per-thread locks needed.
#[tauri::command]
pub async fn create_message<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    mut message: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Use database for all platforms
    if message.get("id").is_none() {
        let uuid = Uuid::new_v4().to_string();
        message["id"] = serde_json::Value::String(uuid);
    }
    
    db::messages::create_message(message).await
        .map_err(|e| e.to_string())
}

/// Modifies an existing message in the database.
#[tauri::command]
pub async fn modify_message<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    message: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Use database for all platforms
    db::messages::update_message(message).await
        .map_err(|e| e.to_string())
}

/// Deletes a message from the database.
#[tauri::command]
pub async fn delete_message<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    thread_id: String,
    message_id: String,
) -> Result<(), String> {
    // Use database for all platforms
    db::messages::delete_message(&message_id).await
        .map_err(|e| e.to_string())
}

/// Retrieves the first assistant associated with a thread from the database.
/// Returns an error if the thread or assistant is not found.
#[tauri::command]
pub async fn get_thread_assistant<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    thread_id: String,
) -> Result<serde_json::Value, String> {
    // Use database for all platforms
    let thread = db::threads::get_thread(&thread_id).await
        .map_err(|e| e.to_string())?
        .ok_or("Thread not found")?;
    
    if let Some(assistants) = thread.get("assistants").and_then(|a| a.as_array()) {
        if let Some(first) = assistants.first() {
            Ok(first.clone())
        } else {
            Err("Assistant not found".to_string())
        }
    } else {
        Err("Assistant not found".to_string())
    }
}

/// Adds a new assistant to a thread's metadata in the database.
#[tauri::command]
pub async fn create_thread_assistant<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    thread_id: String,
    assistant: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Use database for all platforms
    let mut thread = db::threads::get_thread(&thread_id).await
        .map_err(|e| e.to_string())?
        .ok_or("Thread not found")?;
    
    if let Some(assistants) = thread.get_mut("assistants").and_then(|a| a.as_array_mut()) {
        assistants.push(assistant.clone());
    } else {
        thread["assistants"] = serde_json::Value::Array(vec![assistant.clone()]);
    }
    
    db::threads::update_thread(thread).await
        .map_err(|e| e.to_string())?;
    Ok(assistant)
}

/// Modifies an existing assistant's information in a thread's metadata in the database.
#[tauri::command]
pub async fn modify_thread_assistant<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    thread_id: String,
    assistant: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Use database for all platforms
    let mut thread = db::threads::get_thread(&thread_id).await
        .map_err(|e| e.to_string())?
        .ok_or("Thread not found")?;
    
    let assistant_id = assistant
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing id")?;
    
    if let Some(assistants) = thread
        .get_mut("assistants")
        .and_then(|a: &mut serde_json::Value| a.as_array_mut())
    {
        if let Some(index) = assistants
            .iter()
            .position(|a| a.get("id").and_then(|v| v.as_str()) == Some(assistant_id))
        {
            assistants[index] = assistant.clone();
            db::threads::update_thread(thread).await
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(assistant)
}
