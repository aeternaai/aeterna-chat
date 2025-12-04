# Jan Router Service
Router service for intelligent model selection

## Installation

```bash
pip install -r requirements.txt
```

## Running

```bash
# Default: localhost:8765
python main.py

# Custom host/port
python main.py --host 0.0.0.0 --port 9000

# Debug mode
python main.py --log-level debug
```

## API Endpoints

### Health Check
```
GET /health
```

### List Strategies
```
GET /strategies
```

### Route Request
```
POST /route
{
  "messages": [...],
  "availableModels": [...],
  "activeModels": [...]
}
```

### Set Strategy
```
POST /strategy/{strategy_name}
```

## Available Strategies

- **heuristic**: Rule-based routing using query characteristics and model capabilities
- **llm-based**: Uses a lightweight OpenAI-compatible model (defaults to Jan's local API) to make routing decisions with deeper reasoning
- More strategies coming soon (embedding-based, ML classifier)

### LLM Strategy Configuration

The LLM router calls an OpenAI-compatible `/chat/completions` endpoint. You can control it via environment variables before starting the router service:

| Variable | Default | Description |
| --- | --- | --- |
| `ROUTER_LLM_MODEL` | `Phi-4-mini-instruct_Q4_K_M` | Model ID to send to the router endpoint (should be a lightweight model dedicated for routing) |
| `ROUTER_LLM_BASE_URL` | `http://127.0.0.1:1337/v1` | Base URL for the OpenAI-compatible API (without the `/chat/completions` suffix) |
| `ROUTER_LLM_API_KEY` | _empty_ | Optional API key if the endpoint requires authentication |
| `ROUTER_LLM_TEMPERATURE` | `0.1` | Temperature used for routing prompts |
| `ROUTER_LLM_TIMEOUT` | `15` | Request timeout (seconds) when calling the router model |
| `ROUTER_LLM_MAX_TOKENS` | `120` | Maximum tokens to request from the router model |

> Tip: Point the base URL at Jan's bundled OpenAI server to keep routing on-device, or supply your own remote endpoint when experimenting.

You can also update these values at runtime via `POST /config/llm` with a payload containing any of the fields above (`baseUrl`, `apiKey`, `model`, etc.). The desktop app's Local API Server settings automatically call this endpoint so the router reuses your server's host, port, and API key.

**Important:** The `model` field should reference a **dedicated lightweight model** (like `Phi-4-mini-instruct_Q4_K_M`) that is loaded specifically for making routing decisions. This model is separate from the larger models being routed to (e.g., `Qwen3-VL-8B-Instruct-IQ4_XS`). 

The Jan desktop app **automatically loads** `Phi-4-mini-instruct_Q4_K_M` on startup if it's available in your model library, so you don't need to manually start it. If you haven't downloaded this model yet, the router will fall back to heuristic-based routing until the model becomes available.
