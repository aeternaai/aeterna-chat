/*!
   Thread Folder Repository

   Provides database operations for thread folder/project management.
*/

use super::connection::get_pool;
use super::error::{DbError, DbResult};
use serde_json::Value;
use sqlx::Row;

/// Create a new thread folder
pub async fn create_folder(folder: Value) -> DbResult<Value> {
    let pool = get_pool()?;
    
    let id = folder.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Folder ID is required".to_string()))?;
    
    let name = folder.get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Folder name is required".to_string()))?;
    
    let workspace_id = folder.get("workspace_id")
        .and_then(|v| v.as_str());
    
    let parent_folder_id = folder.get("parent_folder_id")
        .and_then(|v| v.as_str());
    
    let created_at = folder.get("created_at")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    
    let updated_at = folder.get("updated_at")
        .and_then(|v| v.as_i64())
        .unwrap_or(created_at);

    sqlx::query(
        r#"
        INSERT INTO thread_folders (id, name, workspace_id, parent_folder_id, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(workspace_id)
    .bind(parent_folder_id)
    .bind(created_at)
    .bind(updated_at)
    .execute(pool)
    .await?;

    Ok(folder)
}

/// Get a folder by ID
pub async fn get_folder(folder_id: &str) -> DbResult<Option<Value>> {
    let pool = get_pool()?;
    
    let row = sqlx::query(
        r#"
        SELECT id, name, workspace_id, parent_folder_id, created_at, updated_at
        FROM thread_folders WHERE id = ?1
        "#
    )
    .bind(folder_id)
    .fetch_optional(pool)
    .await?;
    
    match row {
        Some(row) => {
            let folder = serde_json::json!({
                "id": row.get::<String, _>(0),
                "name": row.get::<String, _>(1),
                "workspace_id": row.get::<Option<String>, _>(2),
                "parent_folder_id": row.get::<Option<String>, _>(3),
                "created_at": row.get::<i64, _>(4),
                "updated_at": row.get::<i64, _>(5),
            });
            
            Ok(Some(folder))
        }
        None => Ok(None),
    }
}

/// List all folders, optionally filtered by workspace
pub async fn list_folders(workspace_id: Option<&str>) -> DbResult<Vec<Value>> {
    let pool = get_pool()?;
    
    let rows = if let Some(wid) = workspace_id {
        sqlx::query(
            r#"
            SELECT id, name, workspace_id, parent_folder_id, created_at, updated_at
            FROM thread_folders WHERE workspace_id = ?1 ORDER BY updated_at DESC
            "#
        )
        .bind(wid)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query(
            r#"
            SELECT id, name, workspace_id, parent_folder_id, created_at, updated_at
            FROM thread_folders ORDER BY updated_at DESC
            "#
        )
        .fetch_all(pool)
        .await?
    };
    
    let folders: Vec<Value> = rows
        .iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<String, _>(0),
                "name": row.get::<String, _>(1),
                "workspace_id": row.get::<Option<String>, _>(2),
                "parent_folder_id": row.get::<Option<String>, _>(3),
                "created_at": row.get::<i64, _>(4),
                "updated_at": row.get::<i64, _>(5),
            })
        })
        .collect();
    
    Ok(folders)
}

/// Update a folder
pub async fn update_folder(folder: Value) -> DbResult<()> {
    let pool = get_pool()?;
    
    let id = folder.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Folder ID is required".to_string()))?;
    
    let name = folder.get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Folder name is required".to_string()))?;
    
    let workspace_id = folder.get("workspace_id")
        .and_then(|v| v.as_str());
    
    let parent_folder_id = folder.get("parent_folder_id")
        .and_then(|v| v.as_str());
    
    let updated_at = chrono::Utc::now().timestamp();

    let result = sqlx::query(
        r#"
        UPDATE thread_folders
        SET name = ?1, workspace_id = ?2, parent_folder_id = ?3, updated_at = ?4
        WHERE id = ?5
        "#,
    )
    .bind(name)
    .bind(workspace_id)
    .bind(parent_folder_id)
    .bind(updated_at)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Folder {} not found", id)));
    }

    Ok(())
}

/// Delete a folder and optionally its threads (CASCADE)
pub async fn delete_folder(folder_id: &str) -> DbResult<()> {
    let pool = get_pool()?;
    
    let result = sqlx::query("DELETE FROM thread_folders WHERE id = ?1")
        .bind(folder_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Folder {} not found", folder_id)));
    }

    Ok(())
}

/// Count folders in a workspace
pub async fn count_folders(workspace_id: Option<&str>) -> DbResult<i64> {
    let pool = get_pool()?;
    
    let count: i64 = if let Some(wid) = workspace_id {
        sqlx::query_scalar("SELECT COUNT(*) FROM thread_folders WHERE workspace_id = ?1")
            .bind(wid)
            .fetch_one(pool)
            .await?
    } else {
        sqlx::query_scalar("SELECT COUNT(*) FROM thread_folders")
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
    async fn test_folder_crud() {
        let _pool = setup_test_db().await;
        
        // Note: These tests require DB_POOL to be initialized
    }
}
