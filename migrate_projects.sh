#!/bin/bash

# Simple Project/Folder Migration Script
# Migrates thread folders/projects from localStorage export to SQLite database
#
# NOTE: This script expects a JSON export of localStorage['thread-management']
# To export from browser console:
#   1. Open Jan app
#   2. Open browser DevTools (Cmd+Option+I on macOS)
#   3. Go to Console tab
#   4. Run: console.log(JSON.stringify(JSON.parse(localStorage.getItem('thread-management')), null, 2))
#   5. Copy the JSON output and save to: ~/Downloads/thread-management.json

set -e

DB_PATH=~/"Library/Application Support/Jan/data/jan.db"
EXPORT_FILE=~/Downloads/thread-management.json

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "=== Project/Folder Migration to Database ==="
echo

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo -e "${RED}Error: Database not found: $DB_PATH${NC}"
    exit 1
fi

# Check if export file exists
if [ ! -f "$EXPORT_FILE" ]; then
    echo -e "${RED}Error: Export file not found: $EXPORT_FILE${NC}"
    echo
    echo "To create the export file:"
    echo "1. Open Jan app"
    echo "2. Open browser DevTools (Cmd+Option+I on macOS)"
    echo "3. Go to Console tab"
    echo "4. Run this command:"
    echo "   console.log(JSON.stringify(JSON.parse(localStorage.getItem('thread-management')), null, 2))"
    echo "5. Copy the JSON output and save to: $EXPORT_FILE"
    echo
    exit 1
fi

# Check for jq or python
if ! command -v jq &> /dev/null && ! command -v python3 &> /dev/null; then
    echo -e "${RED}Error: Neither jq nor python3 is installed${NC}"
    echo "Install jq with: brew install jq"
    exit 1
fi

echo "Reading export file: $EXPORT_FILE"
export_json=$(cat "$EXPORT_FILE")

# Extract folders array from state
if command -v jq &> /dev/null; then
    folders_count=$(echo "$export_json" | jq '.state.folders | length')
    echo "Found $folders_count folders/projects to migrate"
    echo
else
    folders_count=$(echo "$export_json" | python3 -c "import sys, json; print(len(json.load(sys.stdin).get('state', {}).get('folders', [])))")
    echo "Found $folders_count folders/projects to migrate"
    echo
fi

MIGRATED=0
SKIPPED=0
FAILED=0

# Process each folder
for i in $(seq 0 $((folders_count - 1))); do
    if command -v jq &> /dev/null; then
        folder_json=$(echo "$export_json" | jq ".state.folders[$i]")
        folder_id=$(echo "$folder_json" | jq -r '.id // ""')
        folder_name=$(echo "$folder_json" | jq -r '.name // ""')
        updated_at=$(echo "$folder_json" | jq -r '.updated_at // 0')
        # Convert milliseconds to seconds and use updated_at for both created and updated
        created=$((updated_at / 1000))
        updated=$((updated_at / 1000))
    else
        folder_json=$(echo "$export_json" | python3 -c "import sys, json; print(json.dumps(json.load(sys.stdin)['state']['folders'][$i]))")
        folder_id=$(echo "$folder_json" | python3 -c "import sys, json; print(json.load(sys.stdin).get('id', ''))")
        folder_name=$(echo "$folder_json" | python3 -c "import sys, json; print(json.load(sys.stdin).get('name', ''))")
        updated_at=$(echo "$folder_json" | python3 -c "import sys, json; print(json.load(sys.stdin).get('updated_at', 0))")
        # Convert milliseconds to seconds
        created=$((updated_at / 1000))
        updated=$((updated_at / 1000))
    fi
    
    if [ -z "$folder_id" ]; then
        echo -e "${YELLOW}⚠️  Skipping folder with no ID${NC}"
        ((FAILED++))
        continue
    fi
    
    # Check if folder already exists in database
    exists=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM thread_folders WHERE id = '$folder_id';")
    
    if [ "$exists" -gt 0 ]; then
        echo "   Skipping $folder_id (already in database)"
        ((SKIPPED++))
        continue
    fi
    
    # Escape single quotes
    folder_name_escaped=$(echo "$folder_name" | sed "s/'/''/g")
    
    # Insert folder into database (no metadata column in thread_folders table)
    sqlite3 "$DB_PATH" <<EOF
INSERT INTO thread_folders (id, workspace_id, name, created_at, updated_at)
VALUES ('$folder_id', NULL, '$folder_name_escaped', $created, $updated);
EOF
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} Migrated folder: $folder_id ($folder_name)"
        ((MIGRATED++))
    else
        echo -e "${RED}✗${NC} Failed to migrate: $folder_id"
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

# Sync folder_id for threads that have project metadata
echo
echo "Syncing folder_id for threads with project metadata..."
sync_result=$(sqlite3 "$DB_PATH" <<'EOF'
UPDATE threads 
SET folder_id = json_extract(metadata, '$.metadata.project.id')
WHERE folder_id IS NULL 
  AND json_extract(metadata, '$.metadata.project.id') IS NOT NULL;
SELECT changes();
EOF
)

if [ "$sync_result" -gt 0 ]; then
    echo -e "${GREEN}✓${NC} Synced folder_id for $sync_result threads"
else
    echo "   No threads needed folder_id sync"
fi

# Show updated stats
echo
echo "Database statistics:"
sqlite3 "$DB_PATH" <<'EOF'
SELECT '  Folders: ' || COUNT(*) FROM thread_folders;
SELECT '  Threads with folder: ' || COUNT(*) FROM threads WHERE folder_id IS NOT NULL;
SELECT '  Threads without folder: ' || COUNT(*) FROM threads WHERE folder_id IS NULL;
EOF

exit 0
