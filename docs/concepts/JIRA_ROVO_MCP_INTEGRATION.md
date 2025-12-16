# JIRA Rovo MCP Integration Guide

**Status**: Planning Phase  
**Complexity**: Low (uses mcp-remote proxy)  
**Timeline**: 1-3 days  
**Last Updated**: December 5, 2025

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [What is JIRA Rovo MCP](#what-is-jira-rovo-mcp)
3. [Architecture Overview](#architecture-overview)
4. [Implementation Strategy (mcp-remote Proxy)](#implementation-strategy-mcp-remote-proxy)
5. [Implementation Plan](#implementation-plan)
6. [Configuration & Setup](#configuration--setup)
7. [Testing Strategy](#testing-strategy)
8. [Comparison: Rovo vs Custom Server](#comparison-rovo-vs-custom-server)
9. [Migration Path](#migration-path)
10. [Alternative: Direct OAuth Integration](#alternative-direct-oauth-integration)

---

## Executive Summary

### What Changed

Instead of building a custom JIRA MCP server from scratch, we can leverage **JIRA Rovo MCP** - Atlassian's official MCP implementation. Even better: we can use the **`mcp-remote` proxy** which handles OAuth authentication for us!

### Key Benefits

- ✅ **No server development needed** - Use Atlassian's official implementation
- ✅ **No OAuth code needed** - `mcp-remote` handles authentication
- ✅ **Always up-to-date** - Automatically gets new JIRA features
- ✅ **stdio transport** - Jan's most mature transport (same as Linear, Todoist)
- ✅ **Officially supported** - Maintained by Atlassian
- ✅ **Production-ready** - Battle-tested infrastructure
- ✅ **Works today** - No custom code required

### Timeline Reduction

- **Original plan**: 4-6 weeks (build custom MCP server)
- **OAuth approach**: 1-2 weeks (implement OAuth in Jan)
- **mcp-remote approach**: 1-3 days (just configuration!)
- **Savings**: 95% reduction in development time

### Implementation Focus

Main effort is just:
1. Add JIRA Rovo to default MCP configuration
2. Test authentication flow
3. Documentation and user guide
4. (Optional) Improve UX for OAuth in terminal

---

## What is JIRA Rovo MCP

### Overview

JIRA Rovo MCP is Atlassian's official Model Context Protocol server that provides:
- **Hosted Service**: Available at `https://mcp.atlassian.com/v1/sse`
- **SSE Transport**: Server-Sent Events protocol (already supported by Jan)
- **OAuth Authentication**: Standard OAuth 2.0 flow
- **Complete Coverage**: All JIRA APIs exposed as MCP tools
- **Multi-tenant**: Supports multiple JIRA sites per user

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Jan Desktop App                       │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Frontend (React)                                 │  │
│  │  - OAuth flow UI (consent screen)                 │  │
│  │  - Token management                               │  │
│  │  - MCP tool selection                             │  │
│  └───────────┬───────────────────────────────────────┘  │
│              │ Tauri IPC                                 │
│  ┌───────────▼───────────────────────────────────────┐  │
│  │  Backend (Rust)                                   │  │
│  │  - OAuth token storage                            │  │
│  │  - SSE client implementation                      │  │
│  │  - MCP protocol handling                          │  │
│  └───────────┬───────────────────────────────────────┘  │
└──────────────┼───────────────────────────────────────────┘
               │ HTTPS (SSE)
               │ Authorization: Bearer <oauth_token>
               │
┌──────────────▼───────────────────────────────────────────┐
│     Atlassian Infrastructure (Cloud)                     │
│  ┌─────────────────────────────────────────────────┐    │
│  │  https://mcp.atlassian.com/v1/sse               │    │
│  │  - JIRA Rovo MCP Server                         │    │
│  │  - Tool implementations                          │    │
│  │  - JIRA API client                              │    │
│  │  - Multi-site support                           │    │
│  └─────────────┬───────────────────────────────────┘    │
│                │                                          │
│  ┌─────────────▼───────────────────────────────────┐    │
│  │  JIRA Cloud APIs                                │    │
│  │  - Your JIRA site(s)                            │    │
│  │  - Projects, issues, sprints, etc.              │    │
│  └─────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

### Advantages Over Custom Implementation

| Aspect | Custom MCP Server | JIRA Rovo MCP |
|--------|-------------------|---------------|
| **Development Time** | 4-6 weeks | 1-2 weeks |
| **Maintenance** | Team responsibility | Atlassian maintains |
| **Updates** | Manual updates needed | Automatic with JIRA releases |
| **API Coverage** | Need to implement each tool | All JIRA APIs available |
| **Infrastructure** | Self-hosted (via npx) | Atlassian-hosted |
| **Reliability** | Depends on our code | Enterprise SLA |
| **Security** | Our implementation | Atlassian security team |
| **OAuth Support** | Need to implement | Built-in |
| **Multi-site** | Need to add | Supported natively |

---

## Architecture Overview

### The mcp-remote Proxy Approach

Jan can connect to JIRA Rovo MCP using the **`mcp-remote`** proxy, which acts as a bridge between Jan's stdio transport and Atlassian's SSE endpoint:

```
┌─────────────────────────────────────────────────────────┐
│                    Jan Desktop App                       │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Frontend (React)                                 │  │
│  │  - Standard MCP UI                                │  │
│  │  - No OAuth-specific code needed!                 │  │
│  └───────────┬───────────────────────────────────────┘  │
│              │ Tauri IPC                                 │
│  ┌───────────▼───────────────────────────────────────┐  │
│  │  Backend (Rust)                                   │  │
│  │  - Existing MCP stdio support                     │  │
│  │  - No changes needed!                             │  │
│  └───────────┬───────────────────────────────────────┘  │
└──────────────┼───────────────────────────────────────────┘
               │ stdio (stdin/stdout)
               │ npx -y mcp-remote https://mcp.atlassian.com/v1/sse
               │
┌──────────────▼───────────────────────────────────────────┐
│     mcp-remote Proxy (npx package)                       │
│  ┌─────────────────────────────────────────────────┐    │
│  │  - Translates stdio ↔ SSE                       │    │
│  │  - Handles OAuth flow in terminal               │    │
│  │  - Manages token refresh                        │    │
│  │  - Stores tokens locally                        │    │
│  └─────────────┬───────────────────────────────────┘    │
└────────────────┼─────────────────────────────────────────┘
                 │ HTTPS (SSE)
                 │ Authorization: Bearer <oauth_token>
                 │
┌────────────────▼─────────────────────────────────────────┐
│     Atlassian Infrastructure (Cloud)                     │
│  ┌─────────────────────────────────────────────────┐    │
│  │  https://mcp.atlassian.com/v1/sse               │    │
│  │  - JIRA Rovo MCP Server                         │    │
│  │  - Tool implementations                          │    │
│  │  - JIRA API client                              │    │
│  └─────────────┬───────────────────────────────────┘    │
│                │                                          │
│  ┌─────────────▼───────────────────────────────────┐    │
│  │  JIRA Cloud APIs                                │    │
│  │  - Your JIRA site(s)                            │    │
│  └─────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────┘
```

### Why This Is Brilliant

1. **No Jan code changes needed** - stdio transport already works perfectly
2. **No OAuth implementation** - `mcp-remote` handles the entire OAuth flow
3. **No SSE client** - `mcp-remote` does the protocol translation
4. **Proven pattern** - Same approach as other remote MCPs

### How mcp-remote Works

The `mcp-remote` package:
- Spawns as a child process via `npx -y mcp-remote <url>`
- Communicates with Jan via **stdio** (same as local MCP servers)
- Connects to remote SSE endpoint
- Handles OAuth authentication in the terminal (opens browser)
- Stores OAuth tokens securely
- Auto-refreshes tokens when needed
- Translates messages between stdio and SSE protocols

---

## Implementation Strategy (mcp-remote Proxy)

### Configuration-Only Approach

Since `mcp-remote` handles all the complexity, we just need to add JIRA Rovo to Jan's MCP configuration!

### Step 1: Add to Default MCP Config

```rust
// src-tauri/src/core/mcp/constants.rs
pub const DEFAULT_MCP_CONFIG: &str = r#"{
  "mcpServers": {
    // ... existing servers ...
    "jira-rovo": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "https://mcp.atlassian.com/v1/sse"],
      "env": {},
      "active": false,
      "alwaysAllow": [],
      "metadata": {
        "name": "JIRA (Atlassian Rovo)",
        "description": "Official JIRA integration via Atlassian's MCP server. Manage issues, projects, sprints, and more.",
        "category": "Productivity",
        "official": true,
        "requiresAuth": true,
        "authType": "oauth-terminal",
        "icon": "https://www.atlassian.com/favicon.ico",
        "website": "https://www.atlassian.com/software/jira",
        "documentation": "https://jan.ai/docs/mcp-examples/productivity/jira-rovo"
      }
    }
  }
}"#;
```

### Step 2: Update Migration (if needed)

```rust
// src-tauri/src/core/setup.rs
// Add migration to v3 if changing existing config
fn migrate_mcp_config_v2_to_v3(config: &mut Value) -> Result<(), String> {
    // Add JIRA Rovo server if not present
    if let Some(servers) = config.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
        if !servers.contains_key("jira-rovo") {
            servers.insert(
                "jira-rovo".to_string(),
                serde_json::json!({
                    "command": "npx",
                    "args": ["-y", "mcp-remote", "https://mcp.atlassian.com/v1/sse"],
                    "env": {},
                    "active": false,
                    "alwaysAllow": []
                })
            );
        }
    }
    Ok(())
}
```

### Step 3: User Setup Flow

When user enables JIRA Rovo MCP:

1. **Enable in Settings**
   - User toggles "JIRA (Atlassian Rovo)" to active
   - Jan starts the MCP server: `npx -y mcp-remote https://mcp.atlassian.com/v1/sse`

2. **OAuth Authentication (First Time)**
   - `mcp-remote` detects no stored tokens
   - Opens browser to Atlassian OAuth page
   - User logs in and approves permissions
   - `mcp-remote` stores tokens locally
   - Connection established

3. **Subsequent Launches**
   - `mcp-remote` finds stored tokens
   - Auto-refreshes if expired
   - Connection established immediately

### Step 4: Optional UX Improvements

**Show Authentication Status** (optional enhancement):

```typescript
// web-app/src/hooks/useMCPServers.ts
interface MCPServer {
  // ... existing fields ...
  authStatus?: 'authenticated' | 'pending' | 'required' | 'error';
  authMessage?: string;
}
```

Monitor stdout/stderr from `mcp-remote` process for authentication prompts and show in UI.

**Terminal Output Forwarding**:

Currently, `mcp-remote` OAuth prompts appear only in backend logs. We could:
- Show a notification: "JIRA authentication required - check your browser"
- Display terminal output in a modal during first setup
- Add a "Re-authenticate" button if auth fails

---

## Implementation Plan

### Day 1: Configuration & Testing

**Morning: Add Configuration**
- [ ] Add `jira-rovo` to `DEFAULT_MCP_CONFIG` in constants.rs
- [ ] Add metadata (icon, description, category)
- [ ] Test configuration loads correctly
- [ ] Verify `npx -y mcp-remote` works on development machine

**Afternoon: Integration Testing**
- [ ] Enable JIRA Rovo MCP in Jan
- [ ] Verify `mcp-remote` process spawns
- [ ] Complete OAuth flow manually
- [ ] Verify tools are discovered
- [ ] Test tool calling (create issue, search, etc.)

### Day 2: UX & Documentation

**Morning: User Experience**
- [ ] Test on all platforms (macOS, Windows, Linux)
- [ ] Document authentication flow with screenshots
- [ ] Add troubleshooting tips
- [ ] (Optional) Add UI hints for OAuth prompts

**Afternoon: Documentation**
- [ ] Create user guide (`.mdx` file)
- [ ] Add to MCP examples documentation
- [ ] Create setup video/GIF
- [ ] Update CONTRIBUTING.md if needed

### Day 3: Testing & Release

**Morning: Comprehensive Testing**
- [ ] Test multi-site JIRA support
- [ ] Test token refresh (wait for expiry or manually invalidate)
- [ ] Test error handling (network issues, denied auth, etc.)
- [ ] Test alongside other MCP servers
- [ ] Performance testing (tool response times)

**Afternoon: Release Preparation**
- [ ] Create release notes
- [ ] Update changelog
- [ ] Beta test with community members
- [ ] Prepare announcement
- [ ] Merge and release

---

## Configuration & Setup

### MCP Server Configuration

Add to `src-tauri/src/core/mcp/constants.rs`:

```json
{
  "mcpServers": {
    "jira-rovo": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "https://mcp.atlassian.com/v1/sse"],
      "env": {},
      "active": false,
      "alwaysAllow": [],
      "metadata": {
        "name": "JIRA (Atlassian Rovo)",
        "description": "Official JIRA integration. Manage issues, projects, sprints, and more with Atlassian's AI-powered Rovo MCP.",
        "category": "Productivity",
        "official": true,
        "requiresAuth": true,
        "authType": "oauth-terminal",
        "icon": "https://cdn.jsdelivr.net/gh/devicons/devicon/icons/jira/jira-original.svg",
        "documentation": "https://jan.ai/docs/mcp-examples/productivity/jira-rovo"
      }
    }
  }
}
```

### Alternative: Version Pinning (Optional)

If there are version issues with `mcp-remote`, can pin to specific version:

```json
{
  "args": ["-y", "mcp-remote@0.1.13", "https://mcp.atlassian.com/v1/sse"]
}
```

### User Setup Flow (Detailed)

#### Step 1: Enable MCP Server

User opens Jan Settings:
1. Navigate to **Settings** > **MCP Servers**
2. Find **"JIRA (Atlassian Rovo)"** in the list
3. Toggle **Active** switch to ON
4. Jan spawns: `npx -y mcp-remote https://mcp.atlassian.com/v1/sse`

#### Step 2: First-Time Authentication

`mcp-remote` detects no stored tokens and initiates OAuth:

**In Terminal** (backend logs):
```
[INFO] mcp-remote: No authentication found
[INFO] mcp-remote: Opening browser for OAuth authentication...
[INFO] mcp-remote: Please complete authentication in your browser
```

**In Browser**:
- Atlassian login page opens
- User logs in (if not already)
- OAuth consent screen appears:
  ```
  Jan wants to access your JIRA
  
  This will allow Jan to:
  • Read and write issues
  • Read projects and boards
  • Read user information
  
  [Allow]  [Deny]
  ```
- User clicks **Allow**

**Back in Terminal**:
```
[INFO] mcp-remote: Authentication successful
[INFO] mcp-remote: Connected to https://mcp.atlassian.com/v1/sse
[INFO] mcp-remote: Tools available: 36
```

**In Jan UI**:
- MCP server shows **Connected ✓**
- Tools appear in chat interface
- User can start using JIRA tools

#### Step 3: Subsequent Uses

Next time Jan starts:
1. `mcp-remote` finds stored tokens (in `~/.mcp-remote/` or similar)
2. Validates token (auto-refreshes if expired)
3. Connects immediately - no browser required
4. Tools available instantly

### Troubleshooting Setup

**Issue: OAuth browser doesn't open**
- Solution: `mcp-remote` will output a URL in logs. User can manually copy and open in browser.

**Issue: "Authentication failed"**
- Solution: Delete cached tokens and retry:
  ```bash
  rm -rf ~/.mcp-remote/atlassian  # Or wherever mcp-remote stores tokens
  ```

**Issue: "Network error"**
- Solution: Check internet connection, verify `https://mcp.atlassian.com` is accessible

**Issue: Multiple JIRA sites**
- Solution: `mcp-remote` should prompt which site to use during OAuth flow

---

## Testing Strategy

### Manual Testing Checklist

#### MCP Server Activation
- [ ] Can find "JIRA (Atlassian Rovo)" in MCP servers list
- [ ] Toggle active successfully
- [ ] `npx -y mcp-remote` process spawns correctly
- [ ] Process shows in task manager / activity monitor

#### First-Time OAuth
- [ ] Browser opens automatically to Atlassian OAuth page
- [ ] Can log in with Atlassian account
- [ ] OAuth consent screen shows correct permissions
- [ ] Can approve permissions
- [ ] Connection established after approval
- [ ] Success message appears in logs
- [ ] Tools discovered (36+ tools expected)

#### Tool Discovery & Calling
- [ ] Tools appear in Jan chat interface
- [ ] Can search for JIRA-related tools
- [ ] Tool descriptions are clear
- [ ] Can call tool: `jira-get-issues`
- [ ] Can call tool: `jira-create-issue`
- [ ] Can call tool: `jira-search-jql`
- [ ] Tool responses are formatted correctly
- [ ] Error handling works (invalid parameters)

#### Token Persistence
- [ ] Close and restart Jan
- [ ] JIRA MCP auto-connects (no browser prompt)
- [ ] Tools available immediately
- [ ] No authentication errors in logs

#### Token Refresh
- [ ] Wait for token expiry (or manually invalidate)
- [ ] `mcp-remote` auto-refreshes token
- [ ] No interruption to service
- [ ] No browser prompt required

#### Error Handling
- [ ] Deny OAuth consent → Shows error message
- [ ] Network error during OAuth → Graceful error
- [ ] Kill `mcp-remote` process → Jan detects disconnection
- [ ] Invalid JQL query → Error message displayed
- [ ] JIRA site unreachable → Timeout handled gracefully

#### Multi-Platform Testing
- [ ] **macOS**: All features work
- [ ] **Windows**: All features work
- [ ] **Linux**: All features work
- [ ] Browser opening works on all platforms
- [ ] Token storage location correct for each OS

#### Multi-Site Support
- [ ] OAuth flow prompts for site selection (if multiple sites)
- [ ] Can select correct JIRA site
- [ ] Tools work with selected site
- [ ] Can change site (re-authenticate)

### Automated Testing

#### Integration Tests (Rust)

```rust
// src-tauri/src/core/mcp/tests.rs
#[cfg(test)]
mod jira_rovo_tests {
    use super::*;
    
    #[tokio::test]
    async fn test_jira_rovo_config_loads() {
        let config = load_default_mcp_config();
        assert!(config.get("mcpServers")
            .and_then(|s| s.get("jira-rovo"))
            .is_some());
    }
    
    #[tokio::test]
    async fn test_jira_rovo_command_correct() {
        let config = load_default_mcp_config();
        let jira = config.get("mcpServers")
            .and_then(|s| s.get("jira-rovo"))
            .unwrap();
        
        assert_eq!(jira.get("command").and_then(|c| c.as_str()), Some("npx"));
        assert_eq!(
            jira.get("args").and_then(|a| a.as_array()),
            Some(&vec![
                Value::String("-y".to_string()),
                Value::String("mcp-remote".to_string()),
                Value::String("https://mcp.atlassian.com/v1/sse".to_string())
            ])
        );
    }
    
    #[tokio::test]
    async fn test_mcp_remote_available() {
        // Test that npx can find mcp-remote
        let output = std::process::Command::new("npx")
            .args(&["-y", "mcp-remote", "--version"])
            .output()
            .expect("Failed to run npx");
        
        assert!(output.status.success());
    }
}
```

#### E2E Tests

```markdown
### JIRA Rovo MCP E2E Tests

#### Setup
1. Enable JIRA Rovo MCP in Settings
2. Complete OAuth authentication
3. Verify connection successful

#### Test Cases

**TC1: List Projects**
- Input: "List all my JIRA projects"
- Expected: Tool call to `jira-list-projects`, returns project list
- Verify: Project names and keys displayed

**TC2: Create Issue**
- Input: "Create a bug in project DEMO: Login button not working"
- Expected: Tool call to `jira-create-issue`
- Verify: Issue created with correct summary and type

**TC3: Search with JQL**
- Input: "Find all issues assigned to me that are in progress"
- Expected: Tool call to `jira-search-jql` with correct JQL
- Verify: Results match criteria

**TC4: Update Issue**
- Input: "Move DEMO-123 to Done"
- Expected: Tool call to `jira-update-issue-status`
- Verify: Issue status updated in JIRA

**TC5: Complex Workflow**
- Input: "Create sprint report for team Alpha"
- Expected: Multiple tool calls (get sprint, get issues, format report)
- Verify: Comprehensive report generated

**TC6: Error Handling**
- Input: "Create issue in non-existent project XYZ"
- Expected: Error message from JIRA API
- Verify: User-friendly error displayed
```

### Performance Testing

#### Response Time Benchmarks
- Tool discovery: < 3 seconds
- Simple tool call (get issue): < 2 seconds
- Complex tool call (JQL search): < 5 seconds
- Token refresh: < 1 second (transparent to user)

#### Resource Usage
- Memory: < 100MB for mcp-remote process
- CPU: < 5% during idle, < 20% during tool calls
- Network: Minimal (SSE long-polling + API calls)

### Security Testing

- [ ] **Token Storage**: Verify tokens not in plaintext
- [ ] **Log Sanitization**: No tokens in console logs
- [ ] **Process Isolation**: mcp-remote runs as separate process
- [ ] **HTTPS**: All communication over TLS
- [ ] **Token Scopes**: OAuth only grants requested permissions
- [ ] **Token Revocation**: Can revoke from Atlassian dashboard

---

## Comparison: Rovo vs Custom Server

### Feature Matrix

| Feature | Custom MCP Server | JIRA Rovo MCP (mcp-remote) |
|---------|-------------------|----------------------------|
| **Development Time** | 4-6 weeks | 1-3 days |
| **Code Changes** | Extensive (server + Jan) | Minimal (config only) |
| **Maintenance Burden** | High (team owns) | None (Atlassian + mcp-remote) |
| **API Coverage** | Need to implement | Complete (all JIRA) |
| **Updates** | Manual | Automatic |
| **Authentication** | API Token (manual) | OAuth 2.0 (handled by mcp-remote) |
| **Multi-Site Support** | Need to add | Built-in |
| **Hosting** | Self (via npx) | Atlassian Cloud |
| **Reliability** | Depends on code | Enterprise SLA |
| **Security** | Team responsibility | Atlassian + mcp-remote |
| **Transport** | stdio | stdio (via mcp-remote proxy) |
| **Setup Complexity** | Medium (API token) | Low (browser OAuth) |
| **Cost** | $0 (self-hosted) | $0 (free tier) |
| **Customization** | Full control | Limited to API |
| **Offline Support** | Yes | No |
| **Dependencies** | Node.js | npx (Node.js) |

### When to Choose Custom Server

Use a custom MCP server if you need:
- **Offline functionality** - Work without internet
- **Custom logic** - Transform data, add business rules beyond JIRA API
- **Third-party APIs** - Combine JIRA with non-Atlassian services
- **Full control** - Own the entire stack
- **JIRA Data Center** - On-premise installation without internet access
- **Custom tools** - Tools not available in JIRA API

### When to Choose Rovo MCP (Recommended)

Use JIRA Rovo MCP if you want:
- **Faster time to market** - 95% less development time
- **Zero maintenance** - Atlassian handles server updates, mcp-remote handles proxy
- **Complete API coverage** - All JIRA features available day 1
- **Enterprise reliability** - Atlassian SLA
- **OAuth security** - More secure than API tokens, auto-refresh
- **Cloud-first** - Using JIRA Cloud
- **No code approach** - Configuration only

### Decision Matrix

```
┌─────────────────────────────────────────────────────────┐
│                  JIRA MCP Decision Tree                  │
└─────────────────────────────────────────────────────────┘

Are you using JIRA Cloud?
  │
  ├─ YES → Do you need offline access?
  │   │
  │   ├─ NO  → **Use JIRA Rovo MCP** ✓ (Recommended)
  │   │
  │   └─ YES → Do you need custom business logic?
  │       │
  │       ├─ NO  → **Use JIRA Rovo MCP** (connectivity permitting)
  │       │
  │       └─ YES → **Build Custom MCP Server**
  │
  └─ NO (JIRA Data Center) → Is it internet-connected?
      │
      ├─ YES → Can you expose APIs?
      │   │
      │   ├─ YES → **Build Custom MCP Server** (use Data Center APIs)
      │   │
      │   └─ NO  → **Build Custom MCP Server** (local only)
      │
      └─ NO → **Build Custom MCP Server** (offline only)
```

### Hybrid Approach (Advanced)

You can support **both** implementations and let users choose:

```json
{
  "mcpServers": {
    "jira-rovo": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "https://mcp.atlassian.com/v1/sse"],
      "metadata": {
        "name": "JIRA (Official - Cloud)",
        "description": "Atlassian's official JIRA integration",
        "recommended": true,
        "requirements": ["JIRA Cloud", "Internet connection"]
      }
    },
    "jira-custom": {
      "command": "npx",
      "args": ["-y", "@janhq/jira-mcp-server"],
      "metadata": {
        "name": "JIRA (Custom - Advanced)",
        "description": "Custom JIRA integration with offline support",
        "recommended": false,
        "requirements": ["API Token", "Custom configuration"]
      }
    }
  }
}
```

**Recommendation**: Start with Rovo MCP. Only build custom if you have specific requirements that Rovo can't meet.

---

## Migration Path

### For Users (After Implementation)

**New Users - Getting Started**:

1. **Enable JIRA Rovo MCP**
   - Open Jan Settings > MCP Servers
   - Toggle "JIRA (Atlassian Rovo)" to active
   - Wait for `mcp-remote` to start

2. **Complete OAuth**
   - Browser opens automatically
   - Log in to Atlassian account
   - Approve permissions
   - Return to Jan - tools ready!

3. **Start Using**
   - No configuration needed
   - No API tokens to manage
   - All JIRA sites accessible

**If currently using API Token-based JIRA integration**:

1. **Note Current Settings** (optional backup)
   - JIRA URL
   - API token (can delete after migration)

2. **Switch to Rovo MCP**
   - Disable any existing JIRA MCP
   - Enable "JIRA (Atlassian Rovo)"
   - Complete OAuth flow
   - Verify tools work

3. **Benefits You'll Gain**
   - ✅ No more API token rotation
   - ✅ More secure OAuth scopes
   - ✅ Access all your JIRA sites from one connection
   - ✅ Automatic updates with new JIRA features
   - ✅ Can revoke access from Atlassian dashboard

### For Developers (Implementation Strategy)

**Phase 1: Configuration** (Day 1 - Morning)
- Add `jira-rovo` to `DEFAULT_MCP_CONFIG`
- Add metadata (icon, description, category)
- Test configuration loads
- Verify `npx -y mcp-remote` works locally

**Phase 2: Testing** (Day 1 - Afternoon)
- Enable JIRA Rovo MCP in Jan
- Complete OAuth flow
- Verify tool discovery
- Test 5-10 common tools (create issue, search, update, etc.)
- Test on macOS (or primary development platform)

**Phase 3: Multi-Platform Testing** (Day 2 - Morning)
- Test on Windows
- Test on Linux
- Verify OAuth flow on all platforms
- Check token persistence across restarts
- Test token refresh mechanism

**Phase 4: Documentation** (Day 2 - Afternoon)
- Create user guide with screenshots
- Document setup steps
- Add troubleshooting section
- Update MCP examples docs
- Create demo video/GIF

**Phase 5: Beta Testing** (Day 3 - Optional)
- Release to beta testers
- Gather feedback on OAuth flow
- Fix any platform-specific issues
- Performance testing
- Security review

**Phase 6: Release** (Day 3 - Afternoon)
- Update changelog
- Prepare release notes
- Announce in community
- Merge to main
- Deploy to production

### Rollback Plan

If issues arise with Rovo MCP:

1. **Immediate**: Disable by default in config (`"active": false`)
2. **Short-term**: Document manual activation for advanced users
3. **Long-term**: Build custom MCP server if Rovo proves unreliable

---

## Alternative: Direct OAuth Integration

### When You Might Need This

If `mcp-remote` doesn't meet your needs (e.g., UX concerns with terminal OAuth), you can implement direct OAuth in Jan. This requires more work but gives you full control.

### What You'd Build

1. **OAuth Infrastructure** (~1 week)
   - OAuth manager in Rust
   - Deep link handler for callbacks
   - Token storage and refresh logic
   - UI for OAuth consent

2. **SSE Client Modifications** (~2-3 days)
   - Add Bearer token to SSE headers
   - Handle 401 errors (trigger refresh)
   - Connection recovery after token refresh

3. **Frontend UI** (~2-3 days)
   - OAuth connection button
   - Authentication status display
   - Token management interface

**Total Time**: ~1.5-2 weeks (vs. 1-3 days with mcp-remote)

### Code Reference

The original version of this document (in git history) contains complete OAuth implementation code for:
- Rust OAuth manager with PKCE
- Deep link registration
- Token refresh logic
- Frontend React components

Search git history for "OAuth Implementation Strategy" section if you need it.

### Recommendation

**Start with mcp-remote approach**. The terminal-based OAuth is battle-tested and works well. Only invest in custom OAuth if:
- Users report UX issues with terminal auth
- You need better OAuth UI integration
- You want to add custom OAuth features
- mcp-remote has reliability issues

---

## Appendix

### mcp-remote Package Info

- **Package**: `mcp-remote` on npm
- **Purpose**: Proxy between stdio MCP clients and SSE MCP servers
- **Handles**: OAuth flow, token storage, protocol translation
- **Works with**: Any SSE-based MCP server
- **Installation**: Automatic via `npx -y mcp-remote`
- **Version pinning**: Use `mcp-remote@0.1.13` if latest has issues

### Atlassian OAuth Resources

- **OAuth 2.0 (3LO) Guide**: https://developer.atlassian.com/cloud/jira/platform/oauth-2-3lo-apps/
- **Scopes Reference**: https://developer.atlassian.com/cloud/jira/platform/scopes-for-oauth-2-3LO-and-forge-apps/
- **Rovo MCP Documentation**: https://developer.atlassian.com/cloud/rovo/mcp/
- **API Tokens (fallback)**: https://support.atlassian.com/atlassian-account/docs/manage-api-tokens-for-your-atlassian-account/

### Required OAuth Scopes for JIRA

mcp-remote automatically requests these scopes:

```
read:jira-work          # Read projects, issues, boards, sprints
write:jira-work         # Create, update, delete issues
read:jira-user          # Read user information
offline_access          # Get refresh token
```

### Token Storage Locations

`mcp-remote` stores tokens in standard locations:

- **macOS**: `~/.config/mcp-remote/` or `~/Library/Application Support/mcp-remote/`
- **Windows**: `%APPDATA%\mcp-remote\`
- **Linux**: `~/.config/mcp-remote/`

Tokens are typically stored encrypted or with restricted file permissions.

### Debugging Tips

**Enable mcp-remote debug logs**:
```bash
# Set environment variable before starting Jan
DEBUG=mcp-remote:* make dev
```

**Manually test mcp-remote**:
```bash
# Run mcp-remote standalone
npx -y mcp-remote https://mcp.atlassian.com/v1/sse

# Should output:
# [INFO] Opening browser for authentication...
# [INFO] Authentication successful
# [INFO] Connected to https://mcp.atlassian.com/v1/sse
# [INFO] Available tools: 36
```

**Clear cached tokens** (force re-authentication):
```bash
# macOS/Linux
rm -rf ~/.config/mcp-remote/atlassian

# Windows
rd /s /q %APPDATA%\mcp-remote\atlassian
```

### Security Considerations

**mcp-remote Security**:
- Open source - can audit code
- Maintained by MCP community
- Follows OAuth best practices (PKCE)
- Stores tokens with appropriate permissions
- Auto-refresh prevents token exposure

**Jan Security**:
- No token handling in Jan code
- mcp-remote process isolated
- Communication via stdio (no network exposure)
- Can kill mcp-remote process to revoke access

**Atlassian Security**:
- OAuth scopes limit access
- Can revoke from Atlassian dashboard
- Audit logs for API usage
- SOC 2 / ISO 27001 certified infrastructure

---

## Next Steps

### Immediate Actions (Start Today!)

1. **Verify mcp-remote Works**
   ```bash
   npx -y mcp-remote --version
   npx -y mcp-remote https://mcp.atlassian.com/v1/sse
   ```
   - Test OAuth flow manually
   - Verify tools discovered
   - Test a few tool calls

2. **Add to Jan Configuration**
   - Edit `src-tauri/src/core/mcp/constants.rs`
   - Add `jira-rovo` server config
   - Test in development build

3. **Create User Documentation**
   - Setup guide with screenshots
   - Troubleshooting section
   - Video walkthrough

4. **Test on All Platforms**
   - macOS, Windows, Linux
   - Different JIRA sites
   - Token persistence
   - Error scenarios

### Success Criteria

- [ ] Can enable JIRA Rovo MCP in settings
- [ ] OAuth flow completes successfully
- [ ] Tools discovered (30+ expected)
- [ ] Can create JIRA issue from chat
- [ ] Can search issues with JQL
- [ ] Connection persists across restarts
- [ ] Works on macOS, Windows, Linux
- [ ] Documentation clear for end users

---

**Document Version**: 2.0 (mcp-remote approach)  
**Last Updated**: December 5, 2025  
**Status**: Ready for Implementation  
**Recommendation**: **Proceed with mcp-remote + Rovo MCP** - Fastest path to production  
**Estimated Timeline**: 1-3 days for full implementation and testing
