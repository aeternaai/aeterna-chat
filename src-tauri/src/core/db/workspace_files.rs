/*!
   Workspace Files Repository

   Provides database operations for workspace file management.
*/

use super::connection::get_pool;
use super::error::{DbError, DbResult};
use serde_json::Value;
use sqlx::Row;

/// Add a file to a workspace
pub async fn add_file(workspace_id: &str, file: &Value) -> DbResult<Value> {
    let pool = get_pool()?;
    
    let id = file.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("File ID is required".to_string()))?;
    
    let file_path = file.get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("File path is required".to_string()))?;
    
    let name = file.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    let extension = file.get("extension")
        .and_then(|v| v.as_str());
    
    let size = file.get("size")
        .and_then(|v| v.as_i64());
    
    let added_at = file.get("added_at")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    
    let updated_at = file.get("updated_at")
        .and_then(|v| v.as_i64())
        .unwrap_or(added_at);
    
    let is_valid = file.get("is_valid")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    
    let status = file.get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("ready");
    
    let metadata = serde_json::to_string(&file)?;

    sqlx::query(
        r#"
        INSERT INTO workspace_files 
        (id, workspace_id, file_path, name, extension, size, added_at, updated_at, is_valid, status, metadata)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
    )
    .bind(id)
    .bind(workspace_id)
    .bind(file_path)
    .bind(name)
    .bind(extension)
    .bind(size)
    .bind(added_at)
    .bind(updated_at)
    .bind(is_valid)
    .bind(status)
    .bind(&metadata)
    .execute(pool)
    .await?;

    Ok(file.clone())
}

/// Get a file by ID
pub async fn get_file(file_id: &str) -> DbResult<Option<Value>> {
    let pool = get_pool()?;
    
    let row = sqlx::query(
        r#"
        SELECT metadata
        FROM workspace_files WHERE id = ?1
        "#,
    )
    .bind(file_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => {
            let metadata: String = row.get(0);
            let file: Value = serde_json::from_str(&metadata)?;
            Ok(Some(file))
        }
        None => Ok(None),
    }
}

/// List all files in a workspace
pub async fn list_files(workspace_id: &str) -> DbResult<Vec<Value>> {
    let pool = get_pool()?;
    
    let rows = sqlx::query(
        r#"
        SELECT metadata
        FROM workspace_files
        WHERE workspace_id = ?1
        ORDER BY added_at DESC
        "#,
    )
    .bind(workspace_id)
    .fetch_all(pool)
    .await?;
    
    let files: Result<Vec<Value>, _> = rows
        .iter()
        .map(|row| {
            let metadata: String = row.get(0);
            serde_json::from_str(&metadata)
        })
        .collect();
    
    Ok(files?)
}

/// Update a file
pub async fn update_file(file: &Value) -> DbResult<()> {
    let pool = get_pool()?;
    
    let id = file.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("File ID is required".to_string()))?;
    
    let file_path = file.get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("File path is required".to_string()))?;
    
    let name = file.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    
    let extension = file.get("extension")
        .and_then(|v| v.as_str());
    
    let size = file.get("size")
        .and_then(|v| v.as_i64());
    
    let updated_at = chrono::Utc::now().timestamp();
    
    let is_valid = file.get("is_valid")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    
    let status = file.get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("ready");
    
    let metadata = serde_json::to_string(&file)?;

    let result = sqlx::query(
        r#"
        UPDATE workspace_files
        SET file_path = ?1, name = ?2, extension = ?3, size = ?4, 
            updated_at = ?5, is_valid = ?6, status = ?7, metadata = ?8
        WHERE id = ?9
        "#,
    )
    .bind(file_path)
    .bind(name)
    .bind(extension)
    .bind(size)
    .bind(updated_at)
    .bind(is_valid)
    .bind(status)
    .bind(&metadata)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("File {} not found", id)));
    }

    Ok(())
}

/// Delete a file
pub async fn delete_file(file_id: &str) -> DbResult<()> {
    let pool = get_pool()?;
    
    let result = sqlx::query("DELETE FROM workspace_files WHERE id = ?1")
        .bind(file_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("File {} not found", file_id)));
    }

    Ok(())
}

/// Delete all files in a workspace
pub async fn delete_workspace_files(workspace_id: &str) -> DbResult<u64> {
    let pool = get_pool()?;
    
    let result = sqlx::query("DELETE FROM workspace_files WHERE workspace_id = ?1")
        .bind(workspace_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// Count files in a workspace
pub async fn count_files(workspace_id: &str) -> DbResult<i64> {
    let pool = get_pool()?;
    
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspace_files WHERE workspace_id = ?1")
        .bind(workspace_id)
        .fetch_one(pool)
        .await?;

    Ok(count)
}
