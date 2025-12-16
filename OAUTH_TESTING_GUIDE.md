# OAuth Flow Quick Start Guide

## Prerequisites

Before you can test OAuth with JIRA ROVO, you need:

### 1. Get Atlassian OAuth Credentials

1. Go to https://developer.atlassian.com/
2. Sign in with your Atlassian account
3. Navigate to **Console** → **Create** → **OAuth 2.0 integration**
4. Fill in details:
   - **App name**: Jan MCP Test (or any name)
   - **Redirect URI**: `http://localhost:17390/callback` ⚠️ **CRITICAL - Must be exact**
5. Click **Create**
6. Copy your **Client ID** and **Client Secret**

### 2. Update Jan's JIRA ROVO Configuration

Option A - Via UI (Once app is running):
1. Start Jan: `make dev`
2. Go to Settings → MCP Servers
3. Find `jira-rovo` server
4. Click the **Edit JSON** button (code icon `</>`)
5. Update the `oauth.client_id` field with your Client ID
6. Save

Option B - Via File (Before starting):
1. Find your Jan config: `~/Library/Application Support/jan/mcp_config.json`
2. Update the jira-rovo section:
   ```json
   {
     "mcpServers": {
       "jira-rovo": {
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
     }
   }
   ```

### 3. (Optional) Set Client Secret as Environment Variable

The client secret is needed for token exchange. Add to your environment:

```bash
export JIRA_ROVO_CLIENT_SECRET="your_client_secret_here"
```

Or configure it in the MCP server env vars.

---

## Testing the OAuth Flow

### Method 1: Via Jan UI (Recommended)

1. **Start Jan**:
   ```bash
   make dev
   ```

2. **Navigate to MCP Settings**:
   - Click Settings (gear icon)
   - Click "MCP Servers" in sidebar

3. **Find JIRA ROVO Server**:
   - Look for `jira-rovo` in the server list
   - You should see an amber badge: **"Auth Required"** 🟡

4. **Authenticate**:
   - Click the **blue key icon** (🔑) in the actions row
   - Your browser will automatically open to Atlassian login
   - Log in with your Atlassian account
   - Grant the requested permissions
   - Browser will redirect to `http://localhost:17390/callback`
   - You'll see a success message

5. **Verify**:
   - Go back to Jan
   - The badge should now be **green**: "Authenticated" 🟢
   - The key icon should now be **red** (🔓)
   - Check logs: `tail -f ~/Library/Logs/Jan/app.log | grep -i oauth`

6. **Test MCP Connection**:
   - The jira-rovo server should now connect successfully
   - Try using JIRA tools in a chat

---

### Method 2: Via Browser Console (Debugging)

1. **Start Jan**: `make dev`

2. **Open DevTools**: 
   - macOS: `Cmd+Option+I`
   - Windows/Linux: `Ctrl+Shift+I`

3. **Run in Console**:
   ```javascript
   const { invoke } = window.__TAURI__.core;
   
   // Start OAuth flow
   invoke('start_mcp_oauth_flow', { serverName: 'jira-rovo' })
     .then(result => {
       console.log('Auth URL:', result.auth_url);
       window.open(result.auth_url, '_blank');
     });
   
   // Listen for completion
   window.__TAURI__.event.listen('mcp_oauth_complete', (e) => {
     console.log('✅ OAuth Success!', e.payload);
   });
   ```

4. **Complete OAuth in opened browser**

5. **Check status**:
   ```javascript
   invoke('get_mcp_oauth_status', { serverName: 'jira-rovo' })
     .then(status => console.log('Status:', status));
   ```

---

## Monitoring the OAuth Flow

### Watch Logs in Real-Time

Terminal 1 - App logs:
```bash
tail -f ~/Library/Logs/Jan/app.log | grep -E "oauth|jira-rovo"
```

Terminal 2 - MCP logs:
```bash
tail -f ~/Library/Logs/Jan/app.log | grep -i mcp
```

### Check Token Storage

After successful OAuth:
```bash
cat "$HOME/Library/Application Support/jan/mcp_oauth_tokens.json" | jq '.'
```

Should show:
```json
{
  "jira-rovo": {
    "access_token": "eyJ...",
    "refresh_token": "eyJ...",
    "expires_at": 1234567890
  }
}
```

---

## Troubleshooting

### Issue: "OAuth flow failed"
- **Check**: Is your client_id correct in mcp_config.json?
- **Check**: Is the redirect_uri exactly `http://localhost:17390/callback` in Atlassian settings?
- **Check**: Is port 17390 available? (Run `lsof -i :17390`)

### Issue: "Failed to exchange code for token"
- **Check**: Do you have the client_secret configured?
- **Check**: Is the authorization code valid? (They expire quickly)
- **Check**: Network connectivity to Atlassian servers

### Issue: Browser redirects but nothing happens
- **Check**: Is Jan still running?
- **Check**: Logs for callback server errors: `grep "callback" ~/Library/Logs/Jan/app.log`

### Issue: Badge doesn't update
- **Check**: Events are being emitted (see logs)
- **Refresh** the MCP Servers settings page
- **Check**: `get_mcp_oauth_status` returns authenticated: true

---

## What Happens Behind the Scenes

1. **User clicks authenticate**:
   - Frontend calls `start_mcp_oauth_flow('jira-rovo')`
   
2. **Backend generates OAuth URL**:
   - Reads OAuth config from mcp_config.json
   - Generates random state parameter for security
   - Constructs authorization URL
   - Starts HTTP callback server on port 17390
   
3. **Browser opens OAuth page**:
   - User sees Atlassian login
   - Grants permissions to Jan
   
4. **Atlassian redirects to callback**:
   - URL: `http://localhost:17390/callback?code=ABC123&state=xyz`
   - Jan's callback server receives this
   
5. **Backend exchanges code for token**:
   - Validates state parameter
   - POSTs to `https://auth.atlassian.com/oauth/token`
   - Receives access_token and refresh_token
   
6. **Token stored**:
   - Saved to `~/Library/Application Support/jan/mcp_oauth_tokens.json`
   - Emits `mcp_oauth_complete` event
   
7. **MCP server restarts with token**:
   - Token added as Authorization header
   - Server connects successfully

---

## Next Steps After Successful OAuth

1. **Test JIRA Tools**:
   - Start a new chat
   - Ask: "What are my recent JIRA issues?"
   - Jan should use JIRA ROVO tools to fetch data

2. **Test Token Refresh**:
   - Wait for token to expire (or manually set expires_at in past)
   - Backend should automatically refresh token

3. **Test Revocation**:
   - Click red key-off icon
   - Verify token is removed
   - Server should disconnect

---

## Security Notes

⚠️ **Never commit real OAuth credentials to git**
⚠️ **Client secrets should be in environment variables, not config files**
⚠️ **Tokens are stored in Jan's data directory - keep it secure**
⚠️ **Callback server only runs during OAuth flow (stops after receiving code)**

---

## Success Indicators

✅ Browser opens to Atlassian login
✅ After login, redirect shows "OAuth successful" page
✅ Jan logs show: "OAuth flow completed for jira-rovo"
✅ Token file exists and contains valid token
✅ UI badge turns green "Authenticated"
✅ JIRA ROVO server shows as connected (green dot)
✅ JIRA tools are available in chat

---

## Files to Watch

- Config: `~/Library/Application Support/jan/mcp_config.json`
- Tokens: `~/Library/Application Support/jan/mcp_oauth_tokens.json`
- Logs: `~/Library/Logs/Jan/app.log`
