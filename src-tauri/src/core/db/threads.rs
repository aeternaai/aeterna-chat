/*!
   Thread Repository

   Provides database operations for thread management.
*/

use super::connection::get_pool;
use super::error::{DbError, DbResult};
use serde_json::Value;
use sqlx::Row;

/// Create a new thread in the database
pub async fn create_thread(thread: Value) -> DbResult<Value> {
    let pool = get_pool()?;
    
    let id = thread.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Thread ID is required".to_string()))?;
    
    let title = thread.get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    let workspace_id = thread.get("workspace_id")
        .and_then(|v| v.as_str());
    
    let folder_id = thread.get("folder_id")
        .and_then(|v| v.as_str());
    
    let created_at = thread.get("created")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    
    let updated_at = thread.get("updated")
        .and_then(|v| v.as_i64())
        .unwrap_or(created_at);
    
    let metadata = serde_json::to_string(&thread)?;

    sqlx::query(
        r#"
        INSERT INTO threads (id, workspace_id, folder_id, title, created_at, updated_at, metadata)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(id)
    .bind(workspace_id)
    .bind(folder_id)
    .bind(title)
    .bind(created_at)
    .bind(updated_at)
    .bind(&metadata)
    .execute(pool)
    .await?;

    Ok(thread)
}

/// Get a thread by ID
pub async fn get_thread(thread_id: &str) -> DbResult<Option<Value>> {
    let pool = get_pool()?;
    
    let row = sqlx::query("SELECT metadata FROM threads WHERE id = ?1")
        .bind(thread_id)
        .fetch_optional(pool)
        .await?;
    
    match row {
        Some(row) => {
            let metadata: String = row.get(0);
            let thread: Value = serde_json::from_str(&metadata)?;
            Ok(Some(thread))
        }
        None => Ok(None),
    }
}

/// List all threads, optionally filtered by workspace or folder
pub async fn list_threads(
    workspace_id: Option<&str>,
    folder_id: Option<&str>,
) -> DbResult<Vec<Value>> {
    let pool = get_pool()?;
    
    let rows = if let Some(wid) = workspace_id {
        sqlx::query("SELECT metadata FROM threads WHERE workspace_id = ?1 ORDER BY updated_at DESC")
            .bind(wid)
            .fetch_all(pool)
            .await?
    } else if let Some(fid) = folder_id {
        sqlx::query("SELECT metadata FROM threads WHERE folder_id = ?1 ORDER BY updated_at DESC")
            .bind(fid)
            .fetch_all(pool)
            .await?
    } else {
        sqlx::query("SELECT metadata FROM threads ORDER BY updated_at DESC")
            .fetch_all(pool)
            .await?
    };
    
    let threads: Result<Vec<Value>, _> = rows
        .iter()
        .map(|row| {
            let metadata: String = row.get(0);
            serde_json::from_str(&metadata)
        })
        .collect();
    
    Ok(threads?)
}

/// Update a thread
pub async fn update_thread(thread: Value) -> DbResult<()> {
    let pool = get_pool()?;
    
    let id = thread.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Thread ID is required".to_string()))?;
    
    let title = thread.get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    let workspace_id = thread.get("workspace_id")
        .and_then(|v| v.as_str());
    
    let folder_id = thread.get("folder_id")
        .and_then(|v| v.as_str());
    
    let updated_at = chrono::Utc::now().timestamp();
    
    let metadata = serde_json::to_string(&thread)?;

    let result = sqlx::query(
        r#"
        UPDATE threads
        SET workspace_id = ?1, folder_id = ?2, title = ?3, updated_at = ?4, metadata = ?5
        WHERE id = ?6
        "#,
    )
    .bind(workspace_id)
    .bind(folder_id)
    .bind(title)
    .bind(updated_at)
    .bind(&metadata)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Thread {} not found", id)));
    }

    Ok(())
}

/// Delete a thread and all its messages (CASCADE)
pub async fn delete_thread(thread_id: &str) -> DbResult<()> {
    let pool = get_pool()?;
    
    let result = sqlx::query("DELETE FROM threads WHERE id = ?1")
        .bind(thread_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Thread {} not found", thread_id)));
    }

    Ok(())
}

/// Get threads count for a workspace
pub async fn count_threads(workspace_id: Option<&str>) -> DbResult<i64> {
    let pool = get_pool()?;
    
    let count: i64 = if let Some(wid) = workspace_id {
        sqlx::query_scalar("SELECT COUNT(*) FROM threads WHERE workspace_id = ?1")
            .bind(wid)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM threads")
            .fetch_one(pool)
            .await?
    };
    
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::migrations::run_migrations;
    use sqlx::sqlite::SqlitePoolOptions;

    async fn setup_test_db() -> sqlx::SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .expect("Failed to create test database");
        
        run_migrations(&pool).await.expect("Migrations failed");
        pool
    }

    #[tokio::test]
    async fn test_thread_crud() {
        let _pool = setup_test_db().await;
        
        // Note: These tests require DB_POOL to be initialized
        // In real implementation, you'd use dependency injection
        // or test-specific pool initialization
    }
}
