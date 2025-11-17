# ChatGPT to Jan Importer

This script allows you to export all your conversations from ChatGPT and import them into Jan desktop application on macOS.

## Features

✅ Imports all ChatGPT conversations with full message history
✅ Preserves conversation titles and timestamps
✅ Maintains message order and roles (user/assistant/system)
✅ Handles multi-turn conversations
✅ Extracts model information from ChatGPT metadata
✅ Creates proper Jan thread and message format

## Prerequisites

- **Python 3.6+** installed on your Mac
- **Jan desktop app** installed on macOS
- **ChatGPT account** with conversations to export

## Step-by-Step Guide

### Step 1: Export Your ChatGPT Conversations

1. Go to [ChatGPT](https://chat.openai.com)
2. Click on your **profile icon** (bottom left)
3. Select **Settings**
4. Navigate to **Data controls**
5. Click **"Export data"**
6. Confirm your request
7. Wait for an **email from OpenAI** (usually takes a few minutes to 24 hours)
8. Click the download link in the email
9. Extract the downloaded ZIP file
10. Locate the **`conversations.json`** file

### Step 2: Close Jan Application

**Important:** Close Jan completely before running the import script to avoid data conflicts.

```bash
# Make sure Jan is not running
killall Jan 2>/dev/null || true
```

### Step 3: Run the Import Script

```bash
# Navigate to the scripts directory
cd /path/to/jan/scripts

# Make the script executable
chmod +x chatgpt_to_jan_importer.py

# Run the importer
python3 chatgpt_to_jan_importer.py --chatgpt-export /path/to/conversations.json
```

**Example:**

```bash
python3 chatgpt_to_jan_importer.py --chatgpt-export ~/Downloads/conversations.json
```

### Step 4: Open Jan and Verify

1. Open the **Jan desktop application**
2. Check your conversations list
3. Your imported ChatGPT conversations should now appear with the prefix "ChatGPT (Imported)"

## Advanced Usage

### Custom Jan Data Directory

If you've installed Jan in a custom location:

```bash
python3 chatgpt_to_jan_importer.py \
  --chatgpt-export conversations.json \
  --jan-dir "/custom/path/to/Jan/data"
```

### Default Jan Data Locations

- **macOS:** `~/Library/Application Support/Jan/data`
- **Windows:** `%APPDATA%/Jan/data` (not supported by this script yet)
- **Linux:** `~/.config/Jan/data` (not supported by this script yet)

## What Gets Imported

### Conversation Metadata
- ✅ Title
- ✅ Creation timestamp
- ✅ Last update timestamp
- ✅ Model information (GPT-3.5, GPT-4, etc.)

### Messages
- ✅ User messages
- ✅ Assistant responses
- ✅ System messages
- ✅ Message timestamps
- ✅ Message order

### Not Imported (Limitations)
- ❌ File attachments (images, documents)
- ❌ DALL-E generated images
- ❌ Code interpreter outputs
- ❌ Plugin interactions
- ❌ Shared conversation links

## Data Structure

The script creates the following structure in Jan:

```
~/Library/Application Support/Jan/data/threads/
  ├── {thread-id-1}/
  │   ├── thread.json          # Conversation metadata
  │   └── messages.jsonl       # All messages (one per line)
  ├── {thread-id-2}/
  │   ├── thread.json
  │   └── messages.jsonl
  └── ...
```

### Example thread.json

```json
{
  "id": "8f2c9922-db49-4d1e-8620-279c05baf2d0",
  "object": "thread",
  "title": "My ChatGPT Conversation",
  "assistants": [{
    "id": "chatgpt-import",
    "name": "ChatGPT (Imported)",
    "model": {
      "id": "gpt-4",
      "engine": "openai"
    },
    "instructions": "You are a helpful assistant."
  }],
  "created": 1634567890000,
  "updated": 1634567899000,
  "metadata": {
    "imported_from": "chatgpt",
    "original_conversation_id": "abc123",
    "import_date": 1700000000000
  }
}
```

### Example messages.jsonl (one message per line)

```json
{"id":"msg-001","object":"thread.message","thread_id":"8f2c9922-db49-4d1e-8620-279c05baf2d0","role":"user","content":[{"type":"text","text":{"value":"Hello!","annotations":[]}}],"status":"ready","created_at":1634567890000,"completed_at":1634567890000,"metadata":{}}
{"id":"msg-002","object":"thread.message","thread_id":"8f2c9922-db49-4d1e-8620-279c05baf2d0","role":"assistant","content":[{"type":"text","text":{"value":"Hi! How can I help you?","annotations":[]}}],"status":"ready","created_at":1634567891000,"completed_at":1634567891000,"metadata":{}}
```

## Troubleshooting

### "Jan data directory not found"

Make sure Jan has been run at least once to create the data directory:

```bash
ls -la ~/Library/Application\ Support/Jan/data
```

If it doesn't exist, create it manually:

```bash
mkdir -p ~/Library/Application\ Support/Jan/data/threads
```

### "Permission denied"

Make sure you have write permissions:

```bash
chmod +x chatgpt_to_jan_importer.py
```

### "Invalid JSON" or "Unexpected format"

Make sure you're using the correct `conversations.json` file from ChatGPT export, not a different JSON file.

### Conversations don't appear in Jan

1. Make sure Jan was completely closed during import
2. Restart Jan after import completes
3. Check the console output for any error messages
4. Verify files were created: `ls -la ~/Library/Application\ Support/Jan/data/threads/`

### Some conversations are missing

- Check the console output for skipped conversations
- The script skips empty conversations (no messages)
- Conversations with corrupted data may be skipped

## Safety Notes

⚠️ **Backup First:** Before running the import, backup your existing Jan data:

```bash
cp -r ~/Library/Application\ Support/Jan/data ~/Jan_backup_$(date +%Y%m%d)
```

⚠️ **Close Jan:** Always close Jan before running the import to prevent data corruption.

⚠️ **Test First:** Consider testing with a small export first to verify it works correctly.

## Support

If you encounter issues:

1. Check the troubleshooting section above
2. Review the console output for error messages
3. Verify your ChatGPT export file is valid JSON
4. Make sure you have the latest version of Jan installed

## License

This script is part of the Jan project and follows the same license.

## Contributing

Contributions are welcome! If you'd like to improve this importer:

1. Test with different ChatGPT export formats
2. Add support for Windows/Linux
3. Improve error handling
4. Add progress indicators for large imports
