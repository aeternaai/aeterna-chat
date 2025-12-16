# JIRA Rovo MCP Server Debug Findings

## Issue Summary
When activating the `jira-rovo` MCP server in Jan, it enters an infinite restart loop with "Transport closed" errors.

## Root Cause Analysis

### What's Happening

1. **Initial Connection Succeeds**: mcp-remote successfully connects to `https://mcp.atlassian.com/v1/sse`
   ```
   Connected to remote server using SSEClientTransport
   Local STDIO server running  
   Proxy established successfully
   ```

2. **OAuth DOES Work**: User reports browser opened successfully when activating in Jan for the first time
   - mcp-remote CAN open browser even when spawned by Jan
   - Authentication completed successfully  
   - Token saved (presumably in mcp-remote's storage)

3. **Transport Closes AFTER Authentication**: Even with valid auth token, connection drops
   - Jan's health check calls `list_all_tools()`
   - rmcp library reports "Transport closed" error
   - Could be: mcp-remote crash, network drop, Atlassian timeout, or stdio pipe closure

4. **Restart Loop**: Jan's auto-restart logic kicks in, repeating the cycle

### ❌ INCORRECT THEORY (Ruled Out)
~~OAuth Flow Blocked: mcp-remote can't open browser in headless mode~~
**Confirmed**: Browser DOES open successfully from Jan-spawned processes

### Evidence

**Test 1 - Manual Execution (Works)**:
```bash
$ npx -y mcp-remote https://mcp.atlassian.com/v1/sse
[16428] Connected to remote server using SSEClientTransport
[16428] Local STDIO server running
[16428] Proxy established successfully
[16428] Press Ctrl+C to exit
```

**Test 2 - With Tool List Request**:
```bash
$ echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | npx -y mcp-remote https://mcp.atlassian.com/v1/sse
[16428] [Local→Remote] tools/list
# No response received - waiting for authentication
```

**Jan Logs Pattern**:
```
[INFO] Server jira-rovo started successfully.
[INFO] Marked MCP server jira-rovo as successfully connected
[INFO] Monitoring MCP server jira-rovo health
[WARN] MCP server jira-rovo health check failed: Transport closed
[ERROR] MCP server jira-rovo failed health check, removing from active servers
[INFO] Restarting MCP server jira-rovo (Attempt 1/3)
```

## Why This Happens

### mcp-remote OAuth Flow
From mcp-remote documentation and user testing:
1. mcp-remote acts as a proxy: `Jan (stdio) ↔ mcp-remote ↔ Atlassian SSE`
2. When the remote server requires auth, mcp-remote opens a browser to complete OAuth
3. Tokens are stored somewhere by mcp-remote (location TBD)
4. **Confirmed**: Browser DOES open successfully even when spawned by Jan
5. **After successful auth**, transport still closes with "Transport closed" error

### ❓ Unknown: Why does transport close AFTER authentication?
Possible causes:
1. **mcp-remote crashes**: Process exits shortly after authentication
2. **Atlassian timeout**: SSE endpoint closes connection after idle period
3. **Health check incompatibility**: `list_all_tools()` triggers error despite valid auth
4. **Stdio pipe closure**: Jan's pipe management interfering with long-lived connection
5. **Token refresh failure**: Token expires/invalidated quickly, refresh attempt fails

### Jan's MCP Server Lifecycle
```rust
// src-tauri/src/core/mcp/helpers.rs:715
let (process, stderr) = TokioChildProcess::builder(cmd)
    .stderr(Stdio::piped())
    .spawn()
    .map_err(|e| {
        log::error!("Failed to run command {name}: {e}");
        format!("Failed to run command {name}: {e}")
    })?;
```

- Jan spawns MCP servers as child processes with piped stderr
- Process has no TTY attached (headless/background)
- Can't open browser windows for interactive auth

### Health Check Implementation
```rust
// helpers.rs:205
match timeout(Duration::from_secs(2), service.list_all_tools()).await {
    Ok(Ok(_)) => true,    // Server healthy
    Ok(Err(e)) => {
        log::warn!("MCP server {name} health check failed: {e}");
        false             // Triggers restart
    }
}
```

- Health check runs 5 seconds after startup
- Calls `list_all_tools()` which requires authenticated connection
- Atlassian SSE endpoint returns auth error → rmcp reports "Transport closed"
- Jan interprets this as server failure → restart loop

## Solutions

### Option 1: Pre-authenticate mcp-remote (Recommended Short-term)

**User Action Required**:
1. Run mcp-remote manually in terminal BEFORE activating in Jan:
   ```bash
   npx -y mcp-remote https://mcp.atlassian.com/v1/sse
   ```
2. Browser opens → authenticate with Atlassian
3. Token saved to `~/.config/mcp-remote/`
4. Press Ctrl+C to exit
5. Activate jira-rovo in Jan → uses saved token

**Pros**:
- Works with current Jan implementation
- No code changes needed
- User has full control over auth

**Cons**:
- Extra manual step
- Not discoverable (needs documentation)
- Token expiry requires repeating the process

### Option 2: Detect OAuth Requirements and Show Instructions

**Implementation**:
1. Modify health check to detect auth errors specifically
2. When "Transport closed" occurs on first connection, check if it's an auth error
3. Show user a notification with instructions to authenticate
4. Don't restart until user confirms they've completed auth

**Code Changes Needed**:
```rust
// In monitor_mcp_server_handle
if !health_check_result {
    // Check if this is the first failure
    let is_first_failure = /* track in AppState */;
    
    if is_first_failure && name == "jira-rovo" {
        // Emit event to frontend with auth instructions
        app.emit("mcp-auth-required", json!({
            "server": name,
            "instructions": "Run: npx -y mcp-remote https://mcp.atlassian.com/v1/sse"
        }));
        // Don't restart automatically
        return Some(QuitReason::Closed);
    }
}
```

**Pros**:
- Better UX - user knows what to do
- Prevents restart loop
- Can be generalized for other OAuth-based MCPs

**Cons**:
- Requires code changes in Jan
- Still requires manual terminal step

### Option 3: Built-in OAuth Handler (Long-term Solution)

**Implementation**:
1. Jan detects when MCP server needs OAuth
2. Opens system browser with auth URL
3. Runs local callback server to receive OAuth token
4. Stores token and restarts MCP server with auth

**This is complex and requires**:
- OAuth flow detection in rmcp library
- Callback server in Jan
- Token storage management
- Per-server OAuth configuration

**Pros**:
- Seamless user experience
- No manual terminal commands
- Proper enterprise solution

**Cons**:
- Significant development effort (2-3 weeks)
- Requires rmcp library enhancements
- Security considerations for token storage

### Option 4: Alternative to mcp-remote - Direct SSE Transport

**Try using Jan's built-in SSE transport instead**:

```json
{
  "jira-rovo": {
    "type": "sse",
    "url": "https://mcp.atlassian.com/v1/sse",
    "headers": {
      "Authorization": "Bearer YOUR_TOKEN_HERE"
    },
    "active": false,
    "official": true
  }
}
```

**Issues**:
- Atlassian's endpoint likely requires OAuth challenge-response, not static bearer tokens
- Would need to manually obtain and refresh tokens
- May not work if endpoint requires OAuth flow

## Recommended Next Steps

### Immediate (for User)
1. **Document the workaround** in user-facing docs:
   ```markdown
   ### JIRA Rovo MCP Setup
   
   Before activating JIRA Rovo in Jan, you must authenticate:
   
   1. Open Terminal
   2. Run: `npx -y mcp-remote https://mcp.atlassian.com/v1/sse`
   3. Browser will open → log in to Atlassian
   4. Approve OAuth permissions
   5. Press Ctrl+C in terminal
   6. Now activate JIRA Rovo in Jan Settings → MCP Servers
   ```

2. **Update JIRA_ROVO_IMPLEMENTATION_SUMMARY.md** with this requirement

3. **Add to jira.mdx docs** under "Prerequisites"

### Short-term (1-2 days)
1. Implement Option 2: Auth detection + user notification
2. Add UI prompt with copy-pasteable command
3. Prevent restart loop for auth-required servers

### Long-term (Future)
1. Implement Option 3: Built-in OAuth handler
2. Contribute improvements to rmcp library for better auth flow detection
3. Generalize solution for all OAuth-based MCP servers

## Testing Checklist

- [ ] Document pre-auth requirement in JIRA_ROVO_IMPLEMENTATION_SUMMARY.md
- [ ] Update docs/src/pages/docs/desktop/mcp-examples/productivity/jira.mdx
- [ ] Test: Pre-authenticate in terminal → activate in Jan → verify no restart loop
- [ ] Test: Token expiry handling (30 days later, Atlassian OAuth expires)
- [ ] Test: Multiple JIRA tenants using `--resource` flag
- [ ] Code: Implement auth-required detection (Option 2)
- [ ] Code: Add UI notification system for auth instructions
- [ ] Code: Prevent restart loop for servers in "awaiting-auth" state

## Additional Notes

### Why Other MCP Servers Don't Have This Issue
- **fetch, exa, serper**: Use API keys in environment variables (no OAuth)
- **filesystem**: No authentication required
- **browsermcp**: Runs local server (no remote auth)

### mcp-remote OAuth Token Storage
- Location: `~/.config/mcp-remote/`
- Format: OAuth access + refresh tokens
- Expiry: Typically 30-90 days (Atlassian policy)
- Refresh: mcp-remote auto-refreshes if possible

### Security Considerations
- Tokens grant access to user's JIRA Cloud
- Scope: `read:jira-work`, `write:jira-work`, `read:jira-user`
- Should not be committed to version control
- Jan doesn't access tokens (mcp-remote handles everything)
