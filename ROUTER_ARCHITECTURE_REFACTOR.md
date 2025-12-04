# Router Architecture Refactor: Separate Router Model from Response Models

## Overview

This refactor implements **Option C: Separate Lists** architecture to properly separate the router model (Phi-4) from response models, fixing the confusion where the router model was being auto-added to the list of available models for responses.

## Problem Statement

### Original Issue
The router model (Phi-4-mini-instruct_Q4_K_M) was being automatically added to the list of available response models, causing several problems:

1. **Architectural Confusion**: The router model is infrastructure (like a load balancer), not a response option
2. **Wrong Model Selection**: When user requested "switch to qwen 3-vl 8b", the LLM router would sometimes select itself (Phi-4) instead of the requested model
3. **Misleading Available Models**: Users configured `allowed_models = "Qwen3-VL,gemma"` but saw 3 models (including Phi-4)
4. **Self-Referential Routing**: Router model could route to itself, creating logical inconsistency

### Secondary Issue
Settings changes via UI were only saved to `localStorage`, not to `settings.json` file, so changes didn't persist across restarts.

## Solution Architecture

### 1. Core Interface Changes

**File**: `core/src/browser/extensions/router.ts`

Added `routerModel` field to `RouteContext` interface:

```typescript
export interface RouteContext {
  /** Conversation messages (user's question + history) */
  messages: ChatCompletionMessage[]

  /** Thread ID for context */
  threadId?: string

  /** Available response models (from ModelProvider state) - excludes router model */
  availableModels: AvailableModel[]

  /** Router model (used for routing decisions, separate from response models) */
  routerModel?: AvailableModel  // NEW FIELD

  /** Currently active/loaded models */
  activeModels: string[]

  /** User preferences (optional constraints) */
  preferences?: RoutePreferences

  /** Attachments (images, documents) */
  attachments?: {
    images: number
    documents: number
    hasCode: boolean
  }
}
```

### 2. Frontend Changes

**File**: `web-app/src/hooks/useChat.ts`

#### Added Router Model Constant
```typescript
const ROUTER_MODEL_ID = 'Phi-4-mini-instruct_Q4_K_M'
```

#### Modified buildAvailableModels Function
- Added `excludeRouterModel` parameter (default: `true`)
- Skips router model when building response models list
- Enhanced logging to show when router model is excluded

```typescript
const buildAvailableModels = (
  providers: ModelProvider[],
  activeModelIds: string[] = [],
  excludeRouterModel: boolean = true  // NEW PARAMETER
): AvailableModel[] => {
  // ... 
  for (const model of provider.models) {
    // Skip router model if requested (for response models list)
    if (excludeRouterModel && model.id === ROUTER_MODEL_ID) {
      console.log(`[buildAvailableModels]   Skipping router model: ${model.id}`)
      continue
    }
    // ...
  }
}
```

#### Added buildRouterModel Function
New helper function to build router model separately:

```typescript
const buildRouterModel = (
  providers: ModelProvider[],
  activeModelIds: string[] = []
): AvailableModel | undefined => {
  console.log('[buildRouterModel] Looking for router model:', ROUTER_MODEL_ID)
  
  for (const provider of providers) {
    if (!provider.active) continue
    
    const routerModel = provider.models.find(m => m.id === ROUTER_MODEL_ID)
    if (routerModel) {
      const isLoaded = activeModelIds.includes(routerModel.id)
      console.log('[buildRouterModel] Found router model:', ROUTER_MODEL_ID, 'loaded:', isLoaded)
      
      // Build AvailableModel structure...
      return { /* ... */ }
    }
  }
  
  console.log('[buildRouterModel] Router model not found')
  return undefined
}
```

#### Updated Routing Call
```typescript
// Build response models (excluding router model)
const availableModels = buildAvailableModels(providers, activeModelIds, true)

// Build router model separately
const routerModel = buildRouterModel(providers, activeModelIds)

console.log('[Router] Routing query with', availableModels.length, 'response models')
console.log('[Router] Router model:', routerModel?.id || 'none')

const routeDecision = await router.route({
  messages: routingMessages,
  threadId: activeThread.id,
  availableModels,      // Response models only
  routerModel,          // Router model passed separately
  activeModels: activeModelIds,
  // ...
})
```

### 3. Router Extension Changes

**File**: `extensions/router-extension/src/index.ts`

#### Removed Auto-Add Logic
Deleted the entire section that auto-added router model to available models:

```typescript
// REMOVED:
// const ROUTER_MODEL_ID = 'Phi-4-mini-instruct_Q4_K_M'
// const routerModelExists = context.availableModels.find(m => m.id === ROUTER_MODEL_ID)
// const alreadyInFiltered = filteredModels.find(m => m.id === ROUTER_MODEL_ID)
// if (routerModelExists && !alreadyInFiltered && this.allowedModels.length > 0) {
//   filteredModels.push(routerModelExists)
//   console.log(`[RouterExtension] Auto-added router model '${ROUTER_MODEL_ID}' to available models`)
// }
```

#### Updated Logging
Changed terminology from "available models" to "response models":

```typescript
console.log(`[RouterExtension] Input available response models (${context.availableModels.length}):`, context.availableModels.map(m => m.id))
console.log(`[RouterExtension] Router model:`, context.routerModel?.id || 'none')
console.log(`[RouterExtension] Allowed models filter:`, this.allowedModels)

// Filter by allowed models - only applies to RESPONSE models
const filteredModels = this.filterAllowedModels(context.availableModels)
console.log(`[RouterExtension] After filtering (${filteredModels.length}):`, filteredModels.map(m => m.id))
```

#### Updated Python Router Call
```typescript
console.log(`🐍 [RouterExtension] Router model:`, context.routerModel?.id || 'not provided')
console.log(`🐍 [RouterExtension] Sending ${filteredContext.availableModels.length} response models to Python router:`)

const decision = await invoke('route_request', {
  request: {
    messages: filteredContext.messages,
    threadId: filteredContext.threadId,
    availableModels: filteredContext.availableModels,  // Response models
    routerModel: filteredContext.routerModel,          // Router model separate
    activeModels: filteredContext.activeModels,
    attachments: filteredContext.attachments,
    preferences: filteredContext.preferences,
  }
}) as RouteDecision
```

### 4. Python Router Service Changes

**File**: `src-tauri/router-service/models.py`

Added `router_model` field to `RouteRequest`:

```python
class RouteRequest(BaseModel):
    """Request for routing decision"""
    messages: list[Message]
    thread_id: Optional[str] = Field(None, alias="threadId")
    available_models: list[AvailableModel] = Field(alias="availableModels")
    router_model: Optional[AvailableModel] = Field(None, alias="routerModel")  # NEW FIELD
    active_models: list[str] = Field(default_factory=list, alias="activeModels")
    attachments: Optional[Attachments] = None
    preferences: Optional[RoutePreferences] = None
```

**File**: `src-tauri/router-service/strategies/llm.py`

Added validation and logging in `route()` method:

```python
async def route(self, request: RouteRequest) -> RouteResponse:
    if not request.messages:
        raise ValueError("No messages provided for routing")
    if not request.available_models:
        raise ValueError("No available models to route to")

    query = self._get_message_content(request.messages[-1])
    
    # Log router model info
    router_model_info = "none"
    if request.router_model:
        router_model_info = f"{request.router_model.id} ({'loaded' if request.router_model.metadata.is_loaded else 'not loaded'})"
    logger.info("[LLMRouter] Router model: %s", router_model_info)
    
    # Ensure router model is not in available models list
    available_model_ids = [m.id for m in request.available_models]
    if request.router_model and request.router_model.id in available_model_ids:
        logger.warning(
            "[LLMRouter] ⚠️  Router model '%s' found in available models list - this should not happen!",
            request.router_model.id
        )
    
    # ... continue with routing logic
    logger.info(
        "[LLMRouter] Starting route() with %d response models (query chars=%d)",
        len(request.available_models),
        len(query),
    )
    logger.debug("[LLMRouter] Response models: %s", [m.id for m in request.available_models])
```

### 5. Settings Persistence Fix

**File**: `core/src/browser/extension.ts`

Added imports:
```typescript
import { fs } from './fs'
import { joinPath } from './core'
```

Enhanced `updateSettings()` method to persist to file system:

```typescript
async updateSettings(componentProps: Partial<SettingComponentProps>[]): Promise<void> {
  if (!this.name) return

  console.log(`[Extension:${this.name}] updateSettings called with:`, componentProps)

  const settings = await this.getSettings()

  let updatedSettings = settings.map((setting) => {
    const updatedSetting = componentProps.find(
      (componentProp) => componentProp.key === setting.key
    )
    if (updatedSetting && updatedSetting.controllerProps) {
      setting.controllerProps.value = updatedSetting.controllerProps.value
    }
    return setting
  })

  if (!updatedSettings.length) updatedSettings = componentProps as SettingComponentProps[]

  // Save to localStorage (always, for web compatibility)
  localStorage.setItem(this.name, JSON.stringify(updatedSettings))
  console.log(`[Extension:${this.name}] Settings saved to localStorage`)

  // ALSO save to file system if available (Tauri/desktop mode)
  try {
    // Check if fs API is available
    if (globalThis.core?.api?.writeFileSync) {
      console.log(`[Extension:${this.name}] File system API available, persisting to settings.json...`)
      
      // Build path to extension's settings.json file
      // Extension URL is like: file://extensions/router-extension/index.js
      // We want: file://extensions/router-extension/settings.json
      const settingsPath = await joinPath([this.url.replace(/\/[^\/]+$/, ''), 'settings.json'])
      
      console.log(`[Extension:${this.name}] Writing settings to:`, settingsPath)
      
      // Convert settings to JSON format matching settings.json structure
      const settingsJson: Record<string, any> = {}
      updatedSettings.forEach(setting => {
        settingsJson[setting.key] = setting.controllerProps.value
      })
      
      await fs.writeFileSync(settingsPath, JSON.stringify(settingsJson, null, 2))
      console.log(`[Extension:${this.name}] ✅ Settings persisted to settings.json successfully`)
    } else {
      console.log(`[Extension:${this.name}] File system API not available (web mode), using localStorage only`)
    }
  } catch (error) {
    console.error(`[Extension:${this.name}] ⚠️  Failed to persist settings to file system:`, error)
    // Don't throw - localStorage save already succeeded
  }

  updatedSettings.forEach((setting) => {
    this.onSettingUpdate<typeof setting.controllerProps.value>(
      setting.key,
      setting.controllerProps.value
    )
  })
}
```

## Benefits of This Refactor

### 1. Clear Separation of Concerns
- **Router Model**: Infrastructure for making routing decisions (Phi-4)
- **Response Models**: Models that actually answer user queries (Qwen3-VL, gemma, etc.)

### 2. Accurate Model Selection
- Router can only select from response models
- No more self-referential routing (router selecting itself)
- User requests like "switch to qwen 3-vl 8b" now work correctly

### 3. Transparent Configuration
- `allowed_models` filter only applies to response models
- Users see exactly what they configured, no hidden additions
- Router model status shown separately in logs

### 4. Better Debugging
- Clear logging shows router model and response models separately
- Warning if router model somehow appears in response list
- Easy to verify architectural correctness

### 5. Persistent Settings
- UI changes to `allowed_models` now saved to `settings.json`
- Settings persist across app restarts
- Works in both Tauri (file system) and web (localStorage) modes

## Expected Behavior After Refactor

### Console Logs (Example)
```
[buildAvailableModels] Building with 1 providers
[buildAvailableModels]   Skipping router model: Phi-4-mini-instruct_Q4_K_M
[buildAvailableModels] Built 2 available models
[buildAvailableModels] Model IDs: ["Qwen3-VL-8B-Instruct-IQ4_XS", "gemma-3n-E4B-it-IQ4_XS"]

[buildRouterModel] Looking for router model: Phi-4-mini-instruct_Q4_K_M
[buildRouterModel] Found router model: Phi-4-mini-instruct_Q4_K_M loaded: true

[Router] Routing query with 2 response models
[Router] Router model: Phi-4-mini-instruct_Q4_K_M

[RouterExtension] Input available response models (2): ["Qwen3-VL-8B-Instruct-IQ4_XS", "gemma-3n-E4B-it-IQ4_XS"]
[RouterExtension] Router model: Phi-4-mini-instruct_Q4_K_M

🐍 [RouterExtension] Router model: Phi-4-mini-instruct_Q4_K_M (loaded)
🐍 [RouterExtension] Sending 2 response models to Python router:
   1. Qwen3-VL-8B-Instruct-IQ4_XS (llamacpp) - vision, chat
   2. gemma-3n-E4B-it-IQ4_XS (llamacpp) - code, chat

[LLMRouter] Router model: Phi-4-mini-instruct_Q4_K_M (loaded)
[LLMRouter] Starting route() with 2 response models (query chars=25)
[LLMRouter] Response models: ["Qwen3-VL-8B-Instruct-IQ4_XS", "gemma-3n-E4B-it-IQ4_XS"]
```

### Settings Persistence (Example)
```
[Extension:router-extension] updateSettings called with: [{ key: 'allowed_models', controllerProps: { value: 'Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS,Phi-4-mini-instruct_Q4_K_M' } }]
[Extension:router-extension] Settings saved to localStorage
[Extension:router-extension] File system API available, persisting to settings.json...
[Extension:router-extension] Writing settings to: file://extensions/router-extension/settings.json
[Extension:router-extension] ✅ Settings persisted to settings.json successfully
```

## Testing Checklist

### Router Model Separation
- [ ] Verify router model (Phi-4) NOT in `buildAvailableModels()` output when `excludeRouterModel=true`
- [ ] Verify `buildRouterModel()` returns router model separately
- [ ] Verify console logs show "2 response models" (not 3) when only 2 configured
- [ ] Verify router extension logs show router model separately from response models
- [ ] Verify Python router receives router model in separate field
- [ ] Verify warning appears if router model somehow in response models list

### Routing Accuracy
- [ ] Test query: "switch to qwen 3-vl 8b" → Should select Qwen3-VL, not Phi-4
- [ ] Test query: "show me code for..." → Should select gemma (code model)
- [ ] Test query: "describe this image" → Should select Qwen3-VL (vision model)
- [ ] Verify router never selects itself as response model

### Settings Persistence
- [ ] Change `allowed_models` in UI
- [ ] Verify console shows "✅ Settings persisted to settings.json successfully"
- [ ] Verify `extensions/router-extension/settings.json` file updated
- [ ] Restart app
- [ ] Verify settings still reflect UI changes (persisted correctly)

## Migration Notes

### No Breaking Changes for Users
- Existing `allowed_models` configuration continues to work
- No changes to UI or user-facing features
- Purely internal architectural improvement

### For Developers
- If extending router system, use `routerModel` field from `RouteContext`
- Never add router model to `availableModels` list
- Use `buildRouterModel()` helper when router model info needed
- Settings changes now require rebuilding core package

## Files Modified

### Core Package
- `core/src/browser/extensions/router.ts` - Added `routerModel` field to `RouteContext`
- `core/src/browser/extension.ts` - Added file system persistence to `updateSettings()`

### Web App
- `web-app/src/hooks/useChat.ts` - Separated router model from response models

### Router Extension
- `extensions/router-extension/src/index.ts` - Removed auto-add logic, updated logging

### Python Router Service
- `src-tauri/router-service/models.py` - Added `router_model` field to `RouteRequest`
- `src-tauri/router-service/strategies/llm.py` - Added validation and logging

## Related Documentation
- See `ROUTER_MODEL_SELECTION_FIX.md` for the original bug report
- See `ROUTER_DIAGNOSTIC_LOGGING.md` for debugging guide
