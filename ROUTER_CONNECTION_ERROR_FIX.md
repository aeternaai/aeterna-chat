# Router Connection Error Fix

## Problem

After fixing the auto-unload issue, the router started failing with `ConnectError` instead of working properly:

```
[Router] Confidence:1 "Reasoning:" "Selected gemma-3n-E4B-it-IQ4_XS because: best match (LLM router fallback: ConnectError)"
```

## Root Cause

The router configuration and model loading were happening **before** the Local API Server was actually started and running, causing connection failures:

### Initialization Sequence (BROKEN)

1. **App starts** → DataProvider mounts
2. **Router config sent** → Tries to configure Python router with `http://127.0.0.1:1337/v1`
3. **Router model loads** → `Phi-4-mini-instruct_Q4_K_M` starts loading
4. **Local API Server starts** (if `enableOnStartup` is true) → Proxy server binds to port 1337
5. **First routing request** → Router tries to call model → **ConnectError** (server not ready yet)

### The Issues

1. **Race condition**: Config sent before server is listening
2. **No server validation**: No check if server is actually running
3. **Premature model loading**: Router model loaded before it can be accessed via API

## Solution

### Wait for Server to Be Running

Modified both the router config sync and model loading to wait for `serverStatus === 'running'`:

**File:** `web-app/src/providers/DataProvider.tsx`

#### 1. Router Config Sync (lines ~108-143)

```typescript
// Keep Python router LLM config in sync with local API server settings
// Only configure when server is actually running
useEffect(() => {
  if (!isPlatformTauri()) return
  if (serverStatus !== 'running') return // ← NEW: Wait for server
  if (!serverHost || !serverPort) return

  const prefix = apiPrefix?.startsWith('/') ? apiPrefix : `/${apiPrefix ?? ''}`
  const sanitizedPrefix = prefix.replace(/\/+$/, '')
  const apiKeyPayload = apiKey && apiKey.toString().trim().length > 0 ? apiKey : undefined
  if (!apiKeyPayload) return

  const baseUrl = `http://${serverHost}:${serverPort}${sanitizedPrefix}`

  console.log(`[DataProvider] Configuring router with baseUrl: ${baseUrl}, model: Phi-4-mini-instruct_Q4_K_M`)

  serviceHub
    .core()
    .invoke('configure_router_llm', {
      config: {
        baseUrl,
        apiKey: apiKeyPayload,
        model: 'Phi-4-mini-instruct_Q4_K_M',
      },
    })
    .then(() => {
      console.log('[DataProvider] Router LLM config updated successfully')
    })
    .catch((error) => {
      console.warn('[DataProvider] Failed to configure Python router LLM settings:', error)
    })
}, [serviceHub, serverHost, serverPort, apiPrefix, apiKey, serverStatus]) // ← NEW: Added serverStatus dependency
```

#### 2. Router Model Loading (lines ~145-192)

```typescript
// Auto-load router model for LLM-based routing
// Only load after server is running to ensure it's accessible
useEffect(() => {
  if (!isPlatformTauri()) return
  if (serverStatus !== 'running') return // ← NEW: Wait for server

  const ROUTER_MODEL_ID = 'Phi-4-mini-instruct_Q4_K_M'
  const llamacppProvider = getProviderByName('llamacpp')

  if (!llamacppProvider) {
    console.warn('[DataProvider] Cannot load router model: llamacpp provider not available')
    return
  }

  // Check if router model exists in the provider
  const routerModelExists = llamacppProvider.models?.some((m) => m.id === ROUTER_MODEL_ID)
  if (!routerModelExists) {
    console.warn(
      `[DataProvider] Router model '${ROUTER_MODEL_ID}' not found in llamacpp provider. ` +
        'Please download it to enable LLM-based routing.'
    )
    return
  }

  // Check if router model is already loaded
  serviceHub
    .models()
    .getActiveModels()
    .then((activeModels) => {
      const isRouterModelLoaded = activeModels?.includes(ROUTER_MODEL_ID)

      if (isRouterModelLoaded) {
        console.log(`[DataProvider] Router model '${ROUTER_MODEL_ID}' is already loaded`)
        return
      }

      // Load the router model in the background
      console.log(`[DataProvider] Loading router model '${ROUTER_MODEL_ID}' for LLM-based routing...`)
      return serviceHub.models().startModel(llamacppProvider, ROUTER_MODEL_ID)
    })
    .then(() => {
      console.log(`[DataProvider] Router model '${ROUTER_MODEL_ID}' loaded successfully`)
    })
    .catch((error) => {
      console.warn(`[DataProvider] Failed to load router model '${ROUTER_MODEL_ID}':`, error)
    })
}, [serviceHub, getProviderByName, serverStatus]) // ← NEW: Added serverStatus dependency
```

### Initialization Sequence (FIXED)

1. **App starts** → DataProvider mounts
2. **Local API Server starts** (if `enableOnStartup`) → Proxy binds to port 1337
3. **Server status changes** → `serverStatus` = 'running'
4. **Router config sent** → Python router configured with correct URL
5. **Router model loads** → `Phi-4-mini-instruct_Q4_K_M` starts loading
6. **First routing request** → Router calls model → **Success!** ✅

## Benefits

1. **No more ConnectError** - Server is guaranteed to be running before config is sent
2. **Proper initialization order** - Config → Model loading → Routing
3. **Reactive updates** - If server restarts, config and model reload automatically
4. **Better logging** - `[DataProvider]` prefix shows initialization steps

## Expected Logs

### Successful Startup Sequence

```
[DataProvider] Configuring router with baseUrl: http://127.0.0.1:1337/v1, model: Phi-4-mini-instruct_Q4_K_M
[DataProvider] Router LLM config updated successfully
[DataProvider] Loading router model 'Phi-4-mini-instruct_Q4_K_M' for LLM-based routing...
[DataProvider] Router model 'Phi-4-mini-instruct_Q4_K_M' loaded successfully
```

### Successful Routing (No Fallback)

```
[LLMRouter] Starting route() with 3 candidate models (query chars=45)
[LLMRouter] → API Request: POST http://127.0.0.1:1337/v1/chat/completions (model='Phi-4-mini-instruct_Q4_K_M', timeout=15s, api_key=✓ set)
[LLMRouter] ← API Response: 200 OK (took ~234ms)
[LLMRouter] Parsing router response: '2|This query requires vision capabilities'
[LLMRouter] ✓ Successfully selected 'Qwen3-VL-8B-Instruct-IQ4_XS' via LLM router
```

## Testing

### 1. Verify No ConnectError

After rebuilding and restarting:
- Router should NOT show "LLM router fallback: ConnectError"
- Should see "LLM router fallback: False" in metadata

### 2. Check Startup Logs

Should see this order:
1. Local API Server starts
2. Router config sent
3. Router model loads
4. First message uses LLM routing (not fallback)

### 3. Verify Server Status Dependency

If you manually stop/start the server:
- Config should be re-sent when server status becomes 'running'
- Router model should reload if needed

## Edge Cases Handled

### Server Never Starts

If `enableOnStartup` is false or server fails to start:
- Router config is never sent (prevents errors)
- Router model is never loaded (saves resources)
- Routing falls back to heuristic (graceful degradation)

### Server Restarts

If server is restarted during a session:
- `serverStatus` changes from 'running' → 'stopped' → 'running'
- Config is re-sent when status becomes 'running' again
- Router model reloads if it was unloaded

### Model Already Loaded

If router model is already loaded (e.g., from previous session):
- Check detects it's already loaded
- Skips redundant loading
- Just logs confirmation

## Related Fixes

This fix complements:
1. **Auto-unload exemption** - Router model stays loaded (ROUTER_AUTO_UNLOAD_FIX.md)
2. **Enhanced logging** - Better diagnostics (ROUTER_ENHANCED_LOGGING.md)
3. **API key sync** - Runtime configuration (previous fixes)

## Files Modified

- `web-app/src/providers/DataProvider.tsx` - Added `serverStatus` checks and dependencies
