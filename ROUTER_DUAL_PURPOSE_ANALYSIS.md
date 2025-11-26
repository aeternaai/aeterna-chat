# Router Dual-Purpose Model Analysis

## Question
Can a model ID be used both in the list of available models for response AND as a router independently?

## Answer: ✅ YES - Partially Implemented, Needs One Fix

The current implementation **DOES support** using the same model (e.g., Phi-4) for both routing decisions and as a response model, with **one critical issue** that needs fixing.

---

## Current Implementation Status

### ✅ What Works

#### 1. **Multi-Session Support for Router Model**
**File**: `web-app/src/services/models/default.ts` (lines 310-352)

```typescript
const ROUTER_MODEL_ID = 'Phi-4-mini-instruct_Q4_K_M'
const isRouterModel = model === ROUTER_MODEL_ID
const routerAlreadyLoaded = isRouterModel && loadedModels.includes(model)

if (loadedModels.includes(model) && !routerAlreadyLoaded) {
  // Model already loaded and it's not the router model needing multi-session
  return undefined
}

// If router model is already loaded, enable multi-session mode to create separate session for response
const loadSettings = routerAlreadyLoaded 
  ? { ...settings, __allowMultiSession: true }
  : settings

if (routerAlreadyLoaded) {
  console.log(`[ModelsService] Loading additional session for router model '${model}' (response purpose)`)
}
```

**What this does**:
- Detects when Phi-4 is already loaded (for routing)
- Allows loading a **second session** of Phi-4 for responses
- Both sessions run independently with different PIDs
- Router session handles routing decisions
- Response session handles answering user queries

#### 2. **Separate Tracking in RouteContext**
**File**: `core/src/browser/extensions/router.ts`

```typescript
export interface RouteContext {
  /** Available response models (from ModelProvider state) - excludes router model */
  availableModels: AvailableModel[]

  /** Router model (used for routing decisions, separate from response models) */
  routerModel?: AvailableModel
  
  // ...
}
```

**What this does**:
- Router model tracked separately from response models
- Clear architectural separation
- Response models list can include or exclude router model based on configuration

---

### ❌ The Critical Issue

#### Problem: Router Model is ALWAYS Excluded from Response Models

**File**: `web-app/src/hooks/useChat.ts` (lines 78-128)

```typescript
const buildAvailableModels = (
  providers: ModelProvider[],
  activeModelIds: string[] = [],
  excludeRouterModel: boolean = true  // ❌ HARDCODED to true!
): AvailableModel[] => {
  // ...
  for (const model of provider.models) {
    // Skip router model if requested (for response models list)
    if (excludeRouterModel && model.id === ROUTER_MODEL_ID) {
      console.log(`[buildAvailableModels]   Skipping router model: ${model.id}`)
      continue  // ❌ Router model NEVER added to response models
    }
    // ...
  }
}
```

**Then called with hardcoded exclusion**:
```typescript
// Build response models (excluding router model)
const availableModels = buildAvailableModels(providers, activeModelIds, true)  // ❌ Always excludes!
```

**Result**: Even if user adds `Phi-4-mini-instruct_Q4_K_M` to `allowed_models`, it won't appear as a response option because it's filtered out before the `allowed_models` filter is even applied.

---

## Configuration Evidence

**File**: `extensions/router-extension/settings.json`

```json
{
  "key": "allowed_models",
  "controllerProps": {
    "value": "Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS,Phi-4-mini-instruct_Q4_K_M"
  }
}
```

**User Intent**: The user has explicitly added Phi-4 to `allowed_models`, expecting it to be available as a response option.

**Current Behavior**: Phi-4 is excluded BEFORE `allowed_models` filter is applied, so this configuration is ignored.

---

## How It Should Work

### Correct Flow

```
1. User configures allowed_models: "Qwen3-VL,gemma,Phi-4"
   ↓
2. buildAvailableModels() → Includes ALL models (including Phi-4)
   ↓
3. Router extension's filterAllowedModels() → Filters to only allowed models
   ↓
4. Result: availableModels = [Qwen3-VL, gemma, Phi-4]
   ↓
5. Router can select Phi-4 as response model
   ↓
6. If Phi-4 already loaded for routing:
   - startModel() detects routerAlreadyLoaded = true
   - Sets __allowMultiSession: true
   - Loads second session for response
   ↓
7. Two independent Phi-4 sessions running:
   - Session 1 (PID 12345): Router decisions
   - Session 2 (PID 67890): User responses
```

### Current (Broken) Flow

```
1. User configures allowed_models: "Qwen3-VL,gemma,Phi-4"
   ↓
2. buildAvailableModels(excludeRouterModel=true) → Excludes Phi-4
   ↓
3. Result: availableModels = [Qwen3-VL, gemma]  ❌ Phi-4 missing!
   ↓
4. Router extension's filterAllowedModels() → Filters [Qwen3-VL, gemma]
   ↓
5. Final: availableModels = [Qwen3-VL, gemma]
   ↓
6. User explicitly requests "use phi-4" → Router CAN'T select it (not in list)
```

---

## The Fix Required

### Change 1: Make excludeRouterModel Configurable

**File**: `web-app/src/hooks/useChat.ts`

**Option A**: Respect allowed_models configuration
```typescript
// Check if router model is in allowed_models configuration
const routerExtension = RouterManager.instance().get()
const allowedModels = await routerExtension?.getSetting('allowed_models', '')
const allowedModelsList = allowedModels?.split(',').map(m => m.trim()) || []
const isRouterInAllowedModels = allowedModelsList.includes(ROUTER_MODEL_ID)

// Only exclude router model if it's NOT explicitly in allowed_models
const availableModels = buildAvailableModels(
  providers, 
  activeModelIds, 
  !isRouterInAllowedModels  // ✅ Exclude only if NOT in allowed_models
)
```

**Option B**: Never exclude (let allowed_models filter handle it)
```typescript
// Build ALL models, let router extension's allowed_models filter handle exclusion
const availableModels = buildAvailableModels(
  providers, 
  activeModelIds, 
  false  // ✅ Include router model, let filtering handle it
)
```

**Option C**: Add user setting for dual-purpose mode
```typescript
// Add new setting in router extension: "allow_router_as_response_model"
const allowRouterAsResponse = await routerExtension?.getSetting('allow_router_as_response_model', false)

const availableModels = buildAvailableModels(
  providers, 
  activeModelIds, 
  !allowRouterAsResponse  // ✅ User controls behavior
)
```

---

## Recommendation: **Option B** (Simplest)

### Why Option B is Best

1. **Simplicity**: No additional configuration needed
2. **User Control**: `allowed_models` setting already provides filtering
3. **Flexibility**: Users can include/exclude router model via existing setting
4. **Consistency**: Same filtering logic applies to all models

### Implementation

**Change in**: `web-app/src/hooks/useChat.ts` (line ~787)

```typescript
// OLD:
const availableModels = buildAvailableModels(providers, activeModelIds, true)

// NEW:
const availableModels = buildAvailableModels(providers, activeModelIds, false)
```

**Remove the hardcoded exclusion parameter entirely**:

```typescript
const buildAvailableModels = (
  providers: ModelProvider[],
  activeModelIds: string[] = []
  // REMOVE: excludeRouterModel parameter
): AvailableModel[] => {
  // REMOVE: Router model exclusion logic
  // for (const model of provider.models) {
  //   if (excludeRouterModel && model.id === ROUTER_MODEL_ID) {
  //     continue
  //   }
  // }
  
  // Just build ALL models, let router extension filter them
}
```

---

## Verification After Fix

### Test Case 1: Router Model in allowed_models
**Config**: `allowed_models = "Qwen3-VL,gemma,Phi-4"`

**Expected**:
```
[buildAvailableModels] Built 3 available models
[buildAvailableModels] Model IDs: ["Qwen3-VL-8B-Instruct-IQ4_XS", "gemma-3n-E4B-it-IQ4_XS", "Phi-4-mini-instruct_Q4_K_M"]

[RouterExtension] After filtering (3): ["Qwen3-VL-8B-Instruct-IQ4_XS", "gemma-3n-E4B-it-IQ4_XS", "Phi-4-mini-instruct_Q4_K_M"]

User query: "use phi-4 to answer this"
→ Router selects: Phi-4-mini-instruct_Q4_K_M ✅
→ startModel() detects routerAlreadyLoaded = true
→ Creates second session with __allowMultiSession: true
→ Two sessions running independently
```

### Test Case 2: Router Model NOT in allowed_models
**Config**: `allowed_models = "Qwen3-VL,gemma"`

**Expected**:
```
[buildAvailableModels] Built 3 available models (includes Phi-4)

[RouterExtension] After filtering (2): ["Qwen3-VL-8B-Instruct-IQ4_XS", "gemma-3n-E4B-it-IQ4_XS"]
(Phi-4 filtered out by allowed_models)

User query: "use phi-4 to answer this"
→ Router cannot select Phi-4 (not in allowed models) ✅
→ Falls back to best available (Qwen3-VL or gemma)
```

---

## Current Status Summary

| Component | Status | Notes |
|-----------|--------|-------|
| Multi-session support | ✅ Working | Phi-4 can run 2+ sessions simultaneously |
| Router/Response separation | ✅ Working | Tracked separately in RouteContext |
| Auto-unload by PID | ✅ Working | Preserves router session, unloads response session |
| Settings persistence | ✅ Fixed | UI changes saved to settings.json |
| **Router model in response list** | ❌ **BROKEN** | Hardcoded exclusion prevents dual-purpose use |

---

## Conclusion

**Answer**: The architecture **fully supports** dual-purpose models (router + response), but there's a **single line of code** preventing it from working:

```typescript
// Line ~787 in web-app/src/hooks/useChat.ts
const availableModels = buildAvailableModels(providers, activeModelIds, true)
//                                                                       ^^^^
//                                                                   Change to false
```

**Recommended Fix**: Change `true` to `false` and remove router model exclusion logic from `buildAvailableModels()`. Let the existing `allowed_models` filter handle whether router model should be available as a response option.

This gives users full control: if they want Phi-4 only for routing, don't add it to `allowed_models`. If they want it for both routing and responses, add it to `allowed_models`.
