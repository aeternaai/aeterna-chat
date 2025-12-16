# How to Add OAuth Client ID via Jan's UI

## Steps:

1. **Start Jan**:
   ```bash
   make dev
   ```

2. **Navigate to MCP Settings**:
   - Click the Settings icon (gear) in the sidebar
   - Click "MCP Servers" in the left menu

3. **Find JIRA ROVO Server**:
   - Scroll to find the `jira-rovo` server in the list

4. **Edit the Server Configuration**:
   
   **Option A - Edit JSON (Recommended):**
   - Click the **code icon** (`</>`) next to the jira-rovo server
   - You'll see the JSON configuration editor
   - Find the `oauth` section
   - Update the `client_id` field:
   
   ```json
   {
     "command": "npx",
     "args": ["-y", "mcp-remote", "https://mcp.atlassian.com/v1/sse", "--transport", "sse-only"],
     "type": "sse",
     "url": "https://mcp.atlassian.com/v1/sse",
     "oauth": {
       "client_id": "YOUR_ACTUAL_CLIENT_ID_HERE",  ← Change this
       "auth_url": "https://auth.atlassian.com/authorize",
       "token_url": "https://auth.atlassian.com/oauth/token",
       "scopes": [
         "read:jira-work",
         "read:confluence-content.summary",
         "read:jira-user"
       ]
     },
     "active": true,
     "official": false
   }
   ```
   
   - Click **Save**
   
   **Option B - Edit via Form:**
   - Click the **pencil icon** (✏️) to edit
   - This opens a form editor
   - However, OAuth fields might not be visible in the form yet
   - Use the JSON editor instead

5. **Verify**:
   - The server configuration is saved automatically
   - You should see the changes reflected immediately

---

## Method 2: Direct File Editing (Before Starting Jan)

If Jan is not running, you can directly edit the config file:

### Location:
```bash
# macOS
~/Library/Application Support/jan/mcp_config.json

# Linux
~/.config/jan/mcp_config.json

# Windows
%APPDATA%\jan\mcp_config.json
```

### Edit with your favorite editor:

```bash
# macOS/Linux
nano ~/Library/Application\ Support/jan/mcp_config.json

# Or use VS Code
code ~/Library/Application\ Support/jan/mcp_config.json
```

### Configuration Format:

```json
{
  "mcpServers": {
    "jira-rovo": {
      "command": "npx",
      "args": [
        "-y",
        "mcp-remote",
        "https://mcp.atlassian.com/v1/sse",
        "--transport",
        "sse-only"
      ],
      "type": "sse",
      "url": "https://mcp.atlassian.com/v1/sse",
      "env": {},
      "active": true,
      "official": false,
      "oauth": {
        "client_id": "YOUR_CLIENT_ID_HERE",
        "auth_url": "https://auth.atlassian.com/authorize",
        "token_url": "https://auth.atlassian.com/oauth/token",
        "scopes": [
          "read:jira-work",
          "read:confluence-content.summary",
          "read:jira-user"
        ]
      }
    }
  },
  "mcpSettings": {
    "toolCallTimeoutSeconds": 30,
    "baseRestartDelayMs": 1000,
    "maxRestartDelayMs": 30000,
    "backoffMultiplier": 2,
    "proactiveMode": false
  }
}
```

**Replace `YOUR_CLIENT_ID_HERE`** with your actual Atlassian OAuth client ID.

### Save and Start Jan:
```bash
make dev
```

---

## Method 3: Programmatic Update (Advanced)

If you want to update it via code or script:

```bash
#!/bin/bash

# Update client_id in Jan's MCP config
CLIENT_ID="your-actual-client-id"
CONFIG_FILE="$HOME/Library/Application Support/jan/mcp_config.json"

# Create backup
cp "$CONFIG_FILE" "$CONFIG_FILE.backup"

# Update using jq (install with: brew install jq)
jq ".mcpServers[\"jira-rovo\"].oauth.client_id = \"$CLIENT_ID\"" "$CONFIG_FILE" > "$CONFIG_FILE.tmp"
mv "$CONFIG_FILE.tmp" "$CONFIG_FILE"

echo "✅ Updated client_id in $CONFIG_FILE"
```

---

## Adding Client Secret (Important!)

The client secret is needed for the token exchange. You have two options:

### Option A: Environment Variable (Recommended - More Secure)

Add to your shell profile (`~/.zshrc` or `~/.bashrc`):

```bash
export JIRA_ROVO_CLIENT_SECRET="your-client-secret-here"
```

Then reload:
```bash
source ~/.zshrc
```

### Option B: MCP Server Environment Variables

In the MCP config, add to the `env` section:

```json
{
  "jira-rovo": {
    "env": {
      "OAUTH_CLIENT_SECRET": "your-client-secret-here"
    },
    "oauth": {
      "client_id": "your-client-id"
    }
  }
}
```

⚠️ **Security Warning**: Environment variables in config files are less secure. Prefer using system environment variables.

---

## Verification Steps

After updating the config:

1. **Check the file was updated**:
   ```bash
   cat ~/Library/Application\ Support/jan/mcp_config.json | jq '.mcpServers["jira-rovo"].oauth'
   ```
   
   Should show:
   ```json
   {
     "client_id": "your-client-id",
     "auth_url": "https://auth.atlassian.com/authorize",
     "token_url": "https://auth.atlassian.com/oauth/token",
     "scopes": [...]
   }
   ```

2. **Start Jan and verify in UI**:
   - Go to Settings → MCP Servers
   - Click the code icon on jira-rovo
   - Verify client_id is shown

3. **Test OAuth flow**:
   - Click the blue key icon (🔑)
   - Browser should open with correct client_id in URL
   - Check browser URL contains: `client_id=YOUR_CLIENT_ID`

---

## Complete Example

Here's a complete working config you can copy and modify:

```json
{
  "mcpServers": {
    "jira-rovo": {
      "command": "npx",
      "args": [
        "-y",
        "mcp-remote",
        "https://mcp.atlassian.com/v1/sse",
        "--transport",
        "sse-only"
      ],
      "type": "sse",
      "url": "https://mcp.atlassian.com/v1/sse",
      "env": {},
      "active": true,
      "official": false,
      "oauth": {
        "client_id": "REPLACE_WITH_YOUR_ATLASSIAN_CLIENT_ID",
        "auth_url": "https://auth.atlassian.com/authorize",
        "token_url": "https://auth.atlassian.com/oauth/token",
        "scopes": [
          "read:jira-work",
          "read:confluence-content.summary",
          "read:jira-user",
          "offline_access"
        ],
        "redirect_uri": "http://localhost:17390/callback"
      }
    }
  },
  "mcpSettings": {
    "toolCallTimeoutSeconds": 30,
    "baseRestartDelayMs": 1000,
    "maxRestartDelayMs": 30000,
    "backoffMultiplier": 2,
    "proactiveMode": false
  }
}
```

**Note**: The `redirect_uri` is optional in the config since it's hardcoded in the backend to `http://localhost:17390/callback`. But make sure this **exact URI** is registered in your Atlassian OAuth app settings!

---

## Troubleshooting

### Config not updating in UI?
- Restart Jan completely
- Check file permissions: `ls -la ~/Library/Application\ Support/jan/mcp_config.json`
- Make sure JSON is valid: `jq '.' ~/Library/Application\ Support/jan/mcp_config.json`

### OAuth flow shows error "invalid client_id"?
- Double-check client_id is correct (no spaces, exact match from Atlassian)
- Verify it's inside the `oauth` object, not at root level
- Check Atlassian OAuth app is active

### Where to get client_id?
1. Go to https://developer.atlassian.com/console/myapps/
2. Select your OAuth 2.0 app (or create one)
3. Copy the **Client ID** from the app details
4. The **Client secret** is shown separately - copy that too!

---

Need help setting up the Atlassian OAuth app itself? Let me know!
