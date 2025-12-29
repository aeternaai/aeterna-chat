// NOTE: This file contains legacy file-based operations that are no longer used.
// All thread and message operations now use the database (see db/threads.rs, db/messages.rs).
// These functions are kept only for reference during migration but should not be used in new code.
//
// To migrate data from files to database, use the migration scripts:
// - ./migrate_threads.sh
// - ./test_migration.sh
//
// TODO: Remove this file once migration is complete and no dependencies remain.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use tauri::Runtime;

use super::utils::{get_messages_path, get_thread_metadata_path};

/// DEPRECATED: All platforms now use database exclusively
/// This function is kept only for compatibility during migration
#[deprecated(note = "All platforms now use database. This function should not be used.")]
pub fn should_use_sqlite() -> bool {
    // Return true to indicate database is used everywhere
    true
}

/// DEPRECATED: Database handles concurrency automatically
/// This function is kept only for compatibility during migration
#[deprecated(note = "Database operations don't need per-thread locks")]
pub async fn get_lock_for_thread(_thread_id: &str) -> () {
    // No-op: Database handles locking
}

/// DEPRECATED: Use db::messages::create_message instead
/// Write messages to a thread's messages.jsonl file
#[deprecated(note = "Use db::messages::create_message or batch operations")]
pub fn write_messages_to_file(
    messages: &[serde_json::Value],
    path: &std::path::Path,
) -> Result<(), String> {
    let mut file = File::create(path).map_err(|e| e.to_string())?;
    for msg in messages {
        let data = serde_json::to_string(msg).map_err(|e| e.to_string())?;
        writeln!(file, "{data}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// DEPRECATED: Use db::messages::list_messages instead
/// Read messages from a thread's messages.jsonl file
#[deprecated(note = "Use db::messages::list_messages instead")]
pub fn read_messages_from_file<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    thread_id: &str,
) -> Result<Vec<serde_json::Value>, String> {
    let path = get_messages_path(app_handle, thread_id);
    if !path.exists() {
        return Ok(vec![]);
    }

    let file = File::open(&path).map_err(|e| {
        eprintln!("Error opening file {}: {}", path.display(), e);
        e.to_string()
    })?;
    let reader = BufReader::new(file);

    let mut messages = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| {
            eprintln!("Error reading line from file {}: {}", path.display(), e);
            e.to_string()
        })?;
        let message: serde_json::Value = serde_json::from_str(&line).map_err(|e| {
            eprintln!(
                "Error parsing JSON from line in file {}: {}",
                path.display(),
                e
            );
            e.to_string()
        })?;
        messages.push(message);
    }

    Ok(messages)
}

/// DEPRECATED: Use db::threads::update_thread instead
/// Update thread metadata by writing to thread.json
#[deprecated(note = "Use db::threads::update_thread instead")]
pub fn update_thread_metadata<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    thread_id: &str,
    thread: &serde_json::Value,
) -> Result<(), String> {
    let path = get_thread_metadata_path(app_handle, thread_id);
    let data = serde_json::to_string_pretty(thread).map_err(|e| e.to_string())?;
    fs::write(path, data).map_err(|e| e.to_string())?;
    Ok(())
}
