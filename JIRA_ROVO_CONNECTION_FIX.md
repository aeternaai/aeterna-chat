# JIRA ROVO MCP Server Connection Fix

## Problem Summary

The JIRA ROVO MCP server (`jira-rovo`) was failing to connect in Jan, even though the same command worked perfectly when run manually via:

```bash
npx -y mcp-remote https://mcp.atlassian.com/v1/sse --transport sse-only
```

## Root Cause Analysis

The issue was in the MCP server configuration - specifically **missing arguments in the args array**:

### Incorrect Configuration (Before Fix)
```json
{
  "command": "npx",
  "args": ["-y", "mcp-remote", "https://mcp.atlassian.com/v1/sse"],  // Missing --transport flag!
  "env": {},
  "active": false,
  "official": true
}
```

### Error Symptoms
Looking at the logs (`/Users/maot/Library/Application Support/Jan/data/logs/app.log`):

1. **Initial attempts**: Empty error messages
   ```
   [ERROR] Failed to start MCP server jira-rovo: 
   ```

2. **Later attempts**: Missing file error
   ```
   [ERROR] Failed to run command jira-rovo: No such file or directory (os error 2)
   ```

3. **Final error**: NPX trying to install URL as a package
   ```
   [ERROR] Failed to start MCP server jira-rovo: error: unrecognised dependency format: @https://mcp.atlassian.com/v1/sse
   ```

This last error was the key - `npx` was interpreting `https://mcp.atlassian.com/v1/sse` as a package name instead of an argument to `mcp-remote`, because `mcp-remote` was missing from the args!

## The Fix

### Corrected Configuration
```json
{
  "command": "npx",
  "args": ["-y", "mcp-remote", "https://mcp.atlassian.com/v1/sse", "--transport", "sse-only"],
  "env": {},
  "active": false,
  "official": true
}
```

### Files Modified

1. **`src-tauri/src/core/setup.rs`** (Line ~211)
   - Fixed the migration code that adds jira-rovo during schema version 3 migration
   - Added missing `"--transport", "sse-only"` to args array

2. **User config file** (Runtime fix)
   - Updated `/Users/maot/Library/Application Support/Jan/data/mcp_config.json`
   - Fixed the existing broken config to include proper args

## Technical Details

### How Jan's MCP System Works

Jan supports two types of MCP server connections:

1. **Direct HTTP/SSE Transport** (when `type: "http"` or `type: "sse"` AND `url` field exists)
   - Jan creates HTTP/SSE client directly
   - Connects to the URL specified in the `url` field
   - Example: Direct connection to an HTTP API

2. **Child Process / STDIO Transport** (default, no `type` or `type: "stdio"`)
   - Jan spawns a child process with the given command + args
   - Communicates via stdin/stdout using MCP protocol
   - Example: `npx mcp-remote` (which internally uses SSE to proxy to the remote server)

The ROVO server uses **option #2** - it spawns `mcp-remote` as a child process, and `mcp-remote` handles the SSE connection internally. The `--transport sse-only` flag tells `mcp-remote` to use SSE transport.

### Why the Manual Command Worked

When running manually:
```bash
npx -y mcp-remote https://mcp.atlassian.com/v1/sse --transport sse-only
```

- `npx -y mcp-remote` → Downloads/runs the `mcp-remote` package
- `https://mcp.atlassian.com/v1/sse` → URL argument to mcp-remote
- `--transport sse-only` → Tells mcp-remote to use SSE-only mode
- `mcp-remote` acts as a proxy: STDIO ↔ SSE

### Why Jan's Config Failed

Without the complete args array, Jan was running:
```bash
npx -y mcp-remote https://mcp.atlassian.com/v1/sse
# Missing: --transport sse-only
```

Or in the worst case (when args got corrupted):
```bash
npx https://mcp.atlassian.com/v1/sse --transport sse-only
# Missing: -y mcp-remote
```

This caused npx to try installing `https://mcp.atlassian.com/v1/sse` as a package, leading to the "unrecognised dependency format" error.

## Current Status

### ✅ Configuration Fix Applied
The args array is now complete and the server spawns correctly:
```json
{
  "command": "npx",
  "args": ["-y", "mcp-remote", "https://mcp.atlassian.com/v1/sse", "--transport", "sse-only"],
  "env": {},
  "active": false,
  "official": true
}
```

### ⚠️ Known Limitation: OAuth Authentication Required

**The ROVO MCP server requires OAuth authentication**, which Jan doesn't currently support for remote MCP servers. 

Symptoms observed:
```
[INFO] Server jira-rovo started successfully.
[INFO] Marked MCP server jira-rovo as successfully connected
[INFO] Monitoring MCP server jira-rovo health
[WARN] MCP server jira-rovo health check failed: Transport closed
```

The server spawns correctly and connects, but health checks fail because ROVO requires OAuth before responding to `tools/list` requests.

### Testing Confirms Authentication Needed

Manual testing shows the server waits for authentication:
```bash
# This connects but hangs waiting for auth:
echo '{"jsonrpc":"2.0","id":1,"method":"tools/list","params":{}}' | \
  npx -y mcp-remote https://mcp.atlassian.com/v1/sse --transport sse-only
```

## Next Steps for Full ROVO Support

To make ROVO fully functional in Jan, we need to:

1. **Implement OAuth Flow for MCP Servers**
   - Add OAuth configuration to MCP server config (client_id, auth_url, token_url)
   - Implement OAuth browser flow
   - Store and refresh OAuth tokens
   - Add tokens to request headers

2. **Update Health Check Logic**
   - Skip health checks for servers requiring authentication
   - Or implement a different health check (process alive check instead of API call)

3. **Add UI for OAuth Authentication**
   - Show "Authenticate" button for ROVO server
   - Open browser for OAuth flow
   - Display authentication status

## Verification Steps

1. **Check the config is correct**:
   ```bash
   cat "/Users/maot/Library/Application Support/Jan/data/mcp_config.json" | jq '.mcpServers["jira-rovo"]'
   ```

2. **Test the command manually**:
   ```bash
   npx -y mcp-remote https://mcp.atlassian.com/v1/sse --transport sse-only
   ```
   Expected: Should connect and show "Proxy established successfully"

3. **Check Jan startup logs**:
   ```bash
   tail -f "/Users/maot/Library/Application Support/Jan/data/logs/app.log" | grep jira-rovo
   ```
   
4. **Expected current behavior** (with this fix):
   ```
   [INFO] Starting MCP server jira-rovo (Initial attempt)
   [INFO] Server jira-rovo started successfully.
   [INFO] Marked MCP server jira-rovo as successfully connected
   [INFO] Monitoring MCP server jira-rovo health
   [WARN] MCP server jira-rovo health check failed: Transport closed
   ```
   
   This is expected until OAuth is implemented - the server spawns correctly but can't respond without authentication.

## Related Files

- `src-tauri/src/core/mcp/constants.rs` - Default MCP config template
- `src-tauri/src/core/setup.rs` - Migration code for adding new MCP servers
- `src-tauri/src/core/mcp/helpers.rs` - MCP server startup logic
- `src-tauri/src/core/mcp/models.rs` - MCP configuration models

## Prevention

To prevent similar issues in the future:

1. **Always test MCP server configs manually first**:
   ```bash
   npx -y <package> <args...>
   ```

2. **Validate args array completeness** - ensure all command-line flags are included

3. **Check logs immediately** after adding a new MCP server to catch configuration errors early

4. **Add integration tests** for MCP server configurations (future improvement)

## References

- [Model Context Protocol Specification](https://modelcontextprotocol.io)
- [mcp-remote package](https://www.npmjs.com/package/mcp-remote) - Proxy tool for connecting to remote MCP servers
- [Atlassian ROVO MCP Documentation](https://mcp.atlassian.com)
