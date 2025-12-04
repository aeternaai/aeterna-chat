# Router Diagnostic Logging

## Problem

User reports: "I cannot select Qwen3-VL-8B-Instruct-IQ4_XS anymore"

## Diagnostic Logging Added

### 1. Model Selection Logging

**File**: `web-app/src/hooks/useChat.ts`

Added comprehensive logging in `buildAvailableModels()`:
- Number of providers being processed
- Active model IDs list
- Each provider's active status and model count
- Each model's loaded status
- Final available models count and IDs

**Logs to Watch For**:
```
[buildAvailableModels] Building with X providers
[buildAvailableModels] Active model IDs: [...]
[buildAvailableModels] Provider: llamacpp, active: true/false, models: X
[buildAvailableModels] Skipping inactive provider: ... (if inactive)
[buildAvailableModels]   Model: Qwen3-VL-8B-Instruct-IQ4_XS, loaded: true/false
[buildAvailableModels] Built X available models
[buildAvailableModels] Model IDs: [...]
```

### 2. Router Filtering Logging

**File**: `extensions/router-extension/src/index.ts`

Added detailed logging in `route()`:
- Input available models before filtering
- Allowed models filter configuration
- Models after filtering
- Router model auto-add logic
- Final available models for routing

**Logs to Watch For**:
```
[RouterExtension] Routing request...
[RouterExtension] Input available models (X): [...]
[RouterExtension] Allowed models filter: [...]
[RouterExtension] After filtering (X): [...]
[RouterExtension] Auto-added router model ... (if applicable)
[RouterExtension] Final available models (X): [...]
```

## How to Use

### Step 1: Build and Run

```bash
make clean
make dev
```

### Step 2: Monitor Console

Open browser DevTools (F12) and watch console logs.

### Step 3: Send a Query

Type any message and send it. Watch the logs to see:

1. **Which models are being built into availableModels**
   - Is Qwen3-VL in the list?
   - Is the llamacpp provider active?
   
2. **How router filtering works**
   - Does Qwen3-VL make it through the filter?
   - What's in the allowed_models configuration?

## Potential Issues to Check

### Issue 1: Provider Not Active

**Symptom**: Log shows `Skipping inactive provider: llamacpp`

**Cause**: The llamacpp provider's `active` flag is false

**Fix**: Check why llamacpp provider became inactive. Possible causes:
- Extension failed to load
- Provider initialization error
- State management issue

### Issue 2: Allowed Models Filter

**Symptom**: Log shows Qwen3-VL in input but not in filtered output

**Cause**: `allowed_models` setting excludes Qwen3-VL

**Fix**: 
1. Check router settings (`Settings > Router`)
2. Verify `allowed_models` field
3. Either:
   - Add `Qwen3-VL-8B-Instruct-IQ4_XS` to the list
   - OR clear the field to allow all models

### Issue 3: Model Not Downloaded

**Symptom**: Qwen3-VL not in provider.models list

**Cause**: Model was deleted or never downloaded

**Fix**: Re-download Qwen3-VL from model hub

### Issue 4: Multi-Session Duplicate IDs

**Symptom**: `activeModelIds` shows duplicates like `['Phi-4', 'Phi-4', 'Qwen3']`

**Cause**: Multi-session support returns duplicate model IDs when same model loaded twice

**Impact**: Should NOT affect `.includes()` check, but might indicate unexpected behavior

**Investigation**: Check if this causes any array length or filtering issues

## Expected Behavior

### Normal Flow

```
1. buildAvailableModels() called
   ├─ Provider: llamacpp, active: true, models: 3
   ├─ Model: Phi-4-mini-instruct_Q4_K_M, loaded: true
   ├─ Model: Qwen3-VL-8B-Instruct-IQ4_XS, loaded: false
   └─ Model: gemma-3n-E4B-it-IQ4_XS, loaded: false
   Result: 3 models available

2. Router.route() called
   ├─ Input: [Phi-4, Qwen3-VL, gemma-3n]
   ├─ Filter: [] (empty = all allowed)
   └─ Output: [Phi-4, Qwen3-VL, gemma-3n]
   Result: All models passed through

3. Routing decision made
   └─ Selected: Qwen3-VL (for vision task)
```

### With Allowed Models Filter

```
1. buildAvailableModels() called
   Result: [Phi-4, Qwen3-VL, gemma-3n]

2. Router.route() called
   ├─ Input: [Phi-4, Qwen3-VL, gemma-3n]
   ├─ Filter: ['Qwen3-VL-8B-Instruct-IQ4_XS', 'gemma-3n-E4B-it-IQ4_XS']
   ├─ After filter: [Qwen3-VL, gemma-3n]
   ├─ Router model check: Phi-4 exists but not in filtered → auto-add
   └─ Final: [Qwen3-VL, gemma-3n, Phi-4]
   Result: Qwen3-VL available for routing
```

## Debugging Steps

### 1. Check Provider Status

Look for:
```
[buildAvailableModels] Provider: llamacpp, active: false
```

If llamacpp is inactive, that's the root cause.

**Next steps**:
- Check extension loading errors
- Verify provider initialization
- Check for provider activation logic

### 2. Check Model Existence

Look for Qwen3-VL in:
```
[buildAvailableModels] Model IDs: [...]
```

If not present, check:
- Is model downloaded?
- Is model in provider's model list?
- Did model scan fail?

### 3. Check Filter Configuration

Look for:
```
[RouterExtension] Allowed models filter: [...]
```

If Qwen3-VL is not in this list (and list is not empty), it will be filtered out.

**Next steps**:
- Check Settings > Router > Allowed Models
- Verify comma-separated format
- Check for typos in model ID

### 4. Check Final Available Models

Look for:
```
[RouterExtension] Final available models (X): [...]
```

If Qwen3-VL is missing here, trace back through the logs to see where it was removed.

## Common Scenarios

### Scenario 1: Model Not Found

```
[buildAvailableModels] Provider: llamacpp, active: true, models: 2
[buildAvailableModels]   Model: Phi-4-mini-instruct_Q4_K_M
[buildAvailableModels]   Model: gemma-3n-E4B-it-IQ4_XS
[buildAvailableModels] Model IDs: [Phi-4, gemma-3n]
```

**Diagnosis**: Qwen3-VL not in model list
**Solution**: Re-download model

### Scenario 2: Provider Inactive

```
[buildAvailableModels] Provider: llamacpp, active: false, models: 3
[buildAvailableModels] Skipping inactive provider: llamacpp
[buildAvailableModels] Model IDs: []
```

**Diagnosis**: llamacpp provider not active
**Solution**: Check extension initialization

### Scenario 3: Filtered Out

```
[RouterExtension] Input available models: [Phi-4, Qwen3-VL, gemma-3n]
[RouterExtension] Allowed models filter: [gemma-3n-E4B-it-IQ4_XS]
[RouterExtension] After filtering: [gemma-3n]
```

**Diagnosis**: Allowed models excludes Qwen3-VL
**Solution**: Update allowed_models setting

## Next Steps After Diagnosis

Once you identify the issue from the logs, report back with:

1. **What logs appeared** when you sent a message
2. **Which scenario** matches your situation
3. **Specific values** from the logs (provider active status, model IDs, filter config)

This will help pinpoint the exact fix needed.

## Removing Diagnostic Logs

Once the issue is resolved, we can remove the verbose logging:

1. Remove console.log statements from `buildAvailableModels()`
2. Remove detailed logging from router extension's `route()`
3. Keep minimal error logging for production use
