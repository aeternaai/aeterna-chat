# JIRA MCP Schema Issue - Complete Analysis

## Executive Summary

The JIRA ROVO MCP server is successfully connected and responding to health checks, but llama.cpp fails to use the tools due to **incompatible JSON schemas** in the tool definitions. This is NOT an OAuth issue - authentication works fine. The problem is schema format incompatibility.

## Current Status

### ✅ What's Working
1. OAuth authentication completed successfully
2. MCP server connected (process 98831 running)
3. Health checks passing every 5 seconds
4. `tools/list` requests and responses working
5. Tools are being retrieved from the server

### ❌ What's Broken
1. llama.cpp cannot parse the tool schemas
2. Chat requests fail with schema conversion error
3. JIRA tools unusable in conversations

## The Error Explained

```
JSON schema conversion failed:
Unrecognized schema: {"description":"This is the field value. The actual value will depends on the field type."}
```

### What This Means

**The Problem**: JIRA ROVO returns **generic/dynamic schemas** for some tool parameters because JIRA fields can be any type (string, number, array, object, custom field, etc.). The MCP server doesn't know the exact type ahead of time.

**llama.cpp's Requirement**: Function calling in llama.cpp requires **explicit, well-defined JSON schemas** with specific types per the OpenAI function calling spec.

### Example of the Issue

**JIRA ROVO might return**:
```json
{
  "name": "jira-update-issue",
  "description": "Update a JIRA issue",
  "inputSchema": {
    "type": "object",
    "properties": {
      "issueKey": {
        "type": "string",
        "description": "The issue key (e.g., PROJ-123)"
      },
      "fields": {
        "description": "This is the field value. The actual value will depends on the field type."
        // ❌ MISSING: "type" property
      }
    }
  }
}
```

**llama.cpp expects**:
```json
{
  "name": "jira-update-issue",
  "description": "Update a JIRA issue",
  "inputSchema": {
    "type": "object",
    "properties": {
      "issueKey": {
        "type": "string",
        "description": "The issue key (e.g., PROJ-123)"
      },
      "fields": {
        "type": "object",  // ✅ Required
        "description": "Field updates",
        "additionalProperties": true  // Allow any fields
      }
    },
    "required": ["issueKey"]
  }
}
```

## Why Health Checks Show No Tools

Looking at the health check code:

```rust
// src-tauri/src/core/mcp/helpers.rs:245
match timeout(Duration::from_secs(30), service.list_all_tools()).await {
    Ok(Ok(_)) => {
        // Server responded successfully
        true  // ❌ Tools are discarded!
    }
    // ...
}
```

**The health check doesn't log or return the tools** - it only checks if `list_all_tools()` succeeds. The tools ARE being returned (messages 37, 38, 39, 40 in your logs), but the health check just verifies the call doesn't fail.

### Where Tools Are Actually Retrieved

Tools are fetched for use via:
1. **Frontend**: `useTools` hook → calls `getServiceHub().mcp().getTools()`
2. **Tauri Command**: `get_tools` command → calls `list_all_tools()` on each server
3. **Chat Flow**: Tools passed to llama.cpp when building chat completion request

From `src-tauri/src/core/mcp/commands.rs`:
```rust
pub async fn get_tools(state: State<'_, AppState>) -> Result<Vec<ToolWithServer>, String> {
    let servers = state.mcp_servers.lock().await;
    let mut all_tools: Vec<ToolWithServer> = Vec::new();

    for (server_name, service) in servers.iter() {
        let tools = match timeout(timeout_duration, service.list_all_tools()).await {
            Ok(result) => result.map_err(|e| e.to_string())?,
            Err(_) => continue,
        };

        for tool in tools {
            all_tools.push(ToolWithServer {
                name: tool.name.to_string(),
                description: tool.description.as_ref().map(|d| d.to_string()),
                input_schema: serde_json::Value::Object((*tool.input_schema).clone()),  // Schema passed as-is
                server: server_name.clone(),
            });
        }
    }
    Ok(all_tools)
}
```

**The schema is passed through unchanged** from the MCP server to the frontend to llama.cpp.

## The Complete Data Flow

```
JIRA ROVO MCP Server
    ↓ (returns tools with generic schemas)
Rust: get_tools command
    ↓ (passes schemas unchanged)
Frontend: useTools hook → updateTools
    ↓ (stores in app state)
Chat: sendMessage → sendCompletion
    ↓ (normalizes to OpenAI format)
normalizeTools function
    ↓ (maps to function calling format)
llama.cpp extension → engine.chat()
    ↓ (sends to llama.cpp server)
llama.cpp server
    ❌ FAILS: "Unrecognized schema"
```

## Tool Schema Normalization

From `web-app/src/lib/completion.ts`:
```typescript
export const normalizeTools = (
  tools: MCPTool[]
): ChatCompletionTool[] | Tool[] | undefined => {
  if (tools.length === 0) return undefined
  return tools.map((tool) => ({
    type: 'function',
    function: {
      name: tool.name,
      description: tool.description?.slice(0, 1024),
      parameters: tool.inputSchema,  // ❌ Schema passed unchanged
      strict: false,
    },
  }))
}
```

**The bug**: `tool.inputSchema` is passed directly without validation or transformation.

## Solutions (Ranked by Effort/Impact)

### Option 1: Schema Sanitization (Recommended) ⭐

**Where**: Modify `normalizeTools` function in `web-app/src/lib/completion.ts`

**What**: Add schema validation and fix incomplete schemas before passing to llama.cpp

```typescript
function sanitizeSchema(schema: Record<string, unknown>): Record<string, unknown> {
  const sanitized = { ...schema }
  
  // Ensure all properties have types
  if (sanitized.properties && typeof sanitized.properties === 'object') {
    for (const [key, value] of Object.entries(sanitized.properties as Record<string, any>)) {
      if (value && typeof value === 'object' && !value.type) {
        // Add default type for properties missing it
        value.type = 'object'
        value.additionalProperties = true
      }
    }
  }
  
  return sanitized
}

export const normalizeTools = (tools: MCPTool[]) => {
  return tools.map((tool) => ({
    type: 'function',
    function: {
      name: tool.name,
      description: tool.description?.slice(0, 1024),
      parameters: sanitizeSchema(tool.inputSchema),  // ✅ Fix schemas
      strict: false,
    },
  }))
}
```

**Pros**: 
- Fixes the immediate issue
- Works for any MCP server with schema issues
- Minimal code change

**Cons**: 
- May not preserve exact JIRA field semantics
- Generic fix might not be perfect for all tools

### Option 2: Skip Invalid Tools

**Where**: Same location, filter tools with invalid schemas

```typescript
export const normalizeTools = (tools: MCPTool[]) => {
  const validTools = tools.filter((tool) => {
    try {
      // Validate schema has required properties
      const schema = tool.inputSchema
      if (!schema.properties) return false
      
      for (const prop of Object.values(schema.properties as any)) {
        if (!prop.type) return false  // Reject tools with missing types
      }
      return true
    } catch {
      return false
    }
  })
  
  return validTools.map(/* ... */)
}
```

**Pros**: 
- Safe - only uses tools that work
- No risk of incorrect schema transformation

**Cons**: 
- JIRA tools completely unusable
- Doesn't solve the problem, just hides it

### Option 3: Rust-Level Schema Transformation

**Where**: `src-tauri/src/core/mcp/commands.rs` in `get_tools`

**What**: Fix schemas when retrieving tools from servers

```rust
fn sanitize_input_schema(schema: &mut serde_json::Value) {
    if let Some(properties) = schema.get_mut("properties").and_then(|p| p.as_object_mut()) {
        for (_key, value) in properties.iter_mut() {
            if let Some(obj) = value.as_object_mut() {
                if !obj.contains_key("type") {
                    // Add default type for properties without one
                    obj.insert("type".to_string(), json!("object"));
                    obj.insert("additionalProperties".to_string(), json!(true));
                }
            }
        }
    }
}

pub async fn get_tools(...) -> Result<Vec<ToolWithServer>, String> {
    // ... existing code ...
    for tool in tools {
        let mut input_schema = serde_json::Value::Object((*tool.input_schema).clone());
        sanitize_input_schema(&mut input_schema);  // ✅ Fix schema
        
        all_tools.push(ToolWithServer {
            input_schema,
            // ... rest ...
        });
    }
}
```

**Pros**: 
- Fixes at the source before frontend sees it
- One place to handle all schema issues

**Cons**: 
- Requires Rust changes
- Rebuild time longer

### Option 4: Use Different Inference Backend

**What**: Try OpenAI-compatible API or other backends that might be more tolerant

**Pros**: 
- Might work without code changes

**Cons**: 
- Doesn't fix the root cause
- May not have local models

## Recommended Implementation Plan

### Phase 1: Quick Fix (Option 1)
1. Add `sanitizeSchema` function to `web-app/src/lib/completion.ts`
2. Apply it in `normalizeTools`
3. Test with JIRA ROVO
4. Verify tools work in chat

### Phase 2: Robust Solution (Option 3)
1. Move schema sanitization to Rust
2. Add comprehensive schema validation
3. Log warnings for transformed schemas
4. Add unit tests for edge cases

### Phase 3: Upstream Fix
1. Report issue to JIRA ROVO MCP team
2. Suggest they include explicit types in schemas
3. Or add schema version negotiation to MCP spec

## Testing Plan

1. **Verify tools are retrieved**:
   - Add logging in `get_tools` command
   - Log tool count and names
   - Check for JIRA tools in list

2. **Check schema structure**:
   - Log `tool.inputSchema` before normalization
   - Identify which tools have missing types
   - Document JIRA's schema format

3. **Test schema sanitization**:
   - Add unit tests for `sanitizeSchema`
   - Test with JIRA-like schemas
   - Verify llama.cpp accepts transformed schemas

4. **Integration test**:
   - Send chat message requiring JIRA tool
   - Verify llama.cpp doesn't error
   - Confirm tool call is generated
   - Check JIRA API is called

## Next Steps

Would you like me to:

1. **Implement Option 1** (schema sanitization in TypeScript)?
2. **Add detailed logging** to see exact tool schemas from JIRA?
3. **Implement Option 3** (Rust-level schema fixing)?
4. **Create a diagnostic command** to dump all tool schemas to a file for inspection?

Let me know which approach you prefer, and I'll implement it!
