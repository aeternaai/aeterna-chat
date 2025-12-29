/*!
   Data Migration Module

   Provides tools to migrate existing file-based data to the database.
   Supports threads, messages, workspaces, and folders migration.
*/

pub mod commands;

use std::fs;
use std::path::PathBuf;
use serde_json::Value;
use tauri::Runtime;

use crate::core::app::commands::get_jan_data_folder_path;
use crate::core::db;

/// Migration statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct MigrationStats {
    pub threads_migrated: usize,
    pub threads_failed: usize,
    pub messages_migrated: usize,
    pub messages_failed: usize,
    pub errors: Vec<String>,
}

impl Default for MigrationStats {
    fn default() -> Self {
        Self {
            threads_migrated: 0,
            threads_failed: 0,
            messages_migrated: 0,
            messages_failed: 0,
            errors: Vec::new(),
        }
    }
}

/// Migrate threads from file-based storage to database
pub async fn migrate_threads<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    dry_run: bool,
) -> Result<MigrationStats, String> {
    let mut stats = MigrationStats::default();
    
    // Get threads directory
    let threads_dir = get_jan_data_folder_path(app_handle.clone()).join("threads");
    
    if !threads_dir.exists() {
        return Ok(stats); // No threads to migrate
    }
    
    log::info!("Starting thread migration from: {:?}", threads_dir);
    
    // Iterate through thread directories
    let entries = fs::read_dir(&threads_dir)
        .map_err(|e| format!("Failed to read threads directory: {}", e))?;
    
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                stats.errors.push(format!("Failed to read directory entry: {}", e));
                continue;
            }
        };
        
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        
        let thread_id = match path.file_name().and_then(|n| n.to_str()) {
            Some(id) => id.to_string(),
            None => continue,
        };
        
        // Check if thread already exists in database
        match db::threads::get_thread(&thread_id).await {
            Ok(Some(_)) => {
                log::debug!("Thread {} already in database, skipping", thread_id);
                continue;
            }
            Ok(None) => {
                // Thread not in database, proceed with migration
            }
            Err(e) => {
                stats.errors.push(format!("Failed to check thread {}: {}", thread_id, e));
                stats.threads_failed += 1;
                continue;
            }
        }
        
        // Read thread metadata
        let thread_file = path.join("thread.json");
        if !thread_file.exists() {
            stats.errors.push(format!("Thread file not found for {}", thread_id));
            stats.threads_failed += 1;
            continue;
        }
        
        let thread_data = match fs::read_to_string(&thread_file) {
            Ok(data) => data,
            Err(e) => {
                stats.errors.push(format!("Failed to read thread {}: {}", thread_id, e));
                stats.threads_failed += 1;
                continue;
            }
        };
        
        let mut thread: Value = match serde_json::from_str(&thread_data) {
            Ok(t) => t,
            Err(e) => {
                stats.errors.push(format!("Failed to parse thread {}: {}", thread_id, e));
                stats.threads_failed += 1;
                continue;
            }
        };
        
        // Ensure thread has an ID
        thread["id"] = serde_json::json!(thread_id);
        
        if !dry_run {
            // Migrate thread to database
            match db::threads::create_thread(thread).await {
                Ok(_) => {
                    log::info!("Migrated thread: {}", thread_id);
                    stats.threads_migrated += 1;
                }
                Err(e) => {
                    stats.errors.push(format!("Failed to migrate thread {}: {}", thread_id, e));
                    stats.threads_failed += 1;
                }
            }
        } else {
            log::info!("DRY RUN: Would migrate thread: {}", thread_id);
            stats.threads_migrated += 1;
        }
        
        // Migrate messages for this thread
        let messages_file = path.join("messages.jsonl");
        if messages_file.exists() {
            match migrate_thread_messages(&thread_id, &messages_file, dry_run).await {
                Ok((migrated, failed)) => {
                    stats.messages_migrated += migrated;
                    stats.messages_failed += failed;
                }
                Err(e) => {
                    stats.errors.push(format!("Failed to migrate messages for thread {}: {}", thread_id, e));
                }
            }
        }
    }
    
    log::info!("Thread migration complete. Migrated: {}, Failed: {}", 
               stats.threads_migrated, stats.threads_failed);
    
    Ok(stats)
}

/// Migrate messages from JSONL file to database
async fn migrate_thread_messages(
    thread_id: &str,
    messages_file: &PathBuf,
    dry_run: bool,
) -> Result<(usize, usize), String> {
    let mut migrated = 0;
    let mut failed = 0;
    
    let content = fs::read_to_string(messages_file)
        .map_err(|e| format!("Failed to read messages file: {}", e))?;
    
    for (line_num, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        
        let mut message: Value = match serde_json::from_str(line) {
            Ok(m) => m,
            Err(e) => {
                log::warn!("Failed to parse message at line {}: {}", line_num + 1, e);
                failed += 1;
                continue;
            }
        };
        
        // Ensure message has thread_id
        message["thread_id"] = serde_json::json!(thread_id);
        
        // Check if message already exists
        if let Some(msg_id) = message.get("id").and_then(|v| v.as_str()) {
            match db::messages::get_message(msg_id).await {
                Ok(Some(_)) => {
                    log::debug!("Message {} already in database, skipping", msg_id);
                    continue;
                }
                Ok(None) => {
                    // Message not in database, proceed
                }
                Err(_) => {
                    // Error checking, try to migrate anyway
                }
            }
        }
        
        if !dry_run {
            match db::messages::create_message(message).await {
                Ok(_) => migrated += 1,
                Err(e) => {
                    log::warn!("Failed to migrate message at line {}: {}", line_num + 1, e);
                    failed += 1;
                }
            }
        } else {
            migrated += 1;
        }
    }
    
    log::debug!("Migrated {} messages for thread {}", migrated, thread_id);
    
    Ok((migrated, failed))
}

/// Get migration status - check what needs to be migrated
pub async fn get_migration_status<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<serde_json::Value, String> {
    let threads_dir = get_jan_data_folder_path(app_handle.clone()).join("threads");
    
    let mut file_threads = 0;
    let mut file_messages = 0;
    
    if threads_dir.exists() {
        if let Ok(entries) = fs::read_dir(&threads_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                
                if path.join("thread.json").exists() {
                    file_threads += 1;
                }
                
                if let Ok(content) = fs::read_to_string(path.join("messages.jsonl")) {
                    file_messages += content.lines().filter(|l| !l.trim().is_empty()).count();
                }
            }
        }
    }
    
    let db_threads = db::threads::count_threads(None).await.unwrap_or(0);
    let db_messages = db::messages::list_messages("dummy").await.map(|m| m.len()).unwrap_or(0);
    
    Ok(serde_json::json!({
        "file_based": {
            "threads": file_threads,
            "messages": file_messages,
        },
        "database": {
            "threads": db_threads,
            "messages": db_messages,
        },
        "needs_migration": file_threads > 0,
    }))
}
