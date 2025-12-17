# JIRA Rovo MCP Integration - Fix Complete ✅

## Problem Identified

The `jira-rovo` MCP server was stuck in an infinite restart loop with the error:
```
Transport closed
```

## Root Cause

Through wrapper script debugging, we discovered that `mcp-remote` was using the **`http-first` transport strategy** by default, which caused it to:
1. Try HTTP transport first → Failed with `HTTP 404: Missing sessionId parameter`
2. Before falling back to SSE, the connection would close
3. Jan's health check would fail → trigger restart → infinite loop

### Debug Log Evidence
```
[31301] Using transport strategy: http-first
[31301] Received error: Error POSTing to endpoint (HTTP 404): Missing sessionId parameter
```

## Solution Implemented

Force `mcp-remote` to use **SSE-only transport** to bypass the HTTP attempt:

### Updated Configuration

**Source Code** (`src-tauri/src/core/mcp/constants.rs`):
```rust
"jira-rovo": {
  "command": "npx",
  "args": ["-y", "mcp-remote", "https://mcp.atlassian.com/v1/sse", "--transport", "sse-only"],
  "env": {},
  "active": false,
  "official": true
}
```

**Runtime Config** (`~/Library/Application Support/Jan/data/mcp_config.json`):
```json
{
  "mcpServers": {
    "jira-rovo": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "https://mcp.atlassian.com/v1/sse", "--transport", "sse-only"],
      "env": {},
      "active": true,
      "official": true
    }
  }
}
```

### Key Learnings

1. **Argument Order Matters**: `mcp-remote` requires URL *before* flags
   - ✅ Correct: `npx mcp-remote https://... --transport sse-only`
   - ❌ Wrong: `npx mcp-remote --transport sse-only https://...`

2. **Transport Strategy Values**:
   - `sse-only` (what we use)
   - `http-only`
   - `sse-first` (default)
   - `http-first`

3. **OAuth Flow**: The browser-based OAuth works perfectly from Jan-spawned processes - the issue was NOT authentication

## Verification

Manual test confirmed successful connection:
```bash
npx -y mcp-remote https://mcp.atlassian.com/v1/sse --transport sse-only

# Output:
[37678] Using transport strategy: sse-only
[37678] Connected to remote server using SSEClientTransport
[37678] Proxy established successfully between local STDIO and remote SSEClientTransport
```

## Next Steps

1. **Restart Jan** - Apply the fix (in progress)
2. **Activate jira-rovo** in Jan UI
3. **Verify Tools** - Should see 30+ JIRA tools
4. **Test End-to-End**:
   - List JIRA projects
   - Create test issue
   - Search with JQL
   - Update issue
   - Add comment

## Files Modified

1. `src-tauri/src/core/mcp/constants.rs` - Updated default config with `--transport sse-only`
2. `~/Library/Application Support/Jan/data/mcp_config.json` - Updated runtime config

## Documentation to Update

After successful testing:
1. `JIRA_ROVO_DEBUG_FINDINGS.md` - Add root cause and solution
2. `JIRA_ROVO_IMPLEMENTATION_SUMMARY.md` - Update troubleshooting section
3. `docs/src/pages/docs/desktop/mcp-examples/productivity/jira.mdx` - Add transport flag to user setup guide

## Status

✅ **Root cause identified**: HTTP-first transport failing with 404  
✅ **Solution implemented**: Force SSE-only transport  
✅ **Manual test passed**: Connection successful with `--transport sse-only`  
✅ **Configuration updated**: Both source and runtime configs  
🔄 **Jan restarting**: Applying fix...  
⏳ **Pending**: End-to-end testing of JIRA tools

---

**Date**: December 5, 2025  
**Issue**: JIRA Rovo MCP restart loop  
**Status**: Fixed - awaiting verification
