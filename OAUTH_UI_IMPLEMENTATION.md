# OAuth UI Implementation for MCP Servers

## Overview

This document describes the frontend UI implementation for OAuth 2.0 authentication in Jan's MCP server settings. This completes the OAuth integration by providing users with a visual interface to authenticate MCP servers that require OAuth.

## Implementation Date

January 2025

## Components Added

### 1. OAuth Types (`web-app/src/types/oauth.ts`)

Defines TypeScript interfaces for OAuth functionality:

```typescript
export interface OAuthConfig {
  client_id: string
  auth_url: string
  token_url: string
  scopes: string[]
  redirect_uri?: string
}

export interface OAuthToken {
  access_token: string
  refresh_token?: string
  expires_at: number
}

export interface OAuthStatus {
  authenticated: boolean
  expires_at?: number
  server_name: string
}

export interface OAuthFlowResult {
  auth_url: string
}
```

### 2. MCP Service OAuth Methods

Extended the `MCPService` interface and implementations:

**Files Modified:**
- `web-app/src/services/mcp/types.ts` - Added OAuth method signatures to interface
- `web-app/src/services/mcp/default.ts` - Added default no-op implementations
- `web-app/src/services/mcp/tauri.ts` - Added Tauri command invocations

**Methods Added:**
```typescript
async startOAuthFlow(serverName: string): Promise<OAuthFlowResult>
async getOAuthStatus(serverName: string): Promise<OAuthStatus>
async getAllOAuthStatuses(): Promise<Record<string, OAuthStatus>>
async revokeOAuthToken(serverName: string): Promise<void>
```

### 3. OAuth Hook (`web-app/src/hooks/useMCPOAuth.ts`)

Custom React hook for managing OAuth state and operations:

**Features:**
- Loads OAuth statuses for all servers on mount
- Listens to `mcp_oauth_complete` and `mcp_oauth_revoked` Tauri events
- Provides methods to start OAuth flow and revoke tokens
- Shows toast notifications for success/error states
- Manages loading state during OAuth operations
- Opens browser window for OAuth authorization

**API:**
```typescript
interface UseMCPOAuthReturn {
  oauthStatuses: Record<string, OAuthStatus>
  isLoading: boolean
  startOAuthFlow: (serverName: string) => Promise<void>
  revokeOAuthToken: (serverName: string) => Promise<void>
  refreshStatuses: () => Promise<void>
  getServerStatus: (serverName: string) => OAuthStatus | null
  isAuthenticated: (serverName: string) => boolean
}
```

### 4. UI Components in MCP Settings

Modified `web-app/src/routes/settings/mcp-servers.tsx` to add:

#### OAuth Status Badge
Shows authentication state next to "Official" badge:
- **Green badge with key icon**: "Authenticated" - Server is authenticated
- **Amber badge with key-off icon**: "Auth Required" - Server needs authentication

#### OAuth Action Buttons
Added to server card actions (before edit/delete buttons):
- **Blue key icon** (`IconKey`): Click to start OAuth flow (when not authenticated)
- **Red key-off icon** (`IconKeyOff`): Click to revoke token (when authenticated)

**Visual Design:**
- Icons are 18px size, consistent with other action buttons
- Hover effect: background changes to `main-view-fg/10`
- Loading state: opacity reduced and cursor disabled during OAuth operations
- Tooltips show action description

## User Flow

### Authenticating a Server

1. User navigates to Settings → MCP Servers
2. For servers with OAuth configured, an amber "Auth Required" badge appears
3. User clicks the blue key icon in the actions row
4. Backend generates OAuth authorization URL and starts local callback server (port 17390)
5. Browser window opens with OAuth provider's login page
6. User authenticates and grants permissions
7. OAuth provider redirects to `http://localhost:17390/callback` with authorization code
8. Backend exchanges code for access token and stores it in `mcp_oauth_tokens.json`
9. Backend emits `mcp_oauth_complete` event
10. Frontend receives event, updates status, shows success toast
11. Badge changes to green "Authenticated"
12. Blue key icon changes to red key-off icon

### Revoking Authentication

1. User clicks red key-off icon on authenticated server
2. Backend deletes token from storage
3. Backend emits `mcp_oauth_revoked` event
4. Frontend receives event, updates status, shows info toast
5. Badge changes to amber "Auth Required"
6. Red key-off icon changes to blue key icon

## Event-Driven Architecture

The UI uses Tauri events for real-time status updates:

### Events Listened To

**`mcp_oauth_complete`** - Emitted when OAuth flow succeeds
```typescript
{
  server_name: string
  authenticated: boolean
  expires_at?: number
}
```

**`mcp_oauth_revoked`** - Emitted when token is revoked
```typescript
{
  server_name: string
}
```

This allows the UI to update immediately without polling, even if the OAuth callback happens in a background process.

## Internationalization

Added translation keys to `web-app/src/locales/en/mcp-servers.json`:

```json
{
  "oauth": {
    "authenticate": "Authenticate with OAuth",
    "revoke": "Revoke OAuth Token",
    "authenticated": "Authenticated",
    "authRequired": "Authentication Required"
  }
}
```

## Configuration Example

To configure an MCP server with OAuth (e.g., JIRA ROVO):

```json
{
  "mcpServers": {
    "jira-rovo": {
      "command": "npx",
      "args": ["-y", "mcp-remote", "https://mcp.atlassian.com/v1/sse", "--transport", "sse-only"],
      "type": "sse",
      "url": "https://mcp.atlassian.com/v1/sse",
      "oauth": {
        "client_id": "your-client-id",
        "auth_url": "https://auth.atlassian.com/authorize",
        "token_url": "https://auth.atlassian.com/oauth/token",
        "scopes": ["read:jira-work", "read:confluence-content.summary", "read:jira-user"]
      },
      "active": true
    }
  }
}
```

## UI Screenshots (Conceptual)

### Before Authentication
```
┌─────────────────────────────────────────────────────┐
│ ● jira-rovo                                         │
│   [Official] [Auth Required]                        │
│                                                      │
│   Transport: SSE                                     │
│   URL: https://mcp.atlassian.com/v1/sse            │
│                                                      │
│   Actions: [🔑] [</>] [✏️] [🗑️]   [Switch: ON]    │
└─────────────────────────────────────────────────────┘
```

### After Authentication
```
┌─────────────────────────────────────────────────────┐
│ ● jira-rovo                                         │
│   [Official] [Authenticated]                        │
│                                                      │
│   Transport: SSE                                     │
│   URL: https://mcp.atlassian.com/v1/sse            │
│                                                      │
│   Actions: [🔓] [</>] [✏️] [🗑️]   [Switch: ON]    │
└─────────────────────────────────────────────────────┘
```

## Integration with Backend

The UI calls these Tauri commands (implemented in `src-tauri/src/core/mcp/commands.rs`):

1. **`start_mcp_oauth_flow`**
   - Input: `{ serverName: string }`
   - Output: `{ auth_url: string }`
   - Side effect: Starts local HTTP callback server

2. **`get_mcp_oauth_status`**
   - Input: `{ serverName: string }`
   - Output: `{ authenticated: boolean, server_name: string, expires_at?: number }`

3. **`get_all_mcp_oauth_statuses`**
   - Input: None
   - Output: `Record<string, OAuthStatus>`

4. **`revoke_mcp_oauth_token`**
   - Input: `{ serverName: string }`
   - Output: None
   - Side effect: Deletes token from storage

## Error Handling

The UI handles errors gracefully:

- **Network errors**: Shows error toast with message
- **Invalid OAuth config**: Backend returns error, UI shows toast
- **Token expiry**: Backend automatically refreshes token 5 minutes before expiry
- **Manual refresh needed**: If auto-refresh fails, user can re-authenticate via UI

## Testing Checklist

- [ ] OAuth badge appears for servers with `oauth` config
- [ ] Badge shows "Auth Required" when not authenticated
- [ ] Clicking key icon opens browser with correct OAuth URL
- [ ] After OAuth callback, badge updates to "Authenticated"
- [ ] Success toast appears after authentication
- [ ] Key icon changes from blue to red after authentication
- [ ] Clicking key-off icon revokes token
- [ ] Badge updates back to "Auth Required" after revocation
- [ ] Info toast appears after revocation
- [ ] OAuth status persists across app restarts
- [ ] Multiple servers can have independent OAuth states
- [ ] Loading state prevents double-clicking during OAuth flow

## Future Enhancements

### Potential Improvements

1. **Token Expiry Display**
   - Show "Expires in X days/hours" in badge tooltip
   - Visual warning when token is near expiry

2. **OAuth Configuration UI**
   - Add form fields for OAuth config in Add/Edit Server dialog
   - Validate OAuth config before saving

3. **Advanced OAuth Features**
   - Support for PKCE (Proof Key for Code Exchange)
   - Support for refresh token rotation
   - Support for custom redirect URIs

4. **Multi-Step OAuth**
   - Support for OAuth providers requiring additional consent screens
   - Handle device flow for headless environments

5. **OAuth Provider Presets**
   - Built-in templates for common providers (Atlassian, Google, GitHub)
   - Auto-fill common OAuth endpoints

## Related Documentation

- **Backend OAuth Implementation**: See `OAUTH_IMPLEMENTATION_COMPLETE.md`
- **MCP Configuration**: See `CONTRIBUTING.md` - MCP Integration section
- **Tauri Events**: See `web-app/src/types/events.ts`
- **Service Hub Pattern**: See `web-app/src/hooks/useServiceHub.ts`

## Code Locations

| Component | File Path |
|-----------|-----------|
| OAuth Types | `web-app/src/types/oauth.ts` |
| OAuth Hook | `web-app/src/hooks/useMCPOAuth.ts` |
| MCP Service Interface | `web-app/src/services/mcp/types.ts` |
| Tauri MCP Service | `web-app/src/services/mcp/tauri.ts` |
| Default MCP Service | `web-app/src/services/mcp/default.ts` |
| MCP Settings UI | `web-app/src/routes/settings/mcp-servers.tsx` |
| Translations | `web-app/src/locales/en/mcp-servers.json` |
| MCP Server Config Type | `web-app/src/hooks/useMCPServers.ts` |

## Summary

The OAuth UI implementation provides a complete, user-friendly interface for authenticating MCP servers with OAuth 2.0. It follows Jan's established patterns:

- **Service Hub Architecture**: OAuth operations go through `MCPService`
- **Event-Driven Updates**: Uses Tauri events for real-time status changes
- **Consistent UI**: Matches existing MCP server card design
- **Type Safety**: Full TypeScript coverage with strict types
- **Internationalization**: Supports translation keys for all UI text
- **Error Handling**: Graceful error messages via toast notifications
- **Accessibility**: Tooltips and visual indicators for all states

Combined with the backend OAuth implementation, Jan now has a complete OAuth 2.0 system for MCP servers, enabling integration with enterprise tools like JIRA ROVO that require OAuth authentication.
