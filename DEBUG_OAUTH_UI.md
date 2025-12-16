# Debugging OAuth UI in Jan

## Step 1: Check if Config is Loaded

Open Jan, go to Settings → MCP Servers, then open the Browser DevTools Console (Cmd+Option+I on Mac) and run:

```javascript
// Get the current Zustand state
const mcpServers = window.__ZUSTAND_DEVTOOLS__?.getState?.()

// Or access directly
await window.core?.api?.getMcpConfigs()
```

This will show you the raw config. Look for:
- Does `Jira-rovo` exist?
- Does it have an `oauth` field?

## Step 2: Check if useMCPServers has the data

In the console:

```javascript
// Get React DevTools and inspect the useMCPServers hook
// Look for the mcpServers state - does it include oauth field?
```

## Step 3: Add Temporary Debug Logging

Edit `web-app/src/routes/settings/mcp-servers.tsx` and add right after the `useMCPOAuth()` call:

```typescript
const {
  startOAuthFlow,
  revokeOAuthToken,
  isAuthenticated,
  isLoading: oauthLoading,
} = useMCPOAuth()

// ADD THIS DEBUG CODE:
useEffect(() => {
  console.log('[DEBUG] MCP Servers:', mcpServers)
  Object.entries(mcpServers).forEach(([key, config]) => {
    console.log(`[DEBUG] Server ${key}:`, {
      hasOAuth: !!config.oauth,
      oauthConfig: config.oauth,
      fullConfig: config
    })
  })
}, [mcpServers])
```

Then rebuild and check the console when you visit Settings → MCP Servers.

## Step 4: Quick Manual Test

In the browser console when on MCP Servers page:

```javascript
// Check if the OAuth functions are available
console.log('startOAuthFlow:', typeof startOAuthFlow)
console.log('revokeOAuthToken:', typeof revokeOAuthToken)
console.log('isAuthenticated:', typeof isAuthenticated)

// Try calling the backend directly
await window.__TAURI__.core.invoke('get_all_mcp_oauth_statuses')
```

## Step 5: Check Backend

In terminal:

```bash
# Check if the config file has the oauth field
cat ~/Library/Application\ Support/jan/mcp_config.json | jq '.mcpServers["Jira-rovo"].oauth'

# Check if Jan is actually loading it
# Restart Jan with make dev and watch the logs for OAuth-related messages
make dev 2>&1 | grep -i oauth
```

## Expected Output

If everything is working:
1. Console should show `hasOAuth: true` for Jira-rovo
2. The UI should render a badge (green or amber) and a key icon
3. The backend should have OAuth status available

## Common Issues

### Issue: config.oauth is null/undefined
**Fix**: Config not loading properly - check DataProvider.tsx or rebuild

### Issue: Key icon not visible
**Fix**: Check if the conditional `{config.oauth && (` is being evaluated - add console.log before it

### Issue: Hook functions undefined
**Fix**: useMCPOAuth not initialized - check imports and service hub

### Issue: TypeScript errors
**Fix**: Run `yarn tsc --noEmit` to check for type errors
