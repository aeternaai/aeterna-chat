/*!
   Message Repository

   Provides database operations for message management.
*/

use super::connection::get_pool;
use super::error::{DbError, DbResult};
use serde_json::Value;
use sqlx::Row;

/// Create a new message in the database
pub async fn create_message(message: Value) -> DbResult<Value> {
    let pool = get_pool()?;
    
    let id = message.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Message ID is required".to_string()))?;
    
    let thread_id = message.get("thread_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Thread ID is required".to_string()))?;
    
    let role = message.get("role")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Role is required".to_string()))?;
    
    let content = serde_json::to_string(
        message.get("content")
            .ok_or_else(|| DbError::InvalidData("Content is required".to_string()))?
    )?;
    
    let created_at = message.get("created_at")
        .and_then(|v| v.as_i64())
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    
    let status = message.get("status")
        .and_then(|v| v.as_str());
    
    let metadata = serde_json::to_string(&message)?;

    sqlx::query(
        r#"
        INSERT INTO messages (id, thread_id, role, content, created_at, status, metadata)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(id)
    .bind(thread_id)
    .bind(role)
    .bind(&content)
    .bind(created_at)
    .bind(status)
    .bind(&metadata)
    .execute(pool)
    .await?;

    Ok(message)
}

/// Get a message by ID
pub async fn get_message(message_id: &str) -> DbResult<Option<Value>> {
    let pool = get_pool()?;
    
    let row = sqlx::query("SELECT metadata FROM messages WHERE id = ?1")
        .bind(message_id)
        .fetch_optional(pool)
        .await?;
    
    match row {
        Some(row) => {
            let metadata: String = row.get(0);
            let message: Value = serde_json::from_str(&metadata)?;
            Ok(Some(message))
        }
        None => Ok(None),
    }
}

/// List all messages for a thread
pub async fn list_messages(thread_id: &str) -> DbResult<Vec<Value>> {
    let pool = get_pool()?;
    
    let rows = sqlx::query("SELECT metadata FROM messages WHERE thread_id = ?1 ORDER BY created_at ASC")
        .bind(thread_id)
        .fetch_all(pool)
        .await?;
    
    let messages: Result<Vec<Value>, _> = rows
        .iter()
        .map(|row| {
            let metadata: String = row.get(0);
            serde_json::from_str(&metadata)
        })
        .collect();
    
    Ok(messages?)
}

/// Update a message
pub async fn update_message(message: Value) -> DbResult<Value> {
    let pool = get_pool()?;
    
    let id = message.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Message ID is required".to_string()))?;
    
    let role = message.get("role")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DbError::InvalidData("Role is required".to_string()))?;
    
    let content = serde_json::to_string(
        message.get("content")
            .ok_or_else(|| DbError::InvalidData("Content is required".to_string()))?
    )?;
    
    let status = message.get("status")
        .and_then(|v| v.as_str());
    
    let metadata = serde_json::to_string(&message)?;

    let result = sqlx::query(
        r#"
        UPDATE messages
        SET role = ?1, content = ?2, status = ?3, metadata = ?4
        WHERE id = ?5
        "#,
    )
    .bind(role)
    .bind(&content)
    .bind(status)
    .bind(&metadata)
    .bind(id)
    .execute(pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Message {} not found", id)));
    }

    Ok(message)
}

/// Delete a message
pub async fn delete_message(message_id: &str) -> DbResult<()> {
    let pool = get_pool()?;
    
    let result = sqlx::query("DELETE FROM messages WHERE id = ?1")
        .bind(message_id)
        .execute(pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(DbError::NotFound(format!("Message {} not found", message_id)));
    }

    Ok(())
}

/// Delete all messages for a thread
pub async fn delete_thread_messages(thread_id: &str) -> DbResult<u64> {
    let pool = get_pool()?;
    
    let result = sqlx::query("DELETE FROM messages WHERE thread_id = ?1")
        .bind(thread_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// Get message count for a thread
pub async fn count_messages(thread_id: &str) -> DbResult<i64> {
    let pool = get_pool()?;
    
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages WHERE thread_id = ?1")
        .bind(thread_id)
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
    async fn test_message_crud() {
        let _pool = setup_test_db().await;
        
        // Note: These tests require DB_POOL to be initialized
        // In real implementation, you'd use dependency injection
    }
}
