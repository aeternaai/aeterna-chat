# MCP OAuth Detection Implementation

## Overview
Jan now automatically detects and facilitates OAuth authentication for MCP servers that use `mcp-remote` or similar tools requiring browser-based authentication.

## Problem Solved
When connecting to MCP servers like JIRA ROVO that use `mcp-remote` (a subprocess-based proxy for remote MCP servers), Jan was:
1. Spawning the mcp-remote process
2. Immediately attempting MCP protocol handshake
3. Failing health checks because mcp-remote was waiting for OAuth
4. Killing and restarting the server, interrupting the OAuth flow

## Solution
Jan now monitors stderr output from subprocess-based MCP servers to:
1. Detect OAuth authentication prompts
2. Automatically open the authorization URL in the user's browser
3. Wait for authentication completion
4. Then proceed with MCP protocol handshake and health checks

## Implementation Details

### Detection Pattern
Jan watches for this stderr output pattern from `mcp-remote`:
```
[PID] Please authorize this client by visiting:
https://mcp.atlassian.com/v1/authorize?...
```

### Automatic Browser Opening
When detected, Jan:
1. Extracts the OAuth URL from stderr
2. Opens it in the default browser using the `open` crate
3. Emits a `mcp_oauth_required` event to the frontend with server name and URL

### Connection Confirmation
Jan waits for this confirmation in stderr:
```
[PID] Connected to remote server using SSEClientTransport
```
or
```
[PID] Proxy established successfully
```

When detected, Jan emits a `mcp_authenticated` event to the frontend.

### Code Changes

#### src-tauri/Cargo.toml
Added dependency:
```toml
open = "5.0"
```

#### src-tauri/src/core/mcp/helpers.rs
Modified subprocess spawning to:
1. Capture stderr stream
2. Spawn background task to monitor stderr line-by-line
3. Detect auth prompts and open browser
4. Log all stderr output for debugging
5. Emit events to frontend for UI updates

Key sections:
- Lines ~860-920: Subprocess spawn with stderr monitoring
- Background tokio task for continuous stderr reading
- Pattern matching for OAuth prompts and connection confirmation

### Events Emitted

#### `mcp_oauth_required`
```json
{
  "server": "jira-rovo",
  "url": "https://mcp.atlassian.com/v1/authorize?..."
}
```

#### `mcp_authenticated`
```json
{
  "server": "jira-rovo"
}
```

## Frontend Integration (Future Work)
The frontend can listen for these events to:
1. Show a notification when OAuth is required
2. Display the auth URL as a fallback if browser fails to open
3. Show connection status when authentication succeeds
4. Update UI state to indicate auth in progress

Example:
```typescript
import { listen } from '@tauri-apps/api/event'

listen('mcp_oauth_required', (event) => {
  const { server, url } = event.payload
  console.log(`OAuth required for ${server}: ${url}`)
  // Show notification or modal
})

listen('mcp_authenticated', (event) => {
  const { server } = event.payload
  console.log(`${server} authenticated successfully`)
  // Update UI state
})
```

## Testing
To test the OAuth detection:

1. Configure JIRA ROVO MCP server in Jan
2. Activate the server
3. Jan should automatically:
   - Detect the OAuth requirement
   - Open your browser to Atlassian's OAuth page
   - Wait for you to authenticate
   - Connect successfully after authentication

## Technical Notes

### Why This Approach?
- **mcp-remote** is a proxy tool that handles OAuth internally
- It's not Jan's responsibility to manage tokens for mcp-remote
- mcp-remote already has its own OAuth flow implementation
- Jan just needs to detect when auth is needed and help the user complete it

### Subprocess Transport
mcp-remote uses stdin/stdout for MCP protocol communication and stderr for diagnostic logs. This is why we monitor stderr for auth prompts rather than trying to inject tokens.

### Timing
Jan gives the server 1 second to start and potentially prompt for auth before attempting the MCP handshake. This small delay allows time for the subprocess to initialize and output any auth requirements.

## Related Files
- `src-tauri/src/core/mcp/helpers.rs` - Main implementation
- `src-tauri/src/core/mcp/oauth.rs` - OAuth token management (for direct SSE/HTTP, not used for mcp-remote)
- `src-tauri/Cargo.toml` - Dependencies
- `MCP_OAUTH_DETECTION.md` - This documentation

## Removed Code
The following incorrect implementations were removed:
1. OAuth token retrieval for subprocess transport (lines ~780-800)
2. OAUTH_TOKEN environment variable injection (lines ~870-880)

These were based on the wrong assumption that tokens should be passed to mcp-remote. The correct approach is to let mcp-remote manage its own OAuth and just facilitate the browser-based flow.
