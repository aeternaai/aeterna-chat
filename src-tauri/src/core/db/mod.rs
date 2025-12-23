/*!
   Database Module

   Unified database layer for all persistence operations using SQLite.
   Provides a clean abstraction over file-based and database storage.
*/

pub mod connection;
pub mod error;
pub mod folders;
pub mod messages;
pub mod migrations;
pub mod threads;
pub mod workspaces;

// Re-export commonly used types
pub use connection::init_database;
