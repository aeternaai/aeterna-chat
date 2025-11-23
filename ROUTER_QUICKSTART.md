# Router Python Migration - Quick Start Guide

## Installation

### 1. Install Python Dependencies

```bash
cd router-service
pip install -r requirements.txt
```

### 2. Verify Python Version

```bash
python3 --version  # Should be 3.11 or higher
```

### 3. Test Router Service Standalone (Optional)

```bash
# Start the service
cd router-service
python main.py --log-level debug

# In another terminal, test endpoints
curl http://localhost:8765/health
curl http://localhost:8765/strategies

# Test routing (example)
curl -X POST http://localhost:8765/route \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [{"role": "user", "content": "Write a Python function"}],
    "availableModels": [
      {
        "id": "test-model",
        "providerId": "llamacpp",
        "capabilities": ["chat", "code"],
        "metadata": {
          "parameterCount": "7B",
          "contextWindow": 4096,
          "isLoaded": true
        }
      }
    ],
    "activeModels": ["test-model"]
  }'
```

## Running Jan with Python Router

### 1. Build and Run

```bash
# From project root
make dev
```

The router service will automatically start on app launch.

### 2. Check Router Status

Open the browser console (View → Toggle Developer Tools) and run:

```javascript
// Check health
await window.__TAURI__.core.invoke('get_router_health')

// List strategies
await window.__TAURI__.core.invoke('list_router_strategies')

// Test routing manually (if needed)
await window.__TAURI__.core.invoke('route_request', {
  request: {
    messages: [{role: 'user', content: 'Hello'}],
    availableModels: [], // Will be populated by Jan
    activeModels: []
  }
})
```

### 3. Enable Routing in Jan UI

1. Go to Settings → Advanced → Router
2. Enable "Use Model Router"
3. Select models in "Allowed Models" setting
4. Start chatting - routing decisions will appear in console

## Verification Checklist

- [ ] Python dependencies installed successfully
- [ ] `make dev` starts without errors
- [ ] Router service health check passes (see logs)
- [ ] Chat with routing enabled works
- [ ] Console shows "[RouterExtension] Python router: ..." logs
- [ ] Fallback works (stop router service manually, should use TypeScript)

## Logs

Router service logs appear in:
- Console output during `make dev`
- Tauri logs: Check for "[RouterExtension]" and "Starting Python router service"

## Troubleshooting

### Router service fails to start

**Symptom:** Error in logs: "Failed to start router service"

**Solutions:**
1. Check Python version: `python3 --version`
2. Reinstall dependencies: `cd router-service && pip install -r requirements.txt`
3. Check if port 8765 is in use: `lsof -i :8765` (macOS/Linux)

### Router falls back to TypeScript

**Symptom:** Console shows "Python router service not available, using fallback"

**This is expected if:**
- Running in web mode (`make dev-web-app`)
- Python service crashed (check logs)
- Dependencies not installed

**Solutions:**
1. Check logs for Python errors
2. Verify dependencies: `pip list | grep fastapi`
3. Test standalone: `cd router-service && python main.py`

### Import errors in Python

**Symptom:** "Import 'fastapi' could not be resolved"

**Solution:**
```bash
cd router-service
pip install -r requirements.txt --upgrade
```

## Security Scan (Optional)

If you have Snyk installed:

```bash
# Authenticate first
snyk auth

# Scan Python code
snyk code test router-service
```

## Next Steps

Once verified:
1. Test with different query types (code, images, reasoning)
2. Compare routing decisions with TypeScript fallback
3. Monitor latency in console logs
4. Consider contributing additional strategies (embedding-based, LLM-based)

## Advanced: Adding New Strategies

Create a new file `router-service/strategies/my_strategy.py`:

```python
from models import RouteRequest, RouteResponse
from strategies.base import RouterStrategy

class MyStrategy(RouterStrategy):
    @property
    def name(self) -> str:
        return "my-strategy"
    
    @property
    def description(self) -> str:
        return "My custom routing strategy"
    
    async def route(self, request: RouteRequest) -> RouteResponse:
        # Your routing logic here
        return RouteResponse(
            modelId="selected-model-id",
            providerId="llamacpp",
            confidence=0.9,
            reasoning="Custom logic selected this model"
        )
```

Register in `router-service/router.py`:

```python
from strategies.my_strategy import MyStrategy

def _register_strategies(self):
    # ... existing strategies ...
    my_strategy = MyStrategy()
    self.strategies[my_strategy.name] = my_strategy
```

Restart the app and your strategy will be available!
