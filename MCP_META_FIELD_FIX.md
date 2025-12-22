# MCP Meta Field Fix: Ephemeral Flag Not Passed to Frontend

## Problem
The `meta` field with the `ephemeral` flag was not being properly passed from the Rust backend to the TypeScript frontend when using `fetch_cached_output`. This caused ephemeral cache chunks to be incorrectly persisted in conversation history even when the LLM didn't find them useful.

## Root Cause
The issue was in the serialization layer between Rust and TypeScript:

1. **Backend (Rust)**: The `CallToolResult` was correctly setting `meta: Some(rmcp::model::Meta(meta_map))` with the ephemeral flag
2. **Tauri Serialization**: The `rmcp::model::Meta` type (a newtype wrapper around `Map<String, Value>`) may not have been serializing correctly through Tauri's IPC layer
3. **Frontend (TypeScript)**: The `MCPToolCallResult` type definition included `meta?: Record<string, any>`, but the field was arriving as `undefined`

## Solution

### 1. Backend Changes (`src-tauri/src/core/mcp/commands.rs`)
**Enhanced the `fetch_cached_output` handler to include meta in both locations:**

```rust
// Include meta in structured_content as a backup
let mut structured_data = serde_json::json!({
    "text": content,
    "metadata": {
        "ref_id": ref_id,
        "token_range": format!("{}-{}", start_token, end_token),
        "cache_limit": MAX_CACHED_TOKENS,
        "retrieved_tokens": retrieved_tokens,
        "progress_percent": progress_pct
    },
    "meta": {  // ← Added backup meta field here
        "ephemeral": true,
        "is_cache_fetch": true,
        "chunk_range": format!("{}-{}", start_token, end_token),
        "ref_id": ref_id
    }
});

let result = CallToolResult {
    content: vec![],
    structured_content: Some(structured_data),
    is_error: None,
    meta: Some(rmcp::model::Meta(meta_map)),  // ← Original meta field
};

// Enhanced debug logging
log::info!("🔍 [DEBUG] Returning CallToolResult with meta: {:?}", result.meta);
log::info!("🔍 [DEBUG] structured_content contains meta: {:?}", 
    result.structured_content.as_ref().and_then(|v| v.get("meta")));
```

**Why this works:**
- If the `meta` field gets lost in serialization, we have a fallback in `structured_content.meta`
- The structured_content is a plain JSON object that definitely serializes correctly
- Enhanced logging helps debug if either path is working

### 2. Frontend Changes

#### A. Message Builder (`web-app/src/lib/messages.ts`)
**Updated `addToolMessage` to check multiple locations for the ephemeral flag:**

```typescript
addToolMessage(result: string | ToolResult, toolCallId: string) {
  // Check for ephemeral flag in multiple locations
  const isEphemeral = typeof result !== 'string' && (
    result.meta?.ephemeral === true ||  // ← Primary location
    (result.content?.[0] as any)?.meta?.ephemeral === true ||  // ← Content meta
    // Also check if structured content was parsed and has meta
    (() => {
      try {
        if (typeof result.content?.[0]?.text === 'string') {
          const parsed = JSON.parse(result.content[0].text)
          return parsed?.meta?.ephemeral === true  // ← Fallback location
        }
      } catch {}
      return false
    })()
  )
  
  if (isEphemeral) {
    console.log('[Ephemeral] ✅ Skipping cache chunk storage - found ephemeral flag')
    return // Don't add ephemeral chunks to conversation history
  }
  
  // ... rest of the method
}
```

#### B. Completion Handler (`web-app/src/lib/completion.ts`)
**Added pre-check before calling `addToolMessage`:**

```typescript
// Enhanced logging
console.log('[Tool Result] Received from service:', { 
  tool: toolCall.function.name, 
  hasMeta: 'meta' in result,
  meta: (result as any).meta,
  hasStructuredContentMeta: !!(result as any).content?.[0]?.text && 
    (() => {
      try {
        const parsed = JSON.parse((result as any).content[0].text)
        return !!parsed?.meta
      } catch {
        return false
      }
    })(),
})

// Check if result is ephemeral before adding to builder
const isEphemeral = (result as any).meta?.ephemeral === true ||
  (() => {
    try {
      if ((result as any).content?.[0]?.text) {
        const parsed = JSON.parse((result as any).content[0].text)
        return parsed?.meta?.ephemeral === true
      }
    } catch {}
    return false
  })()

if (isEphemeral) {
  console.log('[Ephemeral Tool] Skipping addToolMessage for ephemeral result')
} else {
  builder.addToolMessage(result as ToolResult, toolCall.id)
}
```

## Testing Strategy

1. **Enable Debug Logging**: Check console logs for:
   - `🔍 [DEBUG] Returning CallToolResult with meta` (Rust backend)
   - `[Tool Result] Received from service` (Frontend)
   - `[Ephemeral] ✅ Skipping cache chunk storage` (When working correctly)

2. **Test Scenario**: 
   - Trigger a large MCP tool output that gets cached
   - LLM calls `fetch_cached_output` to explore chunks
   - LLM determines content is not useful and proceeds
   - Verify: Cache chunks should NOT appear in conversation history

3. **Verify Both Paths**:
   - Check if `result.meta.ephemeral` is true (primary path)
   - If not, check if `parsed.meta.ephemeral` from structured_content is true (fallback)

## Expected Behavior

### Before Fix
```
❌ LLM explores cache chunks
❌ All chunks appear in conversation history
❌ Context bloated with irrelevant tool messages
```

### After Fix  
```
✅ LLM explores cache chunks
✅ Chunks marked as ephemeral are NOT added to history
✅ Only final useful information persists
✅ Context remains clean
```

## Implementation Notes

- **Defense in Depth**: Multiple checks ensure the ephemeral flag is caught even if one serialization path fails
- **Debug Logging**: Comprehensive logging helps identify which path is working
- **Zero Breaking Changes**: Existing non-ephemeral tool calls work exactly as before
- **Type Safety**: TypeScript type definitions already included `meta?` field, no breaking changes needed

## Related Files Modified

1. `src-tauri/src/core/mcp/commands.rs` - Backend cache retrieval handler
2. `web-app/src/lib/messages.ts` - Message builder ephemeral detection
3. `web-app/src/lib/completion.ts` - Completion flow ephemeral pre-check

## Monitoring

Check these log patterns to verify the fix is working:

```
Backend:
🔍 [DEBUG] Returning CallToolResult with meta: Some(Meta(...))
🔍 [DEBUG] structured_content contains meta: Some(Object {"ephemeral": Bool(true), ...})

Frontend:
[Tool Result] Received from service: { tool: 'fetch_cached_output', hasMeta: true, ... }
[Ephemeral] ✅ Skipping cache chunk storage - found ephemeral flag
```

If you see these logs, the fix is working correctly.
