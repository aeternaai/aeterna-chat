#!/bin/bash

# Simple Workspace Migration Script
# Migrates workspaces from file-based storage to SQLite database

set -e

DB_PATH=~/"Library/Application Support/Jan/data/jan.db"
WORKSPACES_DIR=~/"Library/Application Support/Jan/data/workspaces"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo "=== Workspace Migration to Database ==="
echo

# Check if database exists
if [ ! -f "$DB_PATH" ]; then
    echo -e "${RED}Error: Database not found: $DB_PATH${NC}"
    exit 1
fi

# Check if workspaces directory exists
if [ ! -d "$WORKSPACES_DIR" ]; then
    echo -e "${YELLOW}Warning: Workspaces directory not found: $WORKSPACES_DIR${NC}"
    echo "Nothing to migrate."
    exit 0
fi

# Count workspaces
TOTAL_WORKSPACES=$(find "$WORKSPACES_DIR" -type d -maxdepth 1 | tail -n +2 | wc -l | tr -d ' ')
echo "Found $TOTAL_WORKSPACES workspace directories"

MIGRATED=0
SKIPPED=0
FAILED=0

# Migrate each workspace
for workspace_dir in "$WORKSPACES_DIR"/*; do
    if [ ! -d "$workspace_dir" ]; then
        continue
    fi
    
    workspace_id=$(basename "$workspace_dir")
    workspace_file="$workspace_dir/workspace.json"
    
    if [ ! -f "$workspace_file" ]; then
        echo -e "${YELLOW}⚠️  No workspace.json for $workspace_id${NC}"
        ((FAILED++))
        continue
    fi
    
    # Check if workspace already exists in database
    exists=$(sqlite3 "$DB_PATH" "SELECT COUNT(*) FROM workspaces WHERE id = '$workspace_id';")
    
    if [ "$exists" -gt 0 ]; then
        echo "   Skipping $workspace_id (already in database)"
        ((SKIPPED++))
        continue
    fi
    
    # Read workspace JSON
    workspace_json=$(cat "$workspace_file")
    
    # Extract fields using jq (or fallback to python)
    if command -v jq &> /dev/null; then
        name=$(echo "$workspace_json" | jq -r '.name // ""')
        created=$(echo "$workspace_json" | jq -r '.created // 0')
        updated=$(echo "$workspace_json" | jq -r '.updated // 0')
        visibility=$(echo "$workspace_json" | jq -r '.visibility // "private"')
    else
        # Fallback to python
        name=$(echo "$workspace_json" | python3 -c "import sys, json; print(json.load(sys.stdin).get('name', ''))")
        created=$(echo "$workspace_json" | python3 -c "import sys, json; print(json.load(sys.stdin).get('created', 0))")
        updated=$(echo "$workspace_json" | python3 -c "import sys, json; print(json.load(sys.stdin).get('updated', 0))")
        visibility=$(echo "$workspace_json" | python3 -c "import sys, json; print(json.load(sys.stdin).get('visibility', 'private'))")
    fi
    
    # Escape single quotes in JSON for SQL
    workspace_json_escaped=$(echo "$workspace_json" | sed "s/'/''/g")
    name_escaped=$(echo "$name" | sed "s/'/''/g")
    
    # Insert workspace into database
    sqlite3 "$DB_PATH" <<EOF
INSERT INTO workspaces (id, name, visibility, created_at, updated_at, metadata)
VALUES ('$workspace_id', '$name_escaped', '$visibility', $created, $updated, '$workspace_json_escaped');
EOF
    
    if [ $? -eq 0 ]; then
        echo -e "${GREEN}✓${NC} Migrated workspace: $workspace_id ($name)"
        ((MIGRATED++))
        
        # Migrate workspace files if they exist
        files_count=0
        files_failed=0
        
        # Check for common file patterns in workspace directory
        for file in "$workspace_dir"/*.pdf "$workspace_dir"/*.txt "$workspace_dir"/*.md; do
            if [ -f "$file" ]; then
                filename=$(basename "$file")
                filepath="$file"
                file_size=$(stat -f%z "$file" 2>/dev/null || stat -c%s "$file" 2>/dev/null || echo 0)
                uploaded_at=$(date +%s)
                
                # Escape single quotes
                filename_escaped=$(echo "$filename" | sed "s/'/''/g")
                filepath_escaped=$(echo "$filepath" | sed "s/'/''/g")
                
                # Insert workspace file
                sqlite3 "$DB_PATH" <<FILEEOF
INSERT OR IGNORE INTO workspace_files (workspace_id, filename, filepath, file_size, uploaded_at)
VALUES ('$workspace_id', '$filename_escaped', '$filepath_escaped', $file_size, $uploaded_at);
FILEEOF
                
                if [ $? -eq 0 ]; then
                    ((files_count++))
                else
                    ((files_failed++))
                fi
            fi
        done
        
        if [ $files_count -gt 0 ]; then
            echo "      → Migrated $files_count workspace files"
        fi
        if [ $files_failed -gt 0 ]; then
            echo -e "      ${YELLOW}→ Failed $files_failed workspace files${NC}"
        fi
    else
        echo -e "${RED}✗${NC} Failed to migrate: $workspace_id"
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
SELECT '  Workspaces: ' || COUNT(*) FROM workspaces;
SELECT '  Workspace Files: ' || COUNT(*) FROM workspace_files;
SELECT '  Threads: ' || COUNT(*) FROM threads;
EOF

exit 0
