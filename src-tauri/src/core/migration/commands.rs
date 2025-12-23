/*!
   Migration Commands

   Tauri commands for running data migrations.
*/

use tauri::Runtime;

use super::{MigrationStats, migrate_threads};

/// Get migration status - what needs to be migrated
#[tauri::command]
pub async fn get_migration_status<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
) -> Result<serde_json::Value, String> {
    super::get_migration_status(app_handle).await
}

/// Run migration from file-based storage to database
#[tauri::command]
pub async fn run_migration<R: Runtime>(
    app_handle: tauri::AppHandle<R>,
    dry_run: bool,
) -> Result<MigrationStats, String> {
    log::info!("Starting migration (dry_run: {})", dry_run);
    
    let stats = migrate_threads(app_handle, dry_run).await?;
    
    log::info!(
        "Migration complete. Threads: {}/{}, Messages: {}/{}",
        stats.threads_migrated,
        stats.threads_migrated + stats.threads_failed,
        stats.messages_migrated,
        stats.messages_migrated + stats.messages_failed
    );
    
    Ok(stats)
}
