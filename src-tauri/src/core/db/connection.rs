/*!
   Database Connection Management

   Manages the SQLite connection pool for all database operations.
   Provides initialization and access to the global connection pool.
*/

use super::error::{DbError, DbResult};
use crate::core::app::commands::get_jan_data_folder_path;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use std::sync::OnceLock;
use std::time::Duration;
use tauri::{AppHandle, Runtime};

const DB_NAME: &str = "jan.db";
const MAX_CONNECTIONS: u32 = 10;
const CONNECTION_TIMEOUT_SECS: u64 = 5;

/// Global database connection pool
static DB_POOL: OnceLock<SqlitePool> = OnceLock::new();

/// Initialize the database connection pool and run migrations
pub async fn init_database<R: Runtime>(app: &AppHandle<R>) -> DbResult<()> {
    log::info!("Initializing database...");

    // Get Jan data directory
    let data_dir = get_jan_data_folder_path(app.clone());
    
    // Ensure directory exists
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| DbError::IoError(e))?;
    }

    // Create database path
    let db_path = data_dir.join(DB_NAME);
    let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

    log::info!("Database path: {}", db_path.display());

    // Create connection options with optimizations
    let connect_options = SqliteConnectOptions::from_str(&db_url)
        .map_err(|e| DbError::PoolError(format!("Failed to parse connection options: {}", e)))?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal) // Write-Ahead Logging for better concurrency
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal) // Good balance of safety and performance
        .busy_timeout(Duration::from_secs(30)) // Wait up to 30 seconds if database is locked
        .foreign_keys(true); // Enable foreign key constraints

    // Create connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(MAX_CONNECTIONS)
        .acquire_timeout(Duration::from_secs(CONNECTION_TIMEOUT_SECS))
        .connect_with(connect_options)
        .await
        .map_err(|e| DbError::PoolError(format!("Failed to create connection pool: {}", e)))?;

    log::info!("Database connection pool created with {} max connections", MAX_CONNECTIONS);

    // Run migrations
    super::migrations::run_migrations(&pool).await?;

    // Store the pool globally
    DB_POOL
        .set(pool)
        .map_err(|_| DbError::PoolError("Database pool already initialized".to_string()))?;

    log::info!("Database initialized successfully");
    Ok(())
}

/// Get reference to the global database pool
pub fn get_pool() -> DbResult<&'static SqlitePool> {
    DB_POOL
        .get()
        .ok_or(DbError::NotInitialized)
}

/// Check if database is initialized
pub fn is_initialized() -> bool {
    DB_POOL.get().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_not_initialized() {
        // Pool should not be initialized in tests by default
        assert!(!is_initialized());
        assert!(get_pool().is_err());
    }
}
