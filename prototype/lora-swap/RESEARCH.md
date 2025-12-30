# Llama.cpp LoRA Support Research

## Research Status: Initial Investigation

**Date**: December 29, 2025  
**Target Version**: b6399+ (used by Jan)

## Known Facts from Jan's Codebase

### 1. OpenAPI Schema (Evidence of LoRA Support)

**File**: `extensions/llamacpp-extension/resources/inference-openapi.json:474-488`

```json
"lora": {
  "type": "array",
  "description": "LoRA adapters to apply for this request.",
  "items": {
    "type": "object",
    "properties": {
      "id": { "type": "integer" },
      "scale": { "type": "number" }
    },
    "required": ["id", "scale"]
  }
}
```

**Implications**:
- The OpenAPI schema suggests llama.cpp's HTTP API supports a `lora` field in `/v1/chat/completions` requests
- Format: Array of objects with `id` (integer) and `scale` (float)
- This indicates **per-request LoRA switching** is intended

### 2. Current Jan Implementation

**Status**: ❌ Not implemented
- Schema exists but no code uses it
- No LoRA loading in `LlamacppExtension`
- No Tauri commands for LoRA management
- No UI for adapter configuration

### 3. Command-Line Arguments (Expected)

Based on standard llama.cpp patterns, we expect these CLI args:

```bash
--lora <path>              # Load LoRA adapter at startup
--lora-scaled <path> <scale>  # Load with custom scale (default: 1.0)
```

**Multiple adapters**: Unknown if multiple `--lora` flags are supported

## Research Questions to Answer

### Critical Questions:

1. **Does llama-server support `--lora` CLI argument?**
   - Test: Start with `--lora /path/to/adapter.gguf`
   - Expected: Server loads adapter at startup
   - Status: ⏳ TO TEST

2. **Does `/v1/chat/completions` accept `lora` field?**
   - Test: Send request with `{"lora": [{"id": 0, "scale": 1.0}]}`
   - Expected: Request uses specified adapter
   - Status: ⏳ TO TEST

3. **Are there runtime `/lora/*` endpoints?**
   - Test: POST to `/lora/load`, `/lora/unload`, GET `/lora/list`
   - Expected: May not exist (likely future feature)
   - Status: ⏳ TO TEST

4. **How are adapter IDs assigned?**
   - Hypothesis: Sequential (0, 1, 2...) based on load order
   - Test: Load multiple adapters, check IDs
   - Status: ⏳ TO TEST

5. **Can multiple adapters be active simultaneously?**
   - Hypothesis: Yes, with different scales
   - Test: Load 2-3 adapters, use in same request
   - Status: ⏳ TO TEST

6. **Is context preserved during adapter switches?**
   - Critical for "hot-swap" designation
   - Test: Multi-turn conversation with mid-stream adapter change
   - Status: ⏳ TO TEST

### Secondary Questions:

7. **What LoRA formats are supported?**
   - GGUF format? (likely)
   - SafeTensors? (unknown)
   - Raw .bin files? (unlikely)

8. **Are there adapter compatibility checks?**
   - Does server validate adapter matches base model?
   - What error messages appear for mismatched adapters?

9. **What's the memory overhead per adapter?**
   - Small (< 100MB)?
   - Depends on adapter rank?

10. **What's the adapter loading time?**
    - Milliseconds? Seconds?
    - Blocking or async?

## Testing Strategy

### Phase 1: CLI Arguments (Process-Level)
```bash
# Start without LoRA
llama-server -m model.gguf --port 8080

# Start with LoRA
llama-server -m model.gguf --port 8080 --lora adapter.gguf

# Start with multiple LoRAs (if supported)
llama-server -m model.gguf --port 8080 --lora adapter1.gguf --lora adapter2.gguf
```

**Expected Results**:
- ✅ Single `--lora` should work (standard feature)
- ❓ Multiple `--lora` flags may or may not work
- ❌ No hot-swap (requires restart)

### Phase 2: Per-Request API
```python
# Request with adapter
requests.post("http://localhost:8080/v1/chat/completions", json={
    "messages": [...],
    "lora": [{"id": 0, "scale": 1.0}]
})

# Request without adapter
requests.post("http://localhost:8080/v1/chat/completions", json={
    "messages": [...],
    "lora": [{"id": 0, "scale": 0.0}]  # Scale 0 = disabled
})
```

**Expected Results**:
- ✅ Likely to work (OpenAPI schema exists)
- ✅ True hot-swap (no restart)
- ✅ Context preserved
- ❓ Adapter must be loaded at startup via CLI

### Phase 3: Runtime API (Exploratory)
```python
# Try to load adapter at runtime
requests.post("http://localhost:8080/lora/load", json={
    "path": "/path/to/adapter.gguf",
    "scale": 1.0
})

# Try to list adapters
requests.get("http://localhost:8080/lora/list")

# Try to unload adapter
requests.post("http://localhost:8080/lora/unload", json={
    "id": 0
})
```

**Expected Results**:
- ❓ May not exist (404 errors)
- ❓ Could be in newer versions than b6399
- ✅ Would be ideal for Jan if it exists

## Upstream Documentation to Check

1. **llama.cpp GitHub**:
   - `examples/server/README.md` - Server documentation
   - `examples/server/server.cpp` - Source code for CLI args
   - `examples/server/api.cpp` - API endpoint handlers
   - Search issues/PRs for "lora" or "adapter"

2. **Jan's llama.cpp Fork**:
   - Check `janhq/llama.cpp` repository
   - May have custom patches or features
   - Build number b6399 corresponds to specific commit

3. **OpenAI API Spec**:
   - llama.cpp aims for OpenAI compatibility
   - Check if `lora` field is standard extension

## Next Steps

1. ✅ Create test harness (`test_hotswap.py`)
2. ⏳ Obtain a small test model + LoRA adapter
3. ⏳ Run Phase 1 tests (CLI arguments)
4. ⏳ Run Phase 2 tests (per-request API)
5. ⏳ Run Phase 3 tests (runtime API)
6. ⏳ Document findings in this file
7. ⏳ Update README with recommendations

## Findings (To Be Populated)

### Test Results

**Phase 1: CLI Arguments**
- Status: ⏳ Not tested yet
- `--lora`: 
- `--lora-scaled`:
- Multiple adapters:

**Phase 2: Per-Request API**
- Status: ⏳ Not tested yet
- `lora` field accepted:
- Context preservation:
- Performance overhead:

**Phase 3: Runtime API**
- Status: ⏳ Not tested yet
- `/lora/load`:
- `/lora/unload`:
- `/lora/list`:

### Performance Benchmarks

| Method | Load Time | Memory Δ | Context Preserved | Recommended |
|--------|-----------|----------|-------------------|-------------|
| Process-level | TBD | TBD | ❌ | ⏳ Testing |
| Per-request | TBD | TBD | TBD | ⏳ Testing |
| Runtime API | TBD | TBD | TBD | ⏳ Testing |

## Recommendations (To Be Finalized)

*This section will be updated after testing completes.*

### For Jan Integration:

**If per-request API works** (most likely):
- ✅ **RECOMMENDED**: Use `lora` field in chat completions
- Pre-load adapters at server startup via CLI
- Switch adapters per-message without restart
- Update `useChat` hook to include `lora` parameter
- Add UI for adapter selection

**If only CLI works**:
- ⚠️ **FALLBACK**: Restart server with new `--lora` args
- Modify `LlamacppExtension.load()` to accept adapters
- Warn users about context loss
- Consider keeping conversation history

**If runtime API works** (bonus):
- ✅ **IDEAL**: Full dynamic loading without restart
- Add Tauri commands for adapter management
- Most flexible user experience
- Check version compatibility

## References

- [Jan Architecture](../../CONTRIBUTING.md)
- [LlamacppExtension](../../extensions/llamacpp-extension/src/index.ts)
- [OpenAPI Schema](../../extensions/llamacpp-extension/resources/inference-openapi.json)
- [llama.cpp upstream](https://github.com/ggerganov/llama.cpp)
- [Jan's llama.cpp fork](https://github.com/janhq/llama.cpp)
