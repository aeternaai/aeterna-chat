/*!
   Database Error Types

   Defines error types for database operations with proper error handling and conversion.
*/

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database not initialized")]
    NotInitialized,

    #[error("Database error: {0}")]
    SqlxError(#[from] sqlx::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("Connection pool error: {0}")]
    PoolError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

impl From<DbError> for String {
    fn from(err: DbError) -> Self {
        err.to_string()
    }
}

pub type DbResult<T> = Result<T, DbError>;
