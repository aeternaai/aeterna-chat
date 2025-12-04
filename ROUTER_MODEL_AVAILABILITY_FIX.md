# Router Model Availability Fix

## Problem

**User Report**: "When I ask to use `Phi-4-mini-instruct_Q4_K_M`, it says the model is not available, even though I added it to the available models."

### Root Cause

The router extension was **filtering models** based on the `allowed_models` setting. If you explicitly set allowed models (e.g., `Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS`), any model not in that list would be filtered out and unavailable for routing.

**The Issue**: The router model (`Phi-4-mini-instruct_Q4_K_M`) was being used for routing decisions, but if it wasn't in the `allowed_models` list, it would be:
1. Filtered out from available models
2. Unavailable for selection as a response model
3. Users couldn't explicitly request it for answering

This created a confusing situation where:
- ✅ Router model loads fine (for routing decisions)
- ❌ But can't be selected for answering (filtered out)

## Solution

### Auto-Include Router Model in Available Models

**File:** `extensions/router-extension/src/index.ts`

```typescript
async route(context: RouteContext): Promise<RouteDecision> {
  console.log(`[RouterExtension] Routing request...`)

  // Filter by allowed models
  const filteredModels = this.filterAllowedModels(context.availableModels)
  
  // IMPORTANT: Ensure router model itself is always available as a response option
  // This allows users to explicitly request the router model for answering
  const ROUTER_MODEL_ID = 'Phi-4-mini-instruct_Q4_K_M'
  const routerModelExists = context.availableModels.find(m => m.id === ROUTER_MODEL_ID)
  const alreadyInFiltered = filteredModels.find(m => m.id === ROUTER_MODEL_ID)
  
  if (routerModelExists && !alreadyInFiltered && this.allowedModels.length > 0) {
    // Router model exists but was filtered out by allowed models - add it back
    filteredModels.push(routerModelExists)
    console.log(`[RouterExtension] Auto-added router model '${ROUTER_MODEL_ID}' to available models`)
  }

  const filteredContext = {
    ...context,
    availableModels: filteredModels,
  }
  
  // ... rest of routing logic
}
```

### How It Works

1. **Filter models** based on `allowed_models` setting (as before)
2. **Check if router model exists** in the original available models
3. **Check if router model was filtered out** (not in the filtered list)
4. **If filtered out AND allowed_models is configured** → Auto-add it back
5. **Log the action** for transparency

### Scenarios

#### Scenario 1: Router Model Not in Allowed List

**Before Fix**:
```
Allowed Models: Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS
Available after filter: [Qwen3-VL, gemma-3n]  ← Router model missing!
User asks for Phi-4: ❌ "Model not available"
```

**After Fix**:
```
Allowed Models: Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS
Available after filter: [Qwen3-VL, gemma-3n, Phi-4-mini-instruct_Q4_K_M]  ← Auto-added!
User asks for Phi-4: ✅ Router can select it
```

#### Scenario 2: Router Model Already in Allowed List

**Settings**:
```
Allowed Models: Qwen3-VL-8B-Instruct-IQ4_XS,Phi-4-mini-instruct_Q4_K_M
```

**Behavior**:
```
Available after filter: [Qwen3-VL, Phi-4]  ← Already present
Auto-add check: Model already in list, skip
Result: ✅ Works as expected (no duplicate)
```

#### Scenario 3: No Allowed Models Filter (Empty List)

**Settings**:
```
Allowed Models: (empty - all models allowed)
```

**Behavior**:
```
Available after filter: [All models including Phi-4]
Auto-add check: allowedModels.length === 0, skip auto-add
Result: ✅ All models available naturally
```

## Benefits

### 1. User Experience
- ✅ Can explicitly request router model for simple queries
- ✅ Router model always available regardless of filter settings
- ✅ No confusing "model not available" errors

### 2. Flexibility
- ✅ Use lightweight router model for simple questions
- ✅ Save resources by routing simple queries to the router model
- ✅ Reserve larger models for complex queries

### 3. Consistency
- ✅ Router model accessible both for routing AND answering
- ✅ No hidden model filtering surprises
- ✅ Transparent behavior with logging

## Use Cases

### Use Case 1: Simple Question → Use Router Model

**User**: "What is 2+2?"

**Routing Decision**:
```
Router analyzes: Simple math question
Router selects: Phi-4-mini-instruct_Q4_K_M (itself!)
Result: ✅ Router model answers directly (fastest response)
```

**Benefits**: No need to load a larger model for trivial queries

### Use Case 2: Complex Vision Task → Use Specialized Model

**User**: "Analyze this image and describe what you see" [image attached]

**Routing Decision**:
```
Router analyzes: Vision task detected
Router selects: Qwen3-VL-8B-Instruct-IQ4_XS (vision model)
Result: ✅ Loads vision-capable model
```

**Benefits**: Right model for the task

### Use Case 3: Explicit Model Request

**User**: "Use Phi-4 to explain quantum physics"

**Before Fix**:
```
❌ Error: Model 'Phi-4-mini-instruct_Q4_K_M' not available
```

**After Fix**:
```
Router finds Phi-4 in available models (auto-added)
Router selects: Phi-4-mini-instruct_Q4_K_M
Result: ✅ Uses requested model
```

## Configuration Examples

### Example 1: Minimal Setup (Router + One Model)

```
Allowed Models: Qwen3-VL-8B-Instruct-IQ4_XS
```

**Available Models**:
- `Qwen3-VL-8B-Instruct-IQ4_XS` (from allowed list)
- `Phi-4-mini-instruct_Q4_K_M` (auto-added router)

**Result**: 2 models available for routing

### Example 2: Multi-Model Setup

```
Allowed Models: Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS,llama-3.2-8B
```

**Available Models**:
- `Qwen3-VL-8B-Instruct-IQ4_XS` (vision)
- `gemma-3n-E4B-it-IQ4_XS` (coding)
- `llama-3.2-8B` (general)
- `Phi-4-mini-instruct_Q4_K_M` (auto-added router)

**Result**: 4 models available, router handles all simple queries

### Example 3: Include Router in Allowed List

```
Allowed Models: Qwen3-VL-8B-Instruct-IQ4_XS,Phi-4-mini-instruct_Q4_K_M
```

**Available Models**:
- `Qwen3-VL-8B-Instruct-IQ4_XS` (from allowed list)
- `Phi-4-mini-instruct_Q4_K_M` (from allowed list, auto-add skipped)

**Result**: Same as before, but explicit (no auto-add needed)

## Logs to Expect

### When Router Model is Auto-Added

```
[RouterExtension] Routing request...
[RouterExtension] Auto-added router model 'Phi-4-mini-instruct_Q4_K_M' to available models
[RouterExtension] Available models after filtering: 3
```

### When Router Model Already Allowed

```
[RouterExtension] Routing request...
[RouterExtension] Available models after filtering: 3
(No auto-add message - already present)
```

### When Router Model Selected for Response

```
[RouterExtension] Python router selected: Phi-4-mini-instruct_Q4_K_M
[RouterExtension] ✓ Model already loaded (router session)
(Uses existing router session - no new load needed)
```

## Edge Cases Handled

### 1. Router Model Doesn't Exist in System

**Scenario**: Router model not downloaded

**Behavior**:
```typescript
const routerModelExists = context.availableModels.find(m => m.id === ROUTER_MODEL_ID)
// routerModelExists === undefined

if (routerModelExists && ...) {
  // This block won't execute - no auto-add
}
```

**Result**: ✅ No crash, silently skipped

### 2. No Allowed Models Filter

**Scenario**: `allowed_models` is empty string

**Behavior**:
```typescript
if (routerModelExists && !alreadyInFiltered && this.allowedModels.length > 0) {
  // this.allowedModels.length === 0, so condition is false
}
```

**Result**: ✅ No auto-add (all models already available)

### 3. Router Model Already in Filtered List

**Scenario**: User manually added router to allowed models

**Behavior**:
```typescript
const alreadyInFiltered = filteredModels.find(m => m.id === ROUTER_MODEL_ID)
// alreadyInFiltered === <model object>

if (routerModelExists && !alreadyInFiltered && ...) {
  // !alreadyInFiltered is false, so condition is false
}
```

**Result**: ✅ No duplicate, skipped

## Future Enhancements

### 1. Configurable Router Model ID

Make the router model ID configurable via settings:

```typescript
const routerModelId = await this.getSetting<string>(
  'router_model_id',
  'Phi-4-mini-instruct_Q4_K_M'
)
```

### 2. Multiple Router Models

Support different router models for different strategies:

```typescript
const routerModels = {
  'llm-based': 'Phi-4-mini-instruct_Q4_K_M',
  'heuristic': null, // No model needed
  'embedding-based': 'text-embedding-3-small'
}
```

### 3. Smart Router Model Selection

Auto-select best router model based on query complexity:

```typescript
// Simple query → Use fast router
// Complex query → Use smarter router
```

## Testing

### Test Case 1: Router Model Filtered Out

1. Set allowed models: `Qwen3-VL-8B-Instruct-IQ4_XS`
2. Ask: "Use Phi-4 to answer: What is 2+2?"
3. Verify: Phi-4 appears in logs as available model
4. Verify: Response comes from Phi-4

### Test Case 2: Router Model in Allowed List

1. Set allowed models: `Qwen3-VL-8B-Instruct-IQ4_XS,Phi-4-mini-instruct_Q4_K_M`
2. Ask: "Use Phi-4 to answer: What is the capital of France?"
3. Verify: No "auto-added" log message
4. Verify: Response comes from Phi-4

### Test Case 3: No Allowed Models Filter

1. Set allowed models: (empty)
2. Ask: "Use Phi-4 to answer: Explain AI"
3. Verify: All models available
4. Verify: Response comes from Phi-4

## Summary

This fix ensures the router model is **always available** as a response option, allowing users to:
- Explicitly request the router model for answering
- Use the lightweight router model for simple queries
- Avoid "model not available" errors
- Have a consistent and predictable routing experience

The router model now serves dual purposes:
1. **Router role**: Making routing decisions (always active)
2. **Response role**: Answering queries when selected (now always available)
