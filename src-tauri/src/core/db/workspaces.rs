/*!
   Workspace Repository

   Provides database operations for workspace management.
*/

use super::connection::get_pool;
use super::error::{DbError, DbResult};
use serde_json::Value;
use sqlx::Row;

/// Create a new workspace in the database
pub async fn create_workspace(workspace: Value) -> DbResult<Value> {
    let pool = get_pool()?;
    
    let id = workspace.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Workspace ID is required".to_string()))?;
    
    let name = workspace.get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Workspace name is required".to_string()))?;
    
    let description = workspace.get("description")
        .and_then(|v| v.as_str());
    
    let visibility = workspace.get("visibility")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Workspace visibility is required".to_string()))?;
    
    let created_at = workspace.get("created_at")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    
    let updated_at = workspace.get("updated_at")
        .and_then(|v| v.as_i64())
        .unwrap_or(created_at);
    
    let owner_id = workspace.get("owner_id")
        .and_then(|v| v.as_str());
    
    let metadata = workspace.get("metadata")
        .map(|v| serde_json::to_string(v))
        .transpose()?;

    sqlx::query(
        r#"
        INSERT INTO workspaces (id, name, description, visibility, created_at, updated_at, owner_id, metadata)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(description)
    .bind(visibility)
    .bind(created_at)
    .bind(updated_at)
    .bind(owner_id)
    .bind(metadata)
    .execute(pool)
    .await?;

    Ok(workspace)
}

/// Get a workspace by ID
pub async fn get_workspace(workspace_id: &str) -> DbResult<Option<Value>> {
    let pool = get_pool()?;
    
    let row = sqlx::query(
        r#"
        SELECT id, name, description, visibility, created_at, updated_at, owner_id, metadata
        FROM workspaces WHERE id = ?1
        "#
    )
    .bind(workspace_id)
    .fetch_optional(pool)
    .await?;
    
    match row {
        Some(row) => {
            let metadata_str: Option<String> = row.get(7);
            let metadata: Option<Value> = metadata_str
                .and_then(|s| serde_json::from_str(&s).ok());
            
            let workspace = serde_json::json!({
                "id": row.get::<String, _>(0),
                "name": row.get::<String, _>(1),
                "description": row.get::<Option<String>, _>(2),
                "visibility": row.get::<String, _>(3),
                "created_at": row.get::<i64, _>(4),
                "updated_at": row.get::<i64, _>(5),
                "owner_id": row.get::<Option<String>, _>(6),
                "metadata": metadata,
            });
            
            Ok(Some(workspace))
        }
        None => Ok(None),
    }
}

/// List all workspaces
pub async fn list_workspaces() -> DbResult<Vec<Value>> {
    let pool = get_pool()?;
    
    let rows = sqlx::query(
        r#"
        SELECT id, name, description, visibility, created_at, updated_at, owner_id, metadata
        FROM workspaces ORDER BY updated_at DESC
        "#
    )
    .fetch_all(pool)
    .await?;
    
    let workspaces: Result<Vec<Value>, DbError> = rows
        .iter()
        .map(|row| {
            let metadata_str: Option<String> = row.get(7);
            let metadata: Option<Value> = metadata_str
                .and_then(|s| serde_json::from_str(&s).ok());
            
            Ok(serde_json::json!({
                "id": row.get::<String, _>(0),
                "name": row.get::<String, _>(1),
                "description": row.get::<Option<String>, _>(2),
                "visibility": row.get::<String, _>(3),
                "created_at": row.get::<i64, _>(4),
                "updated_at": row.get::<i64, _>(5),
                "owner_id": row.get::<Option<String>, _>(6),
                "metadata": metadata,
            }))
        })
        .collect();
    
    Ok(workspaces?)
}

/// Update a workspace
pub async fn update_workspace(workspace: Value) -> DbResult<()> {
    let pool = get_pool()?;
    
    let id = workspace.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Workspace ID is required".to_string()))?;
    
    let name = workspace.get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Workspace name is required".to_string()))?;
    
    let description = workspace.get("description")
        .and_then(|v| v.as_str());
    
    let visibility = workspace.get("visibility")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Workspace visibility is required".to_string()))?;
    
    let updated_at = chrono::Utc::now().timestamp();
    
    let owner_id = workspace.get("owner_id")
        .and_then(|v| v.as_str());
    
    let metadata = workspace.get("metadata")
        .map(|v| serde_json::to_string(v))
        .transpose()?;

    let result = sqlx::query(
        r#"
        UPDATE workspaces
        SET name = ?1, description = ?2, visibility = ?3, updated_at = ?4, owner_id = ?5, metadata = ?6
        WHERE id = ?7
        "#,
    )
    .bind(name)
    .bind(description)
    .bind(visibility)
    .bind(updated_at)
    .bind(owner_id)
    .bind(metadata)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Workspace {} not found", id)));
    }

    Ok(())
}

/// Delete a workspace and all its related data (CASCADE)
pub async fn delete_workspace(workspace_id: &str) -> DbResult<()> {
    let pool = get_pool()?;
    
    let result = sqlx::query("DELETE FROM workspaces WHERE id = ?1")
        .bind(workspace_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Workspace {} not found", workspace_id)));
    }

    Ok(())
}

/// Get workspace count
pub async fn count_workspaces() -> DbResult<i64> {
    let pool = get_pool()?;
    
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces")
        .fetch_one(pool)
        .await?;
    
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
    async fn test_workspace_crud() {
        let _pool = setup_test_db().await;
        
        // Note: These tests require DB_POOL to be initialized
    }
}
