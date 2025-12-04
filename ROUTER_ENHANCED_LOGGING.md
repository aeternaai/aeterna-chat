# Router Enhanced Logging & Diagnostics

## Summary
Enhanced the LLM router with comprehensive logging to diagnose routing failures and model selection issues.

## Changes Made

### Enhanced Logging in `llm.py`

Added detailed logging at every stage of the routing process:

#### 1. Route Entry Point
- **Before**: Basic debug message
- **After**: Info-level logs showing:
  - Number of candidate models
  - List of available model IDs
  - Query character count

#### 2. Success Path
- **Before**: Simple "Selected X" message
- **After**: 
  - ✓ Success indicator with checkmark
  - Model ID selected
  - First 100 chars of reasoning
  - Metadata about LLM vs fallback

#### 3. Failure Path (Fallback Triggered)
- **Before**: Generic exception log
- **After**:
  - ✗ Failure indicator with X mark
  - Error type classification (HTTPStatusError, TimeoutException, etc.)
  - Truncated error message (first 200 chars)
  - Fallback strategy name
  - Full traceback for debugging
  - Separate success log when fallback completes

#### 4. API Call Details
- **Before**: Debug-level request info
- **After**: Info-level logs showing:
  - → Request: URL, model name, timeout, API key status
  - ← Response: HTTP status, reason phrase, elapsed time
  - Request payload at debug level
  - Specific error handling for:
    - `HTTPStatusError` - Shows status code and response body
    - `TimeoutException` - Shows timeout duration
    - `ConnectError` - Shows connection details
    - Generic exceptions - Shows error type and message

#### 5. Response Parsing
- **Before**: Debug-level raw response
- **After**: Info-level logs showing:
  - Raw response text from LLM
  - Parsed format detection (`<number>|<reason>` vs extracted number)
  - Final model index and ID selected
  - Provider information
  - Metadata about parsed index

### New Health Check Method

Added `async def health_check()` to test router model accessibility:
- Sends test prompt to router model
- Returns status dict with:
  - `status`: "healthy" or "unhealthy"
  - `model`: Router model ID
  - `base_url`: API endpoint
  - `response` or `error`: Result details

## Diagnostic Output Examples

### Successful Routing
```
[LLMRouter] Starting route() with 3 candidate models (query chars=45)
[LLMRouter] Available models: ['Qwen3-VL-8B-Instruct-IQ4_XS', 'phi-3-mini', 'llama-3.2']
[LLMRouter] → API Request: POST http://127.0.0.1:1337/v1/chat/completions (model='Phi-4-mini-instruct_Q4_K_M', timeout=15s, api_key=✓ set)
[LLMRouter] ← API Response: 200 OK (took ~234ms)
[LLMRouter] Parsing router response: '2|This query requires vision capabilities for image analysis'
[LLMRouter] Parsed format '<number>|<reason>': index=1, reasoning='This query requires vision capabilities...'
[LLMRouter] Final selection: models[1] = 'Qwen3-VL-8B-Instruct-IQ4_XS' (provider: llamacpp)
[LLMRouter] ✓ Successfully selected 'Qwen3-VL-8B-Instruct-IQ4_XS' via LLM router (reasoning: This query requires vision...)
```

### Fallback Scenario
```
[LLMRouter] Starting route() with 3 candidate models (query chars=45)
[LLMRouter] Available models: ['Qwen3-VL-8B-Instruct-IQ4_XS', 'phi-3-mini']
[LLMRouter] → API Request: POST http://127.0.0.1:1337/v1/chat/completions (model='Phi-4-mini-instruct_Q4_K_M', timeout=15s, api_key=✓ set)
[LLMRouter] ✗ HTTP error from router model API: 404 Not Found - Response body: {"error":"Model session not found"}
[LLMRouter] ✗ LLM routing failed with HTTPStatusError: 404 Not Found - Falling back to heuristic
[LLMRouter] Full error traceback:
Traceback (most recent call last):
  ...
[LLMRouter] Invoking fallback strategy: heuristic
[LLMRouter] ✓ Fallback selected 'Qwen3-VL-8B-Instruct-IQ4_XS' (reasoning: Query contains vision keywords... (LLM router fallback: HTTPStatusError))
```

## Common Issues to Diagnose

### 1. "Client error" / HTTP 4xx Errors
**Symptoms**: `✗ HTTP error from router model API: 401/403/404`

**Likely Causes**:
- 401: API key missing or invalid
- 403: API key unauthorized
- 404: Router model not loaded or session expired

**Check logs for**:
- `api_key=✗ NOT SET` vs `api_key=✓ set`
- Response body showing error details

### 2. Model Selection Mismatch
**Symptoms**: "reasoning" mentions one model but different one selected

**Likely Causes**:
- LLM response didn't follow `<number>|<reason>` format
- Index out of bounds, defaulting to first model
- Parsing failure, fallback to index 0

**Check logs for**:
- `Could not parse response` warnings
- `Response index X out of bounds` warnings
- `parsed_index` in metadata

### 3. Intermittent Failures
**Symptoms**: First request works, subsequent fail

**Likely Causes**:
- Router model session timing out or being unloaded
- Connection pool exhaustion
- Model inference taking too long

**Check logs for**:
- `TimeoutException` after 15s
- `ConnectError` indicating connection issues
- Response times increasing over successive calls

## Debugging Commands

### Check Router Model Status
```bash
# Check if model is loaded in Jan
# Look for "Phi-4-mini-instruct_Q4_K_M" in active models
```

### Test Router Service Health
```bash
curl -X POST http://127.0.0.1:8765/route \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [{"role": "user", "content": "test"}],
    "available_models": [{"id": "test-model", "provider_id": "llamacpp"}]
  }'
```

### Monitor Router Logs
```bash
# In the terminal running the router service
# Look for [LLMRouter] prefixed messages
# Debug level: --log-level debug
python main.py --log-level debug
```

## Next Steps

1. **Run the app** and check logs for:
   - Router model auto-loading: `Router model 'Phi-4-mini-instruct_Q4_K_M' loaded successfully`
   - First routing attempt: Should see detailed request/response logs
   - Second routing attempt: Check if same model session is reused or if 404 occurs

2. **If 404 persists**:
   - Check if model is being unloaded between requests
   - Verify model session timeout settings
   - Consider keeping router model "pinned" (always loaded)

3. **If different model selected than reasoning suggests**:
   - Check the `Parsing router response` log
   - Verify LLM is returning proper format: `<number>|<reason>`
   - May need to adjust LLM system prompt for better formatting
