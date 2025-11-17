#!/bin/bash
# Backup Jan data directory before importing ChatGPT conversations

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Jan data directory (macOS default)
JAN_DATA_DIR="$HOME/Library/Application Support/Jan/data"

# Backup directory
BACKUP_DIR="$HOME/Jan_backup_$(date +%Y%m%d_%H%M%S)"

echo -e "${YELLOW}🔐 Jan Data Backup Utility${NC}"
echo "================================"
echo ""

# Check if Jan data directory exists
if [ ! -d "$JAN_DATA_DIR" ]; then
    echo -e "${RED}✗ Jan data directory not found: $JAN_DATA_DIR${NC}"
    echo "Make sure Jan has been run at least once."
    exit 1
fi

echo -e "${GREEN}✓${NC} Found Jan data directory: $JAN_DATA_DIR"

# Calculate size
SIZE=$(du -sh "$JAN_DATA_DIR" | cut -f1)
echo -e "${GREEN}✓${NC} Data size: $SIZE"

# Create backup
echo ""
echo "📦 Creating backup..."
echo "   Source: $JAN_DATA_DIR"
echo "   Destination: $BACKUP_DIR"
echo ""

if cp -r "$JAN_DATA_DIR" "$BACKUP_DIR"; then
    echo -e "${GREEN}✓ Backup created successfully!${NC}"
    echo ""
    echo "Backup location: $BACKUP_DIR"
    echo ""
    echo "To restore from this backup:"
    echo "  rm -rf '$JAN_DATA_DIR'"
    echo "  cp -r '$BACKUP_DIR' '$JAN_DATA_DIR'"
    echo ""
else
    echo -e "${RED}✗ Backup failed${NC}"
    exit 1
fi
