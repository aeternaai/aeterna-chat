# Tauri Plugin: LangChain

This plugin provides LangChain-powered RAG (Retrieval-Augmented Generation) capabilities for Jan AI.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Jan Application                          │
├─────────────────────────────────────────────────────────────────┤
│  Frontend (React)                                               │
│     │                                                           │
│     ▼                                                           │
│  Tauri Commands (invoke)                                        │
│     │                                                           │
│     ▼                                                           │
│  tauri-plugin-langchain (Rust)                                  │
│     │                                                           │
│     │  JSON-RPC 2.0 over stdio                                  │
│     ▼                                                           │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ langchain_service.py (Python subprocess)                │   │
│  │   ├── Document Ingestion                                │   │
│  │   │     └── LangChain TextSplitters                     │   │
│  │   ├── Embeddings                                        │   │
│  │   │     └── HuggingFace sentence-transformers           │   │
│  │   ├── Vector Store                                      │   │
│  │   │     └── Qdrant (local)                              │   │
│  │   └── Query Pipeline                                    │   │
│  │         └── Similarity search + context assembly        │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Features

- **Document Ingestion**: Process PDF, DOCX, TXT, and Markdown files
- **Intelligent Chunking**: RecursiveCharacterTextSplitter with configurable size/overlap
- **Embeddings**: HuggingFace sentence-transformers (all-MiniLM-L6-v2 by default)
- **Vector Storage**: Qdrant local database
- **Semantic Search**: Similarity search with score thresholds
- **Health Monitoring**: Built-in health checks and service info

## Setup

### Python Environment

The plugin requires Python 3.10+ with LangChain dependencies:

```bash
cd src-tauri/plugins/tauri-plugin-langchain/python
./setup.sh
```

Or manually:

```bash
python3 -m venv venv
source venv/bin/activate
pip install -r requirements.txt
```

### Rust Plugin

The plugin is automatically built with the Tauri application:

```bash
make dev
```

## API

### Tauri Commands

All commands are exposed as Tauri invoke handlers:

#### `spawn_langchain_service`
Start the Python LangChain service.

```typescript
await invoke('plugin:langchain|spawn_langchain_service', {
  binaryPath: null,  // Optional: custom Python path
  envVars: null,     // Optional: environment variables
  config: {          // Optional: service configuration
    health_check_timeout_secs: 5,
    request_timeout_secs: 30,
    max_restart_attempts: 3,
    auto_restart: true
  }
});
```

#### `shutdown_langchain_service`
Stop the Python service gracefully.

```typescript
await invoke('plugin:langchain|shutdown_langchain_service');
```

#### `health_check`
Check service health and get status.

```typescript
const health = await invoke('plugin:langchain|health_check');
// { status: "healthy", version: "1.0.0", uptime_seconds: 123 }
```

#### `ingest_document`
Process and store a document.

```typescript
const result = await invoke('plugin:langchain|ingest_document', {
  params: {
    file_path: "/path/to/document.pdf",
    collection: "my_collection",
    chunk_size: 512,      // Optional
    chunk_overlap: 64,    // Optional
    metadata: { key: "value" }  // Optional
  }
});
// { collection: "my_collection", chunks_count: 42, document_ids: [...] }
```

#### `query_rag`
Query the RAG system for answers.

```typescript
const result = await invoke('plugin:langchain|query_rag', {
  params: {
    query: "What is the main topic?",
    collection: "my_collection",
    k: 3,                     // Optional: top-k results
    score_threshold: 0.5,     // Optional: minimum similarity
    filter: { key: "value" }  // Optional: metadata filter
  }
});
// { answer: "...", sources: [...], num_sources: 3 }
```

#### `get_service_info`
Get detailed service and pipeline information.

```typescript
const info = await invoke('plugin:langchain|get_service_info');
// { is_running: true, service: {...}, pipeline: {...} }
```

## JSON-RPC Protocol

The Python service communicates via JSON-RPC 2.0 over stdio.

### Request Format

```json
{
  "jsonrpc": "2.0",
  "id": "unique-id",
  "method": "health",
  "params": {}
}
```

### Response Format

```json
{
  "jsonrpc": "2.0",
  "id": "unique-id",
  "result": {
    "status": "healthy",
    "version": "1.0.0"
  }
}
```

### Error Response

```json
{
  "jsonrpc": "2.0",
  "id": "unique-id",
  "error": {
    "code": -32602,
    "message": "Invalid params"
  }
}
```

### Methods

| Method | Description |
|--------|-------------|
| `health` | Health check |
| `shutdown` | Graceful shutdown |
| `initialize` | Initialize with config |
| `ingest` | Ingest document |
| `query` | Query RAG |
| `get_info` | Get service info |

## Configuration

### Service Configuration (Rust)

```rust
ServiceConfig {
    health_check_timeout_secs: 5,
    request_timeout_secs: 30,
    max_restart_attempts: 3,
    auto_restart: true,
}
```

### Pipeline Configuration (Python)

The Python service accepts configuration via the `initialize` method:

```json
{
  "embeddings_model": "sentence-transformers/all-MiniLM-L6-v2",
  "chunk_size": 512,
  "chunk_overlap": 64,
  "top_k": 3,
  "score_threshold": 0.0,
  "qdrant_path": "/path/to/storage"
}
```

## Development

### Testing the Python Service

```bash
cd src-tauri/plugins/tauri-plugin-langchain/python
source venv/bin/activate
python3 langchain_service.py
```

Then send JSON-RPC requests via stdin:

```json
{"jsonrpc": "2.0", "id": "1", "method": "health", "params": {}}
```

### Running Rust Tests

```bash
cd src-tauri/plugins/tauri-plugin-langchain
cargo test
```

## Phase 2 Roadmap

- [ ] LLM-powered answer generation integration
- [ ] Streaming responses
- [ ] Advanced retrieval strategies (HyDE, Multi-Query)
- [ ] Conversation memory
- [ ] Document preprocessing pipelines
- [ ] Custom embedding models

## License

MIT License - See LICENSE file for details.
