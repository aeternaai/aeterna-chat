# Quick Start: Import ChatGPT to Jan

## TL;DR - 3 Simple Steps

### 1️⃣ Export from ChatGPT

Visit [chat.openai.com](https://chat.openai.com) → Settings → Data controls → Export data
Wait for email → Download → Extract `conversations.json`

### 2️⃣ Backup Jan Data (Optional but Recommended)

```bash
cd jan/scripts
./backup_jan_data.sh
```

### 3️⃣ Import to Jan

```bash
# Close Jan first!
killall Jan 2>/dev/null

# Validate your export (optional)
python3 validate_chatgpt_export.py ~/Downloads/conversations.json

# Import conversations
python3 chatgpt_to_jan_importer.py --chatgpt-export ~/Downloads/conversations.json
```

Done! Open Jan and your ChatGPT conversations will be there.

## What You'll Need

- ✅ macOS (this script is for Mac)
- ✅ Python 3.6+ (already installed on macOS)
- ✅ Jan desktop app installed
- ✅ Your ChatGPT export file

## File Descriptions

| File | Purpose |
|------|---------|
| `chatgpt_to_jan_importer.py` | Main import script |
| `validate_chatgpt_export.py` | Check if your export file is valid |
| `backup_jan_data.sh` | Backup your Jan data before import |
| `README_CHATGPT_IMPORT.md` | Full documentation |

## Common Issues

**"Jan data directory not found"**
→ Run Jan at least once to create the data directory

**Conversations don't appear**
→ Make sure you closed Jan before importing, then restart it

**"Permission denied"**
→ Run: `chmod +x *.py *.sh` in the scripts directory

## Need More Help?

Read the full documentation: [README_CHATGPT_IMPORT.md](./README_CHATGPT_IMPORT.md)
