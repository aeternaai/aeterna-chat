#!/bin/bash

# Simple Thread Migration Script
# Migrates threads and messages from file-based storage to SQLite database

set -e

DB_PATH=~/"Library/Application Support/Jan/data/jan.db"
THREADS_DIR=~/"Library/Application Support/Jan/data/threads"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "=== Thread Migration to Database ==="
echo

# Check if threads directory exists
if [ ! -d "$THREADS_DIR" ]; then
    echo -e "${RED}Error: Threads directory not found: $THREADS_DIR${NC}"
    exit 1
fi

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo -e "${RED}Error: Database not found: $DB_PATH${NC}"
    exit 1
fi

# Count threads
TOTAL_THREADS=$(find "$THREADS_DIR" -type d -maxdepth 1 | tail -n +2 | wc -l | tr -d ' ')
echo "Found $TOTAL_THREADS thread directories"

MIGRATED=0
SKIPPED=0
FAILED=0

# Migrate each thread
for thread_dir in "$THREADS_DIR"/*; do
    if [ ! -d "$thread_dir" ]; then
        continue
    fi
    
    thread_id=$(basename "$thread_dir")
    thread_file="$thread_dir/thread.json"
    
    if [ ! -f "$thread_file" ]; then
        echo -e "${YELLOW}⚠️  No thread.json for $thread_id${NC}"
        ((FAILED++))
        continue
    fi
    
    # Check if thread already exists in database
    exists=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM threads WHERE id = '$thread_id';")
    
    if [ "$exists" -gt 0 ]; then
        echo "   Skipping $thread_id (already in database)"
        ((SKIPPED++))
        continue
    fi
    
    # Read thread JSON
    thread_json=$(cat "$thread_file")
    
    # Extract fields using jq (or fallback to python)
    if command -v jq &> /dev/null; then
        title=$(echo "$thread_json" | jq -r '.title // ""')
        created=$(echo "$thread_json" | jq -r '.created // 0')
        updated=$(echo "$thread_json" | jq -r '.updated // 0')
        folder_id=$(echo "$thread_json" | jq -r '.metadata.project.id // null' | sed 's/null//')
    else
        # Fallback to python
        title=$(echo "$thread_json" | python3 -c "import sys, json; print(json.load(sys.stdin).get('title', ''))")
        created=$(echo "$thread_json" | python3 -c "import sys, json; print(json.load(sys.stdin).get('created', 0))")
        updated=$(echo "$thread_json" | python3 -c "import sys, json; print(json.load(sys.stdin).get('updated', 0))")
        folder_id=$(echo "$thread_json" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('metadata', {}).get('project', {}).get('id', ''))")
    fi
    
    # Escape single quotes in JSON for SQL
    thread_json_escaped=$(echo "$thread_json" | sed "s/'/''/g")
    
    # Insert thread into database
    if [ -z "$folder_id" ]; then
        folder_id_sql="NULL"
    else
        folder_id_sql="'$folder_id'"
    fi
    
    sqlite3 "$DB_PATH" <<EOF
INSERT INTO threads (id, workspace_id, folder_id, title, created_at, updated_at, metadata)
VALUES ('$thread_id', NULL, $folder_id_sql, '$title', $created, $updated, '$thread_json_escaped');
EOF
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} Migrated thread: $thread_id"
        ((MIGRATED++))
        
        # Migrate messages if they exist
        messages_file="$thread_dir/messages.jsonl"
        if [ -f "$messages_file" ]; then
            msg_count=0
            msg_failed=0
            
            while IFS= read -r message_line; do
                if [ -z "$message_line" ]; then
                    continue
                fi
                
                # Extract message ID
                if command -v jq &> /dev/null; then
                    msg_id=$(echo "$message_line" | jq -r '.id // ""')
                    role=$(echo "$message_line" | jq -r '.role // "user"')
                    created_at=$(echo "$message_line" | jq -r '.created_at // 0')
                    status=$(echo "$message_line" | jq -r '.status // "ready"')
                else
                    msg_id=$(echo "$message_line" | python3 -c "import sys, json; print(json.load(sys.stdin).get('id', ''))")
                    role=$(echo "$message_line" | python3 -c "import sys, json; print(json.load(sys.stdin).get('role', 'user'))")
                    created_at=$(echo "$message_line" | python3 -c "import sys, json; print(json.load(sys.stdin).get('created_at', 0))")
                    status=$(echo "$message_line" | python3 -c "import sys, json; print(json.load(sys.stdin).get('status', 'ready'))")
                fi
                
                if [ -z "$msg_id" ]; then
                    ((msg_failed++))
                    continue
                fi
                
                # Escape single quotes
                message_escaped=$(echo "$message_line" | sed "s/'/''/g")
                
                # Insert message
                sqlite3 "$DB_PATH" <<MSGEOF
INSERT OR IGNORE INTO messages (id, thread_id, role, content, created_at, status, metadata)
VALUES ('$msg_id', '$thread_id', '$role', '', $created_at, '$status', '$message_escaped');
MSGEOF
                
                if [ $? -eq 0 ]; then
                    ((msg_count++))
                else
                    ((msg_failed++))
                fi
            done < "$messages_file"
            
            if [ $msg_count -gt 0 ]; then
                echo "      → Migrated $msg_count messages"
            fi
            if [ $msg_failed -gt 0 ]; then
                echo -e "      ${YELLOW}→ Failed $msg_failed messages${NC}"
            fi
        fi
    else
        echo -e "${RED}✗${NC} Failed to migrate: $thread_id"
        ((FAILED++))
    fi
done

echo
echo "=== Migration Complete ==="
echo -e "${GREEN}Migrated: $MIGRATED${NC}"
echo "Skipped: $SKIPPED"
if [ $FAILED -gt 0 ]; then
    echo -e "${RED}Failed: $FAILED${NC}"
else
    echo "Failed: $FAILED"
fi

# Show updated stats
echo
echo "Database statistics:"
sqlite3 "$DB_PATH" <<'EOF'
SELECT '  Threads: ' || COUNT(*) FROM threads;
SELECT '  Messages: ' || COUNT(*) FROM messages;
SELECT '  Folders: ' || COUNT(*) FROM thread_folders;
EOF

exit 0
