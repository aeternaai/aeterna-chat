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
- More strategies coming soon (embedding-based, LLM-based, ML classifier)
