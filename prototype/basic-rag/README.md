# Basic RAG - Langchain + Qdrant + LlamaCpp

A proof-of-concept Retrieval-Augmented Generation (RAG) system built with Langchain, Qdrant, and LlamaCpp. This standalone application provides project-based document organization with intelligent context-aware question answering.

## Features

- 🗂️ **Project-based Collections**: Organize documents by projects with dedicated vector storage
- 🌍 **Global Collection**: Support for documents not tied to specific projects
- 📄 **Multi-format Support**: PDF, DOCX, TXT, Markdown files
- 🔍 **ColBERT Embeddings**: Advanced retrieval using NemoRetriever ColBERT model
- 🤖 **Powerful LLM**: Nemotron 30B for high-quality answer generation
- 💾 **Local-first**: All data and models stored locally using Qdrant file-based storage
- 🎯 **Chat-level Filtering**: Optional filtering by chat ID within projects

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                         CLI Interface                        │
│                          (cli.py)                            │
└───────────────────────────┬─────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                       RAG Pipeline                           │
│                      (rag/pipeline.py)                       │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │  Document       │  │   Qdrant     │  │   Project     │  │
│  │  Processor      │  │   Vector     │  │   Manager     │  │
│  │                 │  │   Store      │  │               │  │
│  └─────────────────┘  └──────────────┘  └───────────────┘  │
└─────────────────────────────────────────────────────────────┘
                            │
            ┌───────────────┴───────────────┐
            ▼                               ▼
┌─────────────────────────┐   ┌─────────────────────────────┐
│   ColBERT Embeddings    │   │  ChatOpenAI Client (HTTP)   │
│   (NemoRetriever 1B)    │   │    (langchain-openai)       │
└─────────────────────────┘   └──────────────┬──────────────┘
            │                               │
            │                               ▼ HTTP /v1
            │                  ┌─────────────────────────────┐
            │                  │   llama.cpp Server          │
            │                  │   (External Process)        │
            │                  ├─────────────────────────────┤
            │                  │  • Health check endpoint    │
            │                  │  • Streaming support        │
            │                  │  • Retry with backoff       │
            │                  └──────────────┬──────────────┘
            │                                 │
            │                                 ▼
            │                  ┌─────────────────────────────┐
            │                  │    Nemotron 30B (GGUF)      │
            │                  │    (Loaded in server)       │
            │                  └─────────────────────────────┘
            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Qdrant Local Storage                      │
│            (./qdrant_storage/ - SQLite-based)                │
└─────────────────────────────────────────────────────────────┘
```

## Prerequisites

- Python 3.10 or higher
- 16+ GB RAM recommended (10 GB minimum)
- llama.cpp server built and running with a compatible GGUF model
- macOS, Linux, or Windows

## llama.cpp Server Setup

This application requires a running llama.cpp server with an OpenAI-compatible API.

### 1. Build llama.cpp

```bash
# Clone llama.cpp repository
git clone https://github.com/ggerganov/llama.cpp.git
cd llama.cpp

# Build the server
make llama-server

# Optional: Enable GPU acceleration
# Metal (macOS): make llama-server LLAMA_METAL=1
# CUDA (NVIDIA): make llama-server LLAMA_CUDA=1
```

### 2. Download a Model

Download a compatible GGUF model. Recommended:

```bash
# Nemotron 30B (Q4_K_M quantization - ~16 GB)
wget https://huggingface.co/unsloth/Nemotron-3-Nano-30B-A3B-GGUF/resolve/main/Nemotron-3-Nano-30B-A3B-Q4_K_M.gguf
```

### 3. Start the Server

```bash
./llama-server -m Nemotron-3-Nano-30B-A3B-Q4_K_M.gguf -c 8192 --port 8080
```

**Server Options:**
- `-m`: Path to GGUF model file
- `-c`: Context length (default: 8192)
- `--port`: Port number (default: 8080)
- `-ngl`: Number of layers to offload to GPU (e.g., `-ngl 99` for full GPU)

The server provides an OpenAI-compatible API at `http://localhost:8080/v1`.

**Keep this server running** while using the RAG application.

## Installation

1. **Clone or navigate to the project directory:**

```bash
cd prototype/basic-rag
```

2. **Create and activate a virtual environment:**

```bash
python -m venv venv
source venv/bin/activate  # On macOS/Linux
# or
venv\Scripts\activate     # On Windows
```

3. **Install dependencies:**

```bash
pip install -r requirements.txt
```

4. **Configure environment:**

Copy `.env.example` to `.env` and customize if needed:

```bash
cp .env.example .env
```

Default configuration connects to `http://localhost:8080/v1`. Adjust `LLAMA_SERVER_URL` if your server uses a different address.

## Setup

Run the setup command to validate server connection and initialize storage:

```bash
python cli.py setup
```

This will:
- Verify llama.cpp server is reachable
- Initialize Qdrant local storage (`./qdrant_storage/`)
- Create necessary directories

**Important:** Ensure the llama.cpp server is running before executing setup.

## Usage

### 1. Ingest Documents

**Ingest to global collection:**

```bash
python cli.py ingest document.pdf
```

**Ingest to a project collection:**

```bash
python cli.py ingest document.pdf --project-id=proj1 --project-name="Project Alpha"
```

**Ingest with chat context:**

```bash
python cli.py ingest document.pdf \
  --project-id=proj1 \
  --project-name="Project Alpha" \
  --chat-id=chat123
```

### 2. Query the System

**Query global collection:**

```bash
python cli.py query "What is machine learning?"
```

**Query specific project:**

```bash
python cli.py query "What is the project budget?" --project-id=proj1
```

**Query specific chat in project:**

```bash
python cli.py query "What did we discuss about timelines?" \
  --project-id=proj1 \
  --chat-id=chat123
```

**Customize retrieval:**

```bash
python cli.py query "What are the key findings?" \
  --project-id=proj1 \
  --top-k=5
```

### 3. Manage Collections

**List all collections:**

```bash
python cli.py collections list
```

**Get collection details:**

```bash
python cli.py collections info project_proj1
```

**Delete a collection:**

```bash
python cli.py collections delete project_proj1
```

**Force delete without confirmation:**

```bash
python cli.py collections delete project_proj1 --yes
```

### 4. System Information

View system configuration and status:

```bash
python cli.py info
```

## Project Structure

```
prototype/basic-rag/
├── cli.py                  # Command-line interface
├── config.py               # Configuration and settings
├── requirements.txt        # Python dependencies
├── .env.example           # Environment variable template
├── .gitignore             # Git ignore rules
├── README.md              # This file
│
├── rag/                   # Core RAG modules
│   ├── __init__.py
│   ├── pipeline.py        # RAG pipeline orchestration
│   ├── document_processor.py  # Document loading and chunking
│   ├── vector_store.py    # Qdrant vector storage wrapper
│   └── project_manager.py # Collection management
│
└── qdrant_storage/        # Qdrant local database (created by setup)
    └── collections/
        ├── global/
        └── project_*/

Note: Models are managed by the external llama.cpp server.
```

## Configuration

Configuration is managed through environment variables (`.env` file) or uses sensible defaults:

| Variable | Default | Description |
|----------|---------|-------------|
| `QDRANT_STORAGE_PATH` | `./qdrant_storage` | Qdrant database path |
| `MODELS_DIR` | `./models` | Model storage directory |
| `CHUNK_SIZE` | `512` | Text chunk size (characters) |
| `CHUNK_OVERLAP` | `64` | Overlap between chunks |
| `LLM_CONTEXT_LENGTH` | `8192` | LLM context window |
| `LLM_MAX_TOKENS` | `1024` | Max tokens in LLM response |
| `LLM_TEMPERATURE` | `0.7` | LLM sampling temperature |
| `RETRIEVAL_TOP_K` | `3` | Number of documents to retrieve |
| `LLAMA_SERVER_URL` | `http://localhost:8080/v1` | llama.cpp server URL |
| `LLAMA_API_KEY` | `not-needed` | API key (not required for local) |
| `LLAMA_SERVER_TIMEOUT` | `600` | Server timeout (seconds) |
| `LLAMA_STREAMING_ENABLED` | `True` | Enable streaming responses |
| `LLAMA_MAX_RETRIES` | `3` | Max retry attempts on failure |

## Collection Organization

### Project Collections

Each project gets its own collection named `project_{project_id}`:

- Isolated vector storage per project
- All chats in a project share the same collection
- Project-wide RAG context (default behavior)
- Optional chat-level filtering via `--chat-id`

### Global Collection

Documents without a project go to the `global` collection:

- Shared knowledge base across all non-project chats
- Useful for general reference documents
- No project association required

### Metadata Structure

Each document chunk includes:

```json
{
  "project_id": "proj1",
  "project_name": "Project Alpha",
  "chat_id": "chat123",
  "source_file": "document.pdf",
  "source_path": "/full/path/to/document.pdf",
  "chunk_index": 0,
  "timestamp": "2025-12-29T10:30:00.000000",
  "chunk_size": 485,
  "similarity_score": 0.856
}
```

## Models

### LLM (via llama.cpp server)

The application connects to an external llama.cpp server. Recommended model:

- **Model:** `unsloth/Nemotron-3-Nano-30B-A3B-GGUF`
- **Size:** ~15-20 GB (Q4_K_M quantization)
- **Purpose:** Answer generation
- **Context:** 8192 tokens
- **Server:** Must be started independently (see "llama.cpp Server Setup")

### NemoRetriever ColBERT (Embeddings)

- **Model:** `second-state/Llama-NemoRetriever-ColEmbed-v1-GGUF`
- **Size:** ~2 GB (Q8_0 quantization)
- **Purpose:** Document embeddings and retrieval
- **Dimension:** 128 (ColBERT late interaction)
- **Loading:** Managed directly by the application (not via server)

## Supported File Types

- **PDF:** `.pdf`
- **Word:** `.docx`, `.doc`
- **Text:** `.txt`, `.md`, `.markdown`

## API Conversion

This CLI application is designed for easy conversion to a REST API. The service classes in `rag/` are completely decoupled from the CLI:

### Conversion Steps:

1. Replace `cli.py` with a FastAPI application
2. Create endpoint handlers that call `RAGPipeline` methods:
   - `POST /ingest` → `pipeline.ingest_document()`
   - `POST /query` → `pipeline.query()`
   - `GET /collections` → `project_manager.list_collections()`
3. Add authentication and request validation
4. Keep all business logic in `rag/` modules unchanged

### Example FastAPI Structure:

```python
from fastapi import FastAPI
from rag import RAGPipeline

app = FastAPI()
pipeline = RAGPipeline()

@app.post("/ingest")
async def ingest_document(file: UploadFile, project_id: str = None):
    return pipeline.ingest_document(file.filename, project_id)

@app.post("/query")
async def query_rag(question: str, project_id: str = None):
    return pipeline.query(question, project_id)
```

## Performance Considerations

### Memory Usage

- **Application:** ~2-3 GB RAM (embeddings model)
- **llama.cpp Server:** ~8-12 GB RAM (with Q4 quantization)
- **Qdrant:** <1 GB for typical document collections

### Speed

- **Document Ingestion:** ~1-5 seconds per page (depending on chunking)
- **Embeddings:** ~0.5-2 seconds per query
- **LLM Generation:** ~5-30 seconds per response (depending on length and server load)
- **Streaming:** Responses stream token-by-token (configurable via `LLAMA_STREAMING_ENABLED`)

### Optimization Tips

1. **Use smaller models** in llama.cpp server for faster inference
2. **Adjust chunk size** to balance context vs. retrieval precision
3. **Tune `top-k`** to retrieve more/fewer documents
4. **Enable GPU** in llama.cpp server build (Metal/CUDA) for 10x+ speedup
5. **Adjust retry settings** via `LLAMA_MAX_RETRIES` for unreliable networks
6. **Disable streaming** (`LLAMA_STREAMING_ENABLED=False`) if buffering entire response is acceptable

## Troubleshooting

### Server Connection Failed

**Error:** `Connection refused` or `Server not reachable`

**Solution:**
1. Verify the llama.cpp server is running:
   ```bash
   curl http://localhost:8080/health
   ```
2. Check server logs for errors
3. Ensure port 8080 is not blocked by firewall
4. Verify `LLAMA_SERVER_URL` in `.env` matches your server configuration

### Server Timeout

**Error:** `Request timed out after 600 seconds`

**Solution:**
1. Increase `LLAMA_SERVER_TIMEOUT` in `.env` (e.g., `LLAMA_SERVER_TIMEOUT=900`)
2. Use a smaller model with faster inference
3. Reduce `LLM_MAX_TOKENS` to generate shorter responses
4. Check server CPU/GPU utilization (may be overloaded)

### Retry Exhausted

**Error:** `Max retries (3) exceeded`

**Solution:**
1. Check server stability (restart if necessary)
2. Increase `LLAMA_MAX_RETRIES` in `.env`
3. Verify network connectivity between application and server
4. Check server logs for recurring errors

### Port Already in Use

**Error:** llama.cpp server fails to start on port 8080

**Solution:**
1. Use a different port: `./llama-server -m model.gguf --port 8081`
2. Update `LLAMA_SERVER_URL=http://localhost:8081/v1` in `.env`
3. Or stop the process using port 8080: `lsof -ti:8080 | xargs kill -9`

### Out of Memory

**Application:**
- Reduce `CHUNK_SIZE` in `.env`
- Lower `RETRIEVAL_TOP_K` to retrieve fewer documents

**Server:**
- Use a smaller model (e.g., Q4 instead of Q8)
- Reduce context length: `./llama-server -m model.gguf -c 4096`
- Close other applications

### Slow Performance

**Server:**
- Enable GPU support when building llama.cpp:
  ```bash
  # Metal (macOS)
  make llama-server LLAMA_METAL=1
  
  # CUDA (NVIDIA GPU)
  make llama-server LLAMA_CUDA=1
  ```

**Application:**
- Disable streaming for batch processing: `LLAMA_STREAMING_ENABLED=False`
- Reduce `CHUNK_SIZE` to speed up embeddings
- Lower `RETRIEVAL_TOP_K` to retrieve fewer documents

### Collection Not Found

```bash
python cli.py collections list  # Verify collection exists
```

### Check System Status

```bash
python cli.py info  # Shows server connectivity and configuration
```

## Development

### Running Tests

(Tests to be implemented)

```bash
pytest tests/
```

### Code Structure

- **Service Layer:** `rag/` modules contain all business logic
- **CLI Layer:** `cli.py` provides user interface only
- **Configuration:** `config.py` centralizes all settings
- **No Dependencies:** CLI code has zero imports from `click` in service layer

### Adding New Features

1. **New document loader:** Add to `document_processor.py`
2. **New retrieval strategy:** Extend `vector_store.py`
3. **New LLM provider:** Modify `pipeline.py`
4. **New CLI commands:** Add to `cli.py`

## License

See the LICENSE file in the root of the repository.

## Acknowledgments

- **Langchain:** Document processing and LLM orchestration
- **Qdrant:** High-performance vector database
- **LlamaCpp:** Efficient local LLM inference
- **Nemotron:** NVIDIA's powerful language model
- **NemoRetriever:** Advanced ColBERT retrieval model

## Future Enhancements

- [ ] REST API implementation
- [ ] Batch document ingestion
- [ ] Document update/versioning
- [ ] Hybrid search (keyword + semantic)
- [ ] Re-ranking models
- [x] Streaming responses (implemented via `LLAMA_STREAMING_ENABLED`)
- [ ] Multi-modal support (images, tables)
- [ ] Conversation history integration
- [ ] Fine-tuned embeddings for domain-specific use cases
- [ ] Async API support for concurrent requests
