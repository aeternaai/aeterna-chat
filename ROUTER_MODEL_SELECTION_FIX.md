# Router Model Selection Bug - Fixed

## Problem

**User Query**: "switch to qwen 3-vl 8b"

**Expected Behavior**: Router selects Qwen3-VL-8B-Instruct-IQ4_XS

**Actual Behavior**: Router selects Phi-4-mini-instruct_Q4_K_M instead

**Router Response**:
```
modelId: "Phi-4-mini-instruct_Q4_K_M"
reasoning: "The Phi-4-mini-instruct_Q4_K_M model is not specified with an 8B size, but it is the closest match to the requested 3-VL 8B model..."
metadata: {parsed_index: 2, router_response: "3|..."}
```

## Root Cause Analysis

### Issue 1: Confusing Prompt Format

The LLM router was receiving a compact, technical prompt:

**OLD Format**:
```
Query: "switch to qwen 3-vl 8b"

Available models:
1. Qwen3-VL-8B-Instruct-IQ4_XS [llamacpp] - vision, chat | size=8B | ctx=4096 | cold
2. gemma-3n-E4B-it-IQ4_XS [llamacpp] - code, chat | size=unknown | ctx=4096 | cold
3. Phi-4-mini-instruct_Q4_K_M [llamacpp] - chat | size=unknown | ctx=4096 | loaded

Respond ONLY with '<number>|<reason>'
```

**Problems**:
- Model IDs are long and technical (e.g., `Qwen3-VL-8B-Instruct-IQ4_XS`)
- No clear human-readable names
- No explicit instruction to match user's model request
- Router model (Phi-4) doesn't understand it's making a selection mistake

### Issue 2: Auto-Add Order Confusion

When `allowed_models` is set to `Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS`:

1. Filter creates: `[Qwen3-VL, gemma]`
2. Auto-add inserts router model at END: `[Qwen3-VL, gemma, Phi-4]`
3. LLM responds with "3" (meaning 3rd model)
4. After -1 adjustment: index 2 → Phi-4 ❌

**The LLM should have said "1" for Qwen3-VL**, but the confusing format led it to select position 3.

### Issue 3: Model Name Matching

Phi-4 (the router model) tried to match "qwen 3-vl 8b" but got confused by:
- Technical suffixes: `-IQ4_XS`, `-Instruct`
- Model ID vs. friendly name mismatch
- Lack of explicit name matching instruction

## Solution Implemented

### 1. Improved Prompt Format

**NEW Format**:
```
User query: "switch to qwen 3-vl 8b"

Attachments: images=0, documents=0, has_code=False
User preferences: {"note": "none"}

Available models to choose from:

1. ID: Qwen3-VL-8B-Instruct-IQ4_XS
   Name: Qwen3-VL 8B (vision model)
   Capabilities: vision, chat
   Size: 8B
   Status: will need loading

2. ID: gemma-3n-E4B-it-IQ4_XS
   Name: Gemma 3B (coding model)
   Capabilities: code, chat
   Size: unknown
   Status: will need loading

3. ID: Phi-4-mini-instruct_Q4_K_M
   Name: Phi-4 Mini (general purpose)
   Capabilities: chat
   Size: unknown
   Status: loaded and ready

IMPORTANT: Analyze the user's query carefully. If they explicitly request a specific model by name
(e.g., 'use qwen', 'switch to gemma', 'use the vision model'), select that model.

Respond with ONLY: <number>|<brief reason>
Example: 1|User requested Qwen vision model
Example: 2|Coding task requires code-optimized model
```

**Improvements**:
- ✅ Multi-line format with clear sections
- ✅ Human-readable names: "Qwen3-VL 8B (vision model)"
- ✅ Explicit instruction to match user's model request
- ✅ Clear examples of expected responses
- ✅ Status shows "loaded and ready" vs "will need loading"

### 2. Enhanced Diagnostic Logging

**File**: `extensions/router-extension/src/index.ts`

Added logging to show exact models sent to Python router:
```typescript
console.log(`🐍 [RouterExtension] Sending ${filteredContext.availableModels.length} models to Python router:`)
filteredContext.availableModels.forEach((m, idx) => {
  console.log(`   ${idx + 1}. ${m.id} (${m.providerId}) - ${m.capabilities.join(', ')}`)
})
```

This shows:
- Exact order models are sent
- What index each model has
- What the LLM router will see

### 3. Better Response Logging

Added logging to show which model was selected and why:
```typescript
console.log(
  `[RouterExtension] Python router selected: ${decision.modelId} ` +
  `(index would be ${filteredContext.availableModels.findIndex(m => m.id === decision.modelId) + 1}) ` +
  `- ${decision.reasoning}`
)
console.log(`[RouterExtension] Decision metadata:`, decision.metadata)
```

Shows:
- Which model was selected
- What index it was at (1-based for comparison with LLM response)
- The reasoning provided
- Full metadata including parsed index

## Technical Details

### Prompt Building (`_build_routing_prompt`)

**File**: `src-tauri/router-service/strategies/llm.py`

**Changes**:
1. **Model Display Names**: Extract human-readable names from model IDs
   ```python
   if "Qwen" in model.id:
       model_display_name = "Qwen3-VL 8B (vision model)"
   elif "gemma" in model.id:
       model_display_name = "Gemma 3B (coding model)"
   elif "Phi-4" in model.id:
       model_display_name = "Phi-4 Mini (general purpose)"
   ```

2. **Multi-Line Format**: Each model gets 5 lines with clear labels
   ```python
   f"{idx}. ID: {model.id}\n"
   f"   Name: {model_display_name}\n"
   f"   Capabilities: {caps}\n"
   f"   Size: {metadata.parameter_count or 'unknown'}\n"
   f"   Status: {'loaded and ready' if metadata.is_loaded else 'will need loading'}"
   ```

3. **Explicit Instructions**: Clear guidance for model matching
   ```python
   "IMPORTANT: Analyze the user's query carefully. If they explicitly request a specific model by name "
   "(e.g., 'use qwen', 'switch to gemma', 'use the vision model'), select that model."
   ```

4. **Better Examples**: Show expected response format
   ```python
   "Example: 1|User requested Qwen vision model"
   "Example: 2|Coding task requires code-optimized model"
   ```

### Response Parsing (`_parse_router_response`)

**No changes needed** - Already handles:
- Format: `3|reason text`
- Extracts index (3 → 2 after -1)
- Validates bounds
- Falls back to index 0 if invalid

**Works correctly once LLM gives right number**!

## Expected Behavior After Fix

### Test Case 1: Explicit Model Request

**Input**: "switch to qwen 3-vl 8b"

**Expected Flow**:
1. Prompt shows: "1. ID: Qwen3-VL... Name: Qwen3-VL 8B (vision model)"
2. LLM sees "user wants qwen" + "option 1 is Qwen3-VL"
3. LLM responds: "1|User requested Qwen3-VL 8B vision model"
4. Parsed index: 0 (1-1=0)
5. Selected: `models[0]` = Qwen3-VL ✅

**Logs to Expect**:
```
🐍 [RouterExtension] Sending 3 models to Python router:
   1. Qwen3-VL-8B-Instruct-IQ4_XS (llamacpp) - vision, chat
   2. gemma-3n-E4B-it-IQ4_XS (llamacpp) - code, chat
   3. Phi-4-mini-instruct_Q4_K_M (llamacpp) - chat
[RouterExtension] Python router selected: Qwen3-VL-8B-Instruct-IQ4_XS (index would be 1) - User requested Qwen3-VL 8B vision model
```

### Test Case 2: Vision Task

**Input**: "describe this image" [with image attached]

**Expected Flow**:
1. Prompt shows: attachments.images=1
2. LLM sees "image task" + "option 1 has vision capability"
3. LLM responds: "1|Image description requires vision capability"
4. Selected: Qwen3-VL ✅

### Test Case 3: Coding Task

**Input**: "write a function to sort an array"

**Expected Flow**:
1. Prompt shows capabilities: "code, chat" for gemma
2. LLM sees "coding task" + "option 2 is coding model"
3. LLM responds: "2|Coding task best handled by code-optimized model"
4. Selected: gemma ✅

### Test Case 4: General Query

**Input**: "what is the capital of France?"

**Expected Flow**:
1. Simple question, no special requirements
2. LLM picks loaded model for faster response
3. LLM responds: "3|Simple query, using already loaded model"
4. Selected: Phi-4 ✅ (it's already loaded)

## Testing Instructions

### 1. Rebuild with Changes

```bash
make clean
make dev
```

### 2. Test Explicit Model Selection

Send query: "switch to qwen 3-vl 8b"

**Check console for**:
```
🐍 [RouterExtension] Sending X models to Python router:
   1. Qwen3-VL-8B-Instruct-IQ4_XS ...
```

**Verify**:
- Qwen3-VL is at position 1
- Router selects index 1
- Decision shows Qwen3-VL as modelId

### 3. Test Vision Task

Send query: "describe this beautiful landscape" [attach image]

**Verify**: Routes to Qwen3-VL (vision capability)

### 4. Test Coding Task

Send query: "write me a bubble sort in python"

**Verify**: Routes to gemma (code capability)

### 5. Test General Query

Send query: "hello, how are you?"

**Verify**: Routes to already loaded model (Phi-4 if it's the router)

## Fallback Behavior

If Python router fails, TypeScript heuristic router takes over:

```typescript
catch (error) {
  console.error('[RouterExtension] Python router failed, falling back to TypeScript:', error)
  // Uses HeuristicRouter which scores models based on:
  // - Capabilities match
  // - Parameter count (larger for complex tasks)
  // - Whether model is already loaded
}
```

## Configuration

Your current `allowed_models`: `Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS`

**This is correct** - it limits router to your preferred models, and Phi-4 (router model) is auto-added.

**To allow all models**: Clear the `allowed_models` field in Settings > Router

## Debugging Future Issues

If router makes wrong selections, check console logs for:

1. **Model Order**:
   ```
   🐍 [RouterExtension] Sending X models to Python router:
      1. <first model>
      2. <second model>
   ```

2. **Router Response**:
   ```
   [RouterExtension] Decision metadata: {parsed_index: X, router_response: "Y|reason"}
   ```

3. **Mismatch**: If `parsed_index` doesn't match expected model
   - Check what model is at that index
   - Review router's reasoning
   - Check if prompt format was clear

## Summary

**Before**: Compact technical format → LLM confused → Wrong selection
**After**: Clear multi-line format with names → LLM understands → Correct selection

The fix makes the routing prompt **human-readable** and gives **explicit instructions** for model name matching, which should make Phi-4 (the router model) much better at selecting the right target model!
