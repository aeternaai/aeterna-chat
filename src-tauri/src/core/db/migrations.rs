/*!
   Database Migrations

   Handles database schema versioning and migrations using sqlx migrate.
*/

use super::error::{DbError, DbResult};
use sqlx::SqlitePool;

/// Run all pending migrations
pub async fn run_migrations(pool: &SqlitePool) -> DbResult<()> {
    log::info!("Running database migrations...");

    // Create migrations table if it doesn't exist
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS _sqlx_migrations (
            version BIGINT PRIMARY KEY,
            description TEXT NOT NULL,
            installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            success BOOLEAN NOT NULL,
            checksum BLOB NOT NULL,
            execution_time BIGINT NOT NULL
        );
        "#,
    )
    .execute(pool)
    .await
    .map_err(|e| DbError::MigrationError(format!("Failed to create migrations table: {}", e)))?;

    // Run migration 001: Initial schema
    run_migration_001(pool).await?;
    
    // Run migration 002: Add RAG columns to workspace_files
    run_migration_002(pool).await?;

    log::info!("All migrations completed successfully");
    Ok(())
}

/// Migration 001: Initial schema with workspaces, threads, messages, folders, and files
async fn run_migration_001(pool: &SqlitePool) -> DbResult<()> {
    // Check if migration already applied
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = 1)"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if exists {
        log::debug!("Migration 001 already applied, skipping");
        return Ok(());
    }

    log::info!("Applying migration 001: Initial schema");

    let start = std::time::Instant::now();
    
    // Begin transaction
    let mut tx = pool.begin().await?;

    // Create workspaces table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS workspaces (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            description TEXT,
            visibility TEXT NOT NULL CHECK(visibility IN ('private', 'public')),
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            owner_id TEXT,
            metadata TEXT
        );
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Create thread_folders table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS thread_folders (
            id TEXT PRIMARY KEY NOT NULL,
            name TEXT NOT NULL,
            workspace_id TEXT,
            parent_folder_id TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
            FOREIGN KEY (parent_folder_id) REFERENCES thread_folders(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Create threads table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS threads (
            id TEXT PRIMARY KEY NOT NULL,
            workspace_id TEXT,
            folder_id TEXT,
            title TEXT,
            object TEXT NOT NULL DEFAULT 'thread',
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            metadata TEXT,
            FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE SET NULL,
            FOREIGN KEY (folder_id) REFERENCES thread_folders(id) ON DELETE SET NULL
        );
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Create messages table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY NOT NULL,
            thread_id TEXT NOT NULL,
            role TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
            content TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            status TEXT,
            metadata TEXT,
            FOREIGN KEY (thread_id) REFERENCES threads(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Create workspace_files table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS workspace_files (
            id TEXT PRIMARY KEY NOT NULL,
            workspace_id TEXT NOT NULL,
            file_path TEXT NOT NULL,
            name TEXT NOT NULL,
            extension TEXT,
            size INTEGER,
            mime_type TEXT,
            added_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            is_valid INTEGER NOT NULL DEFAULT 1,
            status TEXT,
            checksum TEXT,
            metadata TEXT,
            FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Create indexes for performance
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_threads_workspace ON threads(workspace_id);")
        .execute(&mut *tx)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_threads_folder ON threads(folder_id);")
        .execute(&mut *tx)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_threads_updated ON threads(updated_at DESC);")
        .execute(&mut *tx)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_id);")
        .execute(&mut *tx)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);")
        .execute(&mut *tx)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_workspace_files_workspace ON workspace_files(workspace_id);")
        .execute(&mut *tx)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_workspace_files_path ON workspace_files(file_path);")
        .execute(&mut *tx)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_thread_folders_workspace ON thread_folders(workspace_id);")
        .execute(&mut *tx)
        .await?;

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_thread_folders_parent ON thread_folders(parent_folder_id);")
        .execute(&mut *tx)
        .await?;

    // Record migration
    let duration = start.elapsed().as_millis() as i64;
    sqlx::query(
        r#"
        INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
        VALUES (1, 'Initial schema', 1, X'00', ?1);
        "#,
    )
    .bind(duration)
    .execute(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    log::info!("Migration 001 completed in {}ms", duration);
    Ok(())
}

/// Migration 002: Add RAG columns to workspace_files
async fn run_migration_002(pool: &SqlitePool) -> DbResult<()> {
    // Check if migration already applied
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sqlx_migrations WHERE version = 2)"
    )
    .fetch_one(pool)
    .await
    .unwrap_or(false);

    if exists {
        log::debug!("Migration 002 already applied, skipping");
        return Ok(());
    }

    log::info!("Running migration 002: Add RAG columns to workspace_files");
    let start = std::time::Instant::now();

    let mut tx = pool.begin().await?;

    // Add RAG tracking columns to workspace_files
    sqlx::query(
        r#"
        ALTER TABLE workspace_files ADD COLUMN rag_status TEXT DEFAULT 'pending';
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        ALTER TABLE workspace_files ADD COLUMN rag_chunks INTEGER DEFAULT 0;
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        ALTER TABLE workspace_files ADD COLUMN rag_indexed_at INTEGER;
        "#,
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        ALTER TABLE workspace_files ADD COLUMN rag_error TEXT;
        "#,
    )
    .execute(&mut *tx)
    .await?;

    // Record migration
    let duration = start.elapsed().as_millis() as i64;
    sqlx::query(
        r#"
        INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
        VALUES (2, 'Add RAG columns to workspace_files', 1, X'00', ?1);
        "#,
    )
    .bind(duration)
    .execute(&mut *tx)
    .await?;

    // Commit transaction
    tx.commit().await?;

    log::info!("Migration 002 completed in {}ms", duration);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;

    #[tokio::test]
    async fn test_migrations() {
        // Create in-memory database for testing
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // Run migrations
        run_migrations(&pool).await.expect("Migrations failed");

        // Verify tables exist
        let tables: Vec<String> = sqlx::query_scalar(
            "SELECT name FROM sqlite_master WHERE type='table' ORDER BY name"
        )
        .fetch_all(&pool)
        .await
        .expect("Failed to query tables");

        assert!(tables.contains(&"_sqlx_migrations".to_string()));
        assert!(tables.contains(&"workspaces".to_string()));
        assert!(tables.contains(&"threads".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"thread_folders".to_string()));
        assert!(tables.contains(&"workspace_files".to_string()));
    }

    #[tokio::test]
    async fn test_migration_idempotency() {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");

        // Run migrations twice
        run_migrations(&pool).await.expect("First migration failed");
        run_migrations(&pool).await.expect("Second migration failed");

        // Should only have one migration record
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM _sqlx_migrations WHERE version = 1"
        )
        .fetch_one(&pool)
        .await
        .expect("Failed to count migrations");

        assert_eq!(count, 1);
    }
}
