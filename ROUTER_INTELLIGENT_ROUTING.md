# Router Intelligent Routing Implementation

## Overview

Upgraded the router system from simple model allowlists to intelligent routing with model-specific descriptions. This enables the router to match user queries to models based on their intended purpose.

## Architecture Changes

### Previous System (Allowed Models List)
```json
{
  "key": "allowed_models",
  "value": "Qwen3-VL-8B-Instruct-IQ4_XS,gemma-3n-E4B-it-IQ4_XS"
}
```
- Simple comma-separated string
- No context about model capabilities
- Generic routing based on technical capabilities

### New System (Routing Configurations)
```json
{
  "key": "model_routing_configs",
  "value": "[{\"id\":\"Qwen3-VL-8B-Instruct-IQ4_XS\",\"description\":\"Vision and image understanding tasks. Use for analyzing images, describing visual content, OCR, and any query involving pictures or visual data.\"},{\"id\":\"gemma-3n-E4B-it-IQ4_XS\",\"description\":\"Coding, programming, and technical documentation tasks. Use for writing code, debugging, explaining technical concepts, and software development.\"}]"
}
```
- Structured JSON array with `{id, description}` objects
- Explicit descriptions of when to use each model
- Intelligent routing based on query-description matching

## Implementation Details

### 1. Settings Format Change

**File:** `extensions/router-extension/settings.json`

Changed from:
- Key: `allowed_models`
- Value: Comma-separated model IDs

To:
- Key: `model_routing_configs`
- Value: JSON array of `ModelRoutingConfig` objects

**Example Configuration:**
```json
[
  {
    "id": "Qwen3-VL-8B-Instruct-IQ4_XS",
    "description": "Vision and image understanding tasks. Use for analyzing images, describing visual content, OCR, and any query involving pictures or visual data."
  },
  {
    "id": "gemma-3n-E4B-it-IQ4_XS",
    "description": "Coding, programming, and technical documentation tasks. Use for writing code, debugging, explaining technical concepts, and software development."
  },
  {
    "id": "Phi-4-IQ4_XS",
    "description": "General purpose queries, creative writing, explanations, and reasoning tasks. Use for general knowledge questions, creative content, and conversational interactions."
  }
]
```

### 2. Python Service Updates

**File:** `src-tauri/router-service/models.py`

Added new model class:
```python
class ModelRoutingConfig(BaseModel):
    id: str
    description: str
```

Updated `RouteRequest`:
```python
class RouteRequest(BaseModel):
    # ... existing fields ...
    model_routing_configs: Optional[list[ModelRoutingConfig]] = Field(
        None, 
        alias="modelRoutingConfigs"
    )
```

**File:** `src-tauri/router-service/strategies/llm.py`

Updated `_build_routing_prompt()` to use routing descriptions:
```python
def _build_routing_prompt(self, query, models, model_routing_configs=None, ...):
    # Build routing descriptions map
    routing_descriptions = {}
    if model_routing_configs:
        for config in model_routing_configs:
            routing_descriptions[config.id] = config.description
    
    # For each model:
    model_description = routing_descriptions.get(model.id, default_description)
    prompt += f"When to use: {model_description}\n"
    
    # Enhanced instructions:
    "Match the query to the 'When to use' description for each model"
```

Updated `route()` method:
```python
prompt = self._build_routing_prompt(
    query=query,
    models=request.available_models,
    model_routing_configs=request.model_routing_configs,  # NEW
    attachments=request.attachments,
    preferences=request.preferences,
)
```

### 3. Router Extension Updates

**File:** `extensions/router-extension/src/index.ts`

Changed property type:
```typescript
// Before:
private allowedModels: string[] = []

// After:
private modelRoutingConfigs: Array<{id: string, description: string}> = []
```

New parsing function:
```typescript
private parseRoutingConfigs(configsStr: string): Array<{id: string, description: string}> {
  if (!configsStr || configsStr.trim() === '') {
    return []
  }
  try {
    const configs = JSON.parse(configsStr)
    if (!Array.isArray(configs)) {
      console.warn('[RouterExtension] Invalid routing configs format, expected array:', configsStr)
      return []
    }
    return configs.filter((config) => 
      config && typeof config.id === 'string' && typeof config.description === 'string'
    )
  } catch (error) {
    console.error('[RouterExtension] Failed to parse routing configs:', error)
    return []
  }
}
```

Updated filtering logic:
```typescript
private filterAllowedModels(availableModels: any[]): any[] {
  // If no routing configs configured, return all available models
  if (this.modelRoutingConfigs.length === 0) {
    return availableModels
  }

  // Filter models to only include those with routing configs
  const allowedIds = this.modelRoutingConfigs.map(config => config.id)
  return availableModels.filter((model) => allowedIds.includes(model.id))
}
```

Pass configs to Python router:
```typescript
const decision = await invoke('route_request', {
  request: {
    messages: filteredContext.messages,
    threadId: filteredContext.threadId,
    availableModels: filteredContext.availableModels,
    routerModel: filteredContext.routerModel,
    activeModels: filteredContext.activeModels,
    attachments: filteredContext.attachments,
    preferences: filteredContext.preferences,
    modelRoutingConfigs: this.modelRoutingConfigs,  // NEW
  }
})
```

Updated `onSettingUpdate()`:
```typescript
onSettingUpdate<T>(key: string, value: T): void {
  if (key === 'model_routing_configs') {
    this.modelRoutingConfigs = this.parseRoutingConfigs(value as string)
    console.log('[RouterExtension] Updated model routing configs:', this.modelRoutingConfigs)
  }
}
```

## How It Works

### Routing Process

1. **Configuration Loading**
   - Router extension loads `model_routing_configs` from settings on startup
   - Parses JSON array into TypeScript objects
   - Stores as `modelRoutingConfigs` property

2. **Runtime Updates**
   - User modifies settings in UI
   - `onSettingUpdate()` hook triggered
   - Configs immediately updated without restart

3. **Model Filtering**
   - Available models filtered based on routing configs
   - Only models with explicit routing configs are available

4. **Intelligent Routing**
   - User query sent to Python router
   - Routing configs passed with request
   - LLM prompt includes "When to use: {description}" for each model
   - Router matches query intent to model descriptions

### Example Routing Scenarios

**Query:** "Show me what's in this image"
- **Matches:** Qwen3-VL description "Vision and image understanding tasks..."
- **Selected:** Qwen3-VL-8B-Instruct-IQ4_XS

**Query:** "Write a Python function to sort a list"
- **Matches:** gemma description "Coding, programming, and technical documentation..."
- **Selected:** gemma-3n-E4B-it-IQ4_XS

**Query:** "Explain quantum physics simply"
- **Matches:** Phi-4 description "General purpose queries, creative writing, explanations..."
- **Selected:** Phi-4-IQ4_XS

## Benefits

1. **Explicit Model Purpose** - Each model has clear use case documentation
2. **Intelligent Matching** - Router considers descriptions, not just capabilities
3. **User Control** - Fine-grained control over model routing with natural language
4. **Better UX** - More accurate model selection improves response quality
5. **Runtime Updates** - Settings changes take effect immediately

## Testing

### Manual Testing Checklist

1. **Settings Persistence**
   - [ ] Modify `model_routing_configs` in UI
   - [ ] Restart app
   - [ ] Verify configs persisted to `settings.json`

2. **Model Filtering**
   - [ ] Configure routing for 2 models only
   - [ ] Verify only those 2 models available for routing
   - [ ] Add 3rd model to configs
   - [ ] Verify it appears without restart

3. **Intelligent Routing**
   - [ ] Send vision query → Should select Qwen
   - [ ] Send coding query → Should select gemma
   - [ ] Send general query → Should select Phi-4
   - [ ] Check console logs for routing reasoning

4. **Edge Cases**
   - [ ] Test with empty configs (should allow all models)
   - [ ] Test with invalid JSON (should fallback gracefully)
   - [ ] Test with missing model IDs (should filter them out)

### Console Log Validation

Look for these key logs:

```
[RouterExtension] Model routing configs: [
  {id: "Qwen3-VL-8B-Instruct-IQ4_XS", description: "Vision and image understanding tasks..."},
  {id: "gemma-3n-E4B-it-IQ4_XS", description: "Coding, programming..."}
]

🐍 [RouterExtension] Sending 2 response models to Python router:
   1. Qwen3-VL-8B-Instruct-IQ4_XS (llamacpp) - vision
   2. gemma-3n-E4B-it-IQ4_XS (llamacpp) - coding

[RouterExtension] Python router selected: gemma-3n-E4B-it-IQ4_XS (index would be 2) - 
Query requests code generation, matching gemma's coding expertise
```

## Migration Guide

### For Users

**Old Configuration:**
```json
{
  "key": "allowed_models",
  "value": "model1,model2,model3"
}
```

**New Configuration:**
```json
{
  "key": "model_routing_configs",
  "value": "[{\"id\":\"model1\",\"description\":\"Use for...\"},{\"id\":\"model2\",\"description\":\"Use for...\"}]"
}
```

### Tips for Writing Descriptions

- Be specific about use cases
- Start with action verbs ("Use for...", "Optimized for...")
- Include multiple scenarios per model
- Mention key capabilities (vision, coding, reasoning, etc.)
- Keep concise but informative (1-2 sentences)

**Good Examples:**
- ✅ "Vision and image understanding tasks. Use for analyzing images, describing visual content, OCR, and any query involving pictures or visual data."
- ✅ "Coding, programming, and technical documentation tasks. Use for writing code, debugging, explaining technical concepts, and software development."

**Poor Examples:**
- ❌ "General AI model" (too vague)
- ❌ "Qwen model for stuff" (not descriptive)
- ❌ "Use this for everything" (defeats routing purpose)

## Related Documents

- `ROUTER_IMPLEMENTATION_COMPLETE.md` - Initial router implementation
- `ROUTER_ARCHITECTURE_REFACTOR.md` - Option C: Separate Lists architecture
- `ROUTER_SETTINGS_FINAL_FIX.md` - Settings persistence fixes

## Files Modified

### TypeScript/JavaScript
- `extensions/router-extension/src/index.ts` - Parsing and filtering logic
- `extensions/router-extension/settings.json` - Configuration format

### Python
- `src-tauri/router-service/models.py` - ModelRoutingConfig class
- `src-tauri/router-service/strategies/llm.py` - Prompt building with descriptions

## Status

✅ **COMPLETE** - All components updated, extensions rebuilt successfully

**Build Output:**
- Router extension: 46.73 kB (increased from 33.36 kB due to new parsing logic)
- All extensions built successfully
- No compilation errors
