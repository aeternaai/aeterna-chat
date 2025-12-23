#!/bin/bash

# Data Migration Test Script
# Tests the migration from file-based storage to database

DB_PATH=~/"Library/Application Support/Jan/data/jan.db"
THREADS_DIR=~/"Library/Application Support/Jan/data/threads"

echo "=== Phase 5: Data Migration Testing ==="
echo

# Check if migration is needed
echo "1. Checking migration status..."
echo "   File-based threads directory: $THREADS_DIR"

if [ -d "$THREADS_DIR" ]; then
    THREAD_COUNT=$(find "$THREADS_DIR" -type d -maxdepth 1 | tail -n +2 | wc -l | tr -d ' ')
    echo "   Found $THREAD_COUNT thread directories"
    
    # Count threads in database
    DB_THREADS=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM threads;")
    echo "   Database contains $DB_THREADS threads"
    
    if [ "$THREAD_COUNT" -gt "$DB_THREADS" ]; then
        echo "   ⚠️  Migration needed: $((THREAD_COUNT - DB_THREADS)) threads not yet migrated"
    else
        echo "   ✓ All threads migrated"
    fi
else
    echo "   No file-based threads directory found"
fi

echo

# Show current database stats
echo "2. Current database statistics:"
sqlite3 "$DB_PATH" <<'EOF'
SELECT '   Threads: ' || COUNT(*) FROM threads;
SELECT '   Messages: ' || COUNT(*) FROM messages;
SELECT '   Workspaces: ' || COUNT(*) FROM workspaces;
SELECT '   Folders: ' || COUNT(*) FROM thread_folders;
SELECT '   Workspace files: ' || COUNT(*) FROM workspace_files;
EOF

echo

# Test folder_id sync
echo "3. Checking folder_id synchronization:"
UNSYNCED=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM threads WHERE json_extract(metadata, '\$.metadata.project.id') IS NOT NULL AND folder_id IS NULL;")
echo "   Threads with project but no folder_id: $UNSYNCED"

if [ "$UNSYNCED" -gt 0 ]; then
    echo "   Running folder_id sync..."
    sqlite3 "$DB_PATH" <<'EOF'
UPDATE threads 
SET folder_id = json_extract(metadata, '$.metadata.project.id')
WHERE json_extract(metadata, '$.metadata.project.id') IS NOT NULL
  AND folder_id IS NULL;
SELECT '   Synced ' || changes() || ' threads';
EOF
fi

echo

# Show threads by folder
echo "4. Threads by folder:"
sqlite3 "$DB_PATH" <<'EOF'
.mode column
.headers on
SELECT 
  f.name as "Folder",
  COUNT(t.id) as "Thread Count"
FROM thread_folders f
LEFT JOIN threads t ON t.folder_id = f.id
GROUP BY f.id
ORDER BY COUNT(t.id) DESC;
EOF

echo

# Performance test
echo "5. Performance benchmark:"
echo "   Testing thread list query..."
time sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM threads;" > /dev/null

echo "   Testing thread with messages query..."
SAMPLE_THREAD=$(sqlite3 "$DB_PATH" "SELECT id FROM threads LIMIT 1;")
if [ -n "$SAMPLE_THREAD" ]; then
    time sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM messages WHERE thread_id = '$SAMPLE_THREAD';" > /dev/null
fi

echo

echo "6. Migration commands available:"
echo "   From app terminal or Tauri devtools console:"
echo "   - Get status: invoke('get_migration_status')"
echo "   - Dry run: invoke('run_migration', { dryRun: true })"
echo "   - Run migration: invoke('run_migration', { dryRun: false })"

echo
echo "=== Migration Test Complete ===" 
