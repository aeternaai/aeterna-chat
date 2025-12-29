# Database Architecture

## Overview

Jan now uses SQLite as the primary storage backend for threads, messages, workspaces, and folders. This replaces the previous file-based storage system and provides better performance, data integrity, and concurrent access.

## Database Location

**macOS**: `~/Library/Application Support/Jan/data/jan.db`
**Linux**: `~/.config/Jan/data/jan.db`
**Windows**: `%APPDATA%\Jan\data\jan.db`

## Schema

### Tables

#### `workspaces`
Stores workspace metadata and configuration.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PRIMARY KEY | Unique workspace identifier |
| name | TEXT NOT NULL | Workspace name |
| description | TEXT | Optional description |
| visibility | TEXT NOT NULL | 'private' or 'public' |
| created_at | INTEGER NOT NULL | Unix timestamp (seconds) |
| updated_at | INTEGER NOT NULL | Unix timestamp (seconds) |
| owner_id | TEXT | Optional owner identifier |
| metadata | TEXT | JSON-encoded metadata |

#### `threads`
Stores conversation threads.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PRIMARY KEY | Unique thread identifier |
| workspace_id | TEXT | Foreign key to workspaces (nullable) |
| folder_id | TEXT | Foreign key to thread_folders (nullable) |
| title | TEXT | Thread title |
| created_at | INTEGER NOT NULL | Unix timestamp (seconds) |
| updated_at | INTEGER NOT NULL | Unix timestamp (seconds) |
| metadata | TEXT | JSON-encoded thread data |

**Indexes**:
- `idx_threads_workspace` on `workspace_id`
- `idx_threads_folder` on `folder_id`
- `idx_threads_created` on `created_at`

#### `messages`
Stores individual messages within threads.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PRIMARY KEY | Unique message identifier |
| thread_id | TEXT NOT NULL | Foreign key to threads |
| role | TEXT NOT NULL | 'user', 'assistant', 'system' |
| content | TEXT | Message content |
| created_at | INTEGER NOT NULL | Unix timestamp (seconds) |
| status | TEXT | Message status ('ready', 'pending', etc.) |
| metadata | TEXT | JSON-encoded message data |

**Indexes**:
- `idx_messages_thread` on `thread_id`
- `idx_messages_created` on `created_at`

**Cascade**: Deleting a thread automatically deletes all its messages.

#### `thread_folders`
Stores project/folder organization for threads.

| Column | Type | Description |
|--------|------|-------------|
| id | TEXT PRIMARY KEY | Unique folder identifier |
| name | TEXT NOT NULL | Folder/project name |
| workspace_id | TEXT | Foreign key to workspaces (nullable) |
| parent_folder_id | TEXT | For nested folders (nullable) |
| created_at | INTEGER NOT NULL | Unix timestamp (seconds) |
| updated_at | INTEGER NOT NULL | Unix timestamp (seconds) |

**Indexes**:
- `idx_folders_workspace` on `workspace_id`
- `idx_folders_parent` on `parent_folder_id`

#### `workspace_files`
Tracks files uploaded to workspaces (for RAG/context).

| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PRIMARY KEY AUTOINCREMENT | Auto-generated ID |
| workspace_id | TEXT NOT NULL | Foreign key to workspaces |
| filename | TEXT NOT NULL | Original filename |
| filepath | TEXT NOT NULL | Full path to file |
| file_size | INTEGER | File size in bytes |
| uploaded_at | INTEGER NOT NULL | Unix timestamp (seconds) |

**Indexes**:
- `idx_workspace_files_workspace` on `workspace_id`

**Cascade**: Deleting a workspace automatically deletes all its files.

## Migration from File-Based Storage

### Prerequisites

1. **Backup your data** before migration:
   ```bash
   # Backup database
   cp ~/Library/Application\ Support/Jan/data/jan.db ~/Library/Application\ Support/Jan/data/jan.db.backup
   
   # Backup file directories (optional, originals preserved)
   tar -czf jan-files-backup.tar.gz ~/Library/Application\ Support/Jan/data/threads/ ~/Library/Application\ Support/Jan/data/workspaces/
   ```

2. **Install required tools**:
   - `jq` for JSON parsing: `brew install jq` (macOS)
   - `sqlite3` CLI (usually pre-installed)

### Migration Scripts

#### 1. Migrate Threads & Messages
```bash
./migrate_threads.sh
```

**What it does**:
- Scans `~/Library/Application Support/Jan/data/threads/` directories
- Reads each `thread.json` file
- Extracts `folder_id` from `metadata.project.id` if present
- Inserts thread into database
- Reads `messages.jsonl` for each thread
- Inserts all messages into database
- Skips threads already in database (idempotent)

**Output**:
```
=== Thread Migration to Database ===
Found 39 thread directories

✓ Migrated thread: abc123
   → Migrated 15 messages
✓ Migrated thread: def456
   → Migrated 23 messages
   Skipping xyz789 (already in database)

=== Migration Complete ===
Migrated: 37
Skipped: 2
Failed: 0
```

#### 2. Migrate Workspaces
```bash
./migrate_workspaces.sh
```

**What it does**:
- Scans `~/Library/Application Support/Jan/data/workspaces/` directories
- Reads each `workspace.json` file
- Defaults `visibility` to `'private'` if not present
- Inserts workspace into database
- Auto-discovers workspace files (PDF, TXT, MD) and tracks them
- Skips workspaces already in database

#### 3. Migrate Projects/Folders
```bash
# First, export localStorage data from browser:
# 1. Open Jan app
# 2. Open DevTools (Cmd+Option+I)
# 3. Console tab, run:
#    console.log(JSON.stringify(JSON.parse(localStorage.getItem('thread-management')), null, 2))
# 4. Copy JSON output
# 5. Save to: ~/Downloads/thread-management.json

./migrate_projects.sh
```

**What it does**:
- Reads `~/Downloads/thread-management.json` (localStorage export)
- Parses `state.folders` array
- Converts `updated_at` from milliseconds to seconds
- Inserts folders into `thread_folders` table
- Syncs `folder_id` for existing threads with project metadata
- Shows statistics on thread-folder associations

**Why localStorage export is needed**:
Projects/folders were only stored in browser localStorage, not files on disk. The bash script cannot access browser memory, so manual export is required.

### Verification

Check migration status:
```bash
./test_migration.sh
```

**What it shows**:
- File counts vs database counts
- Threads needing migration
- Message statistics
- Folder associations
- Query performance benchmarks
- Unsynced folder_id warnings

### Rollback

If you need to revert to file-based storage:

1. Stop the Jan app
2. Restore backup:
   ```bash
   cp ~/Library/Application\ Support/Jan/data/jan.db.backup ~/Library/Application\ Support/Jan/data/jan.db
   ```
3. Restart the app

**Note**: File-based code is deprecated. Only use rollback if critical issues occur.

## Database Operations

### Backup

**Automatic WAL checkpoint** (on app close):
```bash
sqlite3 ~/Library/Application\ Support/Jan/data/jan.db "PRAGMA wal_checkpoint(TRUNCATE);"
```

**Manual backup**:
```bash
# While app is running (uses WAL)
cp ~/Library/Application\ Support/Jan/data/jan.db ~/backups/jan-$(date +%Y%m%d).db
cp ~/Library/Application\ Support/Jan/data/jan.db-wal ~/backups/jan-$(date +%Y%m%d).db-wal

# When app is closed
cp ~/Library/Application\ Support/Jan/data/jan.db ~/backups/jan-$(date +%Y%m%d).db
```

### Manual Queries

**Count records**:
```bash
sqlite3 ~/Library/Application\ Support/Jan/data/jan.db <<EOF
SELECT 'Threads: ' || COUNT(*) FROM threads;
SELECT 'Messages: ' || COUNT(*) FROM messages;
SELECT 'Workspaces: ' || COUNT(*) FROM workspaces;
SELECT 'Folders: ' || COUNT(*) FROM thread_folders;
EOF
```

**List recent threads**:
```bash
sqlite3 ~/Library/Application\ Support/Jan/data/jan.db \
  "SELECT id, title, datetime(created_at, 'unixepoch') as created FROM threads ORDER BY created_at DESC LIMIT 10;"
```

**Find threads in a project**:
```bash
sqlite3 ~/Library/Application\ Support/Jan/data/jan.db \
  "SELECT t.id, t.title, f.name as project FROM threads t JOIN thread_folders f ON t.folder_id = f.id WHERE f.id = 'PROJECT_ID';"
```

**Message count by thread**:
```bash
sqlite3 ~/Library/Application\ Support/Jan/data/jan.db \
  "SELECT t.title, COUNT(m.id) as msg_count FROM threads t LEFT JOIN messages m ON t.id = m.thread_id GROUP BY t.id ORDER BY msg_count DESC LIMIT 10;"
```

### Performance Tuning

**Check database size**:
```bash
ls -lh ~/Library/Application\ Support/Jan/data/jan.db*
sqlite3 ~/Library/Application\ Support/Jan/data/jan.db "PRAGMA page_count; PRAGMA page_size;"
```

**Optimize (periodic maintenance)**:
```bash
sqlite3 ~/Library/Application\ Support/Jan/data/jan.db "VACUUM; ANALYZE;"
```

**Check integrity**:
```bash
sqlite3 ~/Library/Application\ Support/Jan/data/jan.db "PRAGMA integrity_check;"
```

## Architecture Details

### Connection Pooling

- **Pool size**: 10 connections (configurable in `db/connection.rs`)
- **WAL mode**: Write-Ahead Logging for better concurrency
- **Lazy initialization**: Database created on first access
- **Thread-safe**: Uses `OnceLock` for global pool

### Repository Pattern

Each data type has a dedicated repository:
- `db::threads` - Thread CRUD operations
- `db::messages` - Message CRUD operations
- `db::workspaces` - Workspace CRUD operations
- `db::folders` - Folder CRUD operations
- `db::workspace_files` - File tracking operations

### Tauri Commands

All database operations are exposed via Tauri commands:
- `list_threads`, `create_thread`, `modify_thread`, `delete_thread`
- `list_messages`, `create_message`, `modify_message`, `delete_message`
- `list_workspaces`, `create_workspace`, `modify_workspace`, `delete_workspace`
- `list_folders`, `create_folder`, `modify_folder`, `delete_folder`
- And more... (see `src-tauri/src/lib.rs` for complete list)

### Frontend Integration

**Desktop (Tauri)**:
- Uses `TauriProjectsService`, `TauriThreadsService`, etc.
- Calls Tauri commands via `invoke()` from `@tauri-apps/api/core`

**Web/Mobile**:
- Uses `DefaultProjectsService` with localStorage
- May use hybrid approach (some data in localStorage, some in IndexedDB)

## Troubleshooting

### "Database is locked"
- Ensure only one Jan instance is running
- Check for zombie processes: `ps aux | grep -i jan`
- Wait for ongoing operations to complete

### Migration script fails
- Check file permissions: `ls -la ~/Library/Application\ Support/Jan/data/`
- Verify jq is installed: `which jq`
- Check logs in terminal output for specific errors

### Missing threads after migration
- Run `./test_migration.sh` to compare counts
- Check `metadata.project.id` is being extracted: 
  ```bash
  sqlite3 ~/Library/Application\ Support/Jan/data/jan.db \
    "SELECT id, json_extract(metadata, '$.metadata.project.id') as project_id FROM threads WHERE folder_id IS NULL LIMIT 5;"
  ```

### Performance issues
- Check database size (should be < 100MB for typical usage)
- Run `VACUUM` and `ANALYZE` (see Performance Tuning above)
- Check WAL file size: `ls -lh ~/Library/Application\ Support/Jan/data/jan.db-wal`
- If WAL is large (>10MB), checkpoint: `PRAGMA wal_checkpoint(TRUNCATE);`

## Future Improvements

- [ ] Automatic migration on first run
- [ ] UI for migration status and progress
- [ ] Incremental backups
- [ ] Database encryption
- [ ] Cloud sync support
- [ ] Multi-workspace support
- [ ] Full-text search on messages
- [ ] Analytics and usage statistics

## Contributing

When adding new data models:

1. Add table to `src-tauri/src/core/db/migrations.rs`
2. Create repository in `src-tauri/src/core/db/`
3. Add Tauri commands in `src-tauri/src/core/*/commands.rs`
4. Register commands in `src-tauri/src/lib.rs`
5. Create service in `web-app/src/services/`
6. Update this documentation

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed guidelines.
