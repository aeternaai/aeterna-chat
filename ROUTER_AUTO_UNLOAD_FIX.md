# Router Model Auto-Unload Fix

## Problem

The LLM router was failing on the second request with "HTTPStatusError" (404), forcing fallback to heuristic routing.

### Root Cause

Jan's **auto-unload mechanism** was unloading the router model when loading the selected response model:

1. **First request**: 
   - Router model `Phi-4-mini-instruct_Q4_K_M` is loaded
   - Router successfully selects a model (e.g., `Qwen3-VL-8B-Instruct-IQ4_XS`)
   - Jan loads `Qwen3-VL-8B-Instruct-IQ4_XS` for the response

2. **Auto-Unload Triggered**:
   - When loading `Qwen3-VL-8B-Instruct-IQ4_XS`, Jan's auto-unload feature kicks in
   - **ALL non-embedding models are unloaded**, including the router model
   - Only `Qwen3-VL-8B-Instruct-IQ4_XS` remains loaded

3. **Second request**:
   - Router tries to call `Phi-4-mini-instruct_Q4_K_M` via `/v1/chat/completions`
   - Model session doesn't exist → **404 Not Found**
   - Falls back to heuristic routing

### Auto-Unload Logic Location

**File:** `extensions/llamacpp-extension/src/index.ts` (lines 1656-1686)

```typescript
if (
  this.autoUnload &&
  !isEmbedding &&
  (loadedModels.length > 0 || otherLoadingPromises.length > 0)
) {
  // Wait for OTHER loading models to finish, then unload everything
  if (otherLoadingPromises.length > 0) {
    await Promise.all(otherLoadingPromises)
  }

  // Now unload all loaded Text models excluding embedding models
  const allLoadedModels = await this.getLoadedModels()
  if (allLoadedModels.length > 0) {
    // ... unload all non-embedding models
  }
}
```

## Solution

### Exempt Router Model from Auto-Unload

Added logic to preserve the router model when auto-unloading:

**File:** `extensions/llamacpp-extension/src/index.ts`

#### 1. Add Router Model ID Property (line ~189)

```typescript
export default class llamacpp_extension extends AIEngine {
  provider: string = 'llamacpp'
  autoUnload: boolean = true
  // Router model ID to exempt from auto-unload
  routerModelId: string = 'Phi-4-mini-instruct_Q4_K_M'
  timeout: number = 600
  // ...
}
```

#### 2. Filter Router Model from Auto-Unload List (lines ~1679-1690)

```typescript
const nonEmbeddingModels: string[] = sessionInfos
  .filter(
    (s): s is SessionInfo => s !== null && s.is_embedding === false
  )
  .map((s) => s.model_id)
  // Exclude router model from auto-unload to prevent 404 errors in LLM routing
  .filter((id) => id !== this.routerModelId)

if (nonEmbeddingModels.length > 0) {
  logger.info(
    `Auto-unloading ${nonEmbeddingModels.length} models (preserving router model: ${this.routerModelId})`
  )
  await Promise.all(
    nonEmbeddingModels.map((modelId) => this.unload(modelId))
  )
}
```

## Benefits

1. **Router model stays loaded** - No 404 errors on subsequent requests
2. **Consistent LLM-based routing** - No fallback to heuristic after first request
3. **Minimal memory overhead** - Router model is lightweight (~2-4GB)
4. **Better user experience** - Routing remains intelligent across all requests

## Behavior After Fix

### Loading Sequence

1. **App Startup**: Router model auto-loaded by DataProvider
2. **User Message**: Router selects best model → loads it
3. **Auto-Unload**: Other models unloaded, **router model preserved**
4. **Next Message**: Router still available → selects model again

### Memory Profile

- **Router model** (`Phi-4-mini-instruct_Q4_K_M`): ~3GB - Always loaded
- **Response model** (e.g., `Qwen3-VL-8B-Instruct-IQ4_XS`): ~5GB - Loaded on demand
- **Total**: ~8GB (vs. 5GB if router unloaded, but with broken routing)

## Alternative Approaches Considered

### 1. Disable Auto-Unload Globally
**Rejected**: Would break single-model-at-a-time UX for users with limited RAM

### 2. Reload Router Model on Each Request
**Rejected**: 
- Adds 5-10s latency per routing decision
- Unnecessary model loading/unloading churn
- Poor UX

### 3. Use External Router Process
**Rejected**: 
- Adds complexity (separate Python/Node.js process)
- Cross-process communication overhead
- Deployment/packaging complications

### 4. Exemption by Model ID (Chosen)
**Selected**: 
- Minimal code change
- No UX impact
- Clean and maintainable
- Router model always available

## Testing

### Verify Fix Works

1. **Check router model stays loaded**:
   ```typescript
   // In browser console after sending multiple messages
   const activeModels = await window.core.api.getActiveModels()
   console.log('Active models:', activeModels)
   // Should include 'Phi-4-mini-instruct_Q4_K_M' across all requests
   ```

2. **Check logs for preservation message**:
   ```
   Auto-unloading 1 models (preserving router model: Phi-4-mini-instruct_Q4_K_M)
   ```

3. **Verify no HTTP 404 errors**:
   ```
   [LLMRouter] ← API Response: 200 OK (took ~234ms)
   [LLMRouter] ✓ Successfully selected 'Qwen3-VL-8B-Instruct-IQ4_XS'
   ```

### Test Scenarios

- ✅ **First message**: Router works
- ✅ **Second message**: Router still works (no fallback)
- ✅ **Model switching**: Response model changes, router model stays loaded
- ✅ **Memory efficiency**: Only 1 response model + router model loaded at a time

## Future Enhancements

### Make Router Model Configurable

Could add setting to allow users to change router model:

```typescript
// In extension settings
{
  key: 'router_model_id',
  title: 'Router Model',
  description: 'Lightweight model used for intelligent routing decisions',
  controllerType: ControllerType.Text,
  controllerProps: {
    value: 'Phi-4-mini-instruct_Q4_K_M',
  },
}
```

### Support Multiple Persistent Models

Could extend to allow multiple models to be "pinned":

```typescript
persistentModelIds: Set<string> = new Set(['Phi-4-mini-instruct_Q4_K_M'])

// In settings
persistentModelIds.add(userSelectedModelId)
```

## Related Files

- `extensions/llamacpp-extension/src/index.ts` - Auto-unload logic and fix
- `web-app/src/providers/DataProvider.tsx` - Router model auto-loading
- `src-tauri/router-service/strategies/llm.py` - LLM router implementation
- `docs/dev/model-switching-mechanism.md` - Auto-unload documentation
