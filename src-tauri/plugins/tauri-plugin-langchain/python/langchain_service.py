"""
LangChain Service for Jan AI

A JSON-RPC 2.0 service that provides LangChain-powered RAG capabilities.
Communicates via stdio (stdin/stdout) with the Tauri backend.

This service handles:
- Document ingestion with intelligent chunking
- Vector embeddings via HuggingFace models
- Semantic search and retrieval
- LLM-powered answer generation

Protocol: JSON-RPC 2.0 over stdio
- One request per line (newline-delimited JSON)
- One response per line
"""

import sys
import json
import logging
import time
import traceback
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, Optional, List

# Configure logging to stderr (stdout is for JSON-RPC)
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s',
    stream=sys.stderr
)
logger = logging.getLogger("langchain_service")

# LangChain imports
try:
    from langchain_core.documents import Document
    from langchain_text_splitters import RecursiveCharacterTextSplitter
    from langchain_community.document_loaders import (
        PyPDFLoader,
        TextLoader,
        Docx2txtLoader,
    )
    from langchain_huggingface import HuggingFaceEmbeddings
    from langchain_community.vectorstores import Qdrant
    from qdrant_client import QdrantClient
    from qdrant_client.models import Distance, VectorParams
    LANGCHAIN_AVAILABLE = True
except ImportError as e:
    logger.warning(f"LangChain imports failed: {e}")
    LANGCHAIN_AVAILABLE = False


# JSON-RPC Error Codes
class JsonRpcError:
    PARSE_ERROR = -32700
    INVALID_REQUEST = -32600
    METHOD_NOT_FOUND = -32601
    INVALID_PARAMS = -32602
    INTERNAL_ERROR = -32603
    # Custom error codes
    SERVICE_NOT_INITIALIZED = -32000
    INGESTION_FAILED = -32001
    QUERY_FAILED = -32002
    FILE_NOT_FOUND = -32003


class LangChainService:
    """
    JSON-RPC service for LangChain RAG operations.
    
    Handles document ingestion, embedding generation, and retrieval-augmented
    generation queries.
    """
    
    def __init__(self):
        self.start_time = datetime.now()
        self.version = "1.0.0"
        self.initialized = False
        
        # Configuration (can be overridden via environment)
        self.config = {
            "embeddings_model": "sentence-transformers/all-MiniLM-L6-v2",
            "chunk_size": 512,
            "chunk_overlap": 64,
            "top_k": 3,
            "score_threshold": 0.0,
            "qdrant_path": None,  # Set during initialization
            "llm_server_url": "http://127.0.0.1:8080/v1",  # llama.cpp server
            "llm_api_key": "not-needed",
            "llm_max_tokens": 1024,
            "llm_temperature": 0.7,
            "llm_timeout": 600,
        }
        
        # Components (initialized lazily)
        self.embeddings = None
        self.qdrant_client = None
        self.text_splitter = None
        self.llm = None
        
        logger.info(f"LangChain Service v{self.version} created")
    
    def initialize(self, config: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """
        Initialize the service with the given configuration.
        
        Args:
            config: Optional configuration overrides
            
        Returns:
            Status dict with initialization result
        """
        try:
            if config:
                self.config.update(config)
            
            # Set default Qdrant path if not provided
            if not self.config.get("qdrant_path"):
                self.config["qdrant_path"] = str(Path.home() / ".jan" / "qdrant_storage")
            
            # Ensure Qdrant storage directory exists
            qdrant_path = Path(self.config["qdrant_path"])
            qdrant_path.mkdir(parents=True, exist_ok=True)
            
            logger.info(f"Initializing with config: {self.config}")
            
            # Initialize embeddings model
            logger.info(f"Loading embeddings model: {self.config['embeddings_model']}")
            self.embeddings = HuggingFaceEmbeddings(
                model_name=self.config["embeddings_model"],
                model_kwargs={"device": "cpu"},  # Use CPU for compatibility
                encode_kwargs={"normalize_embeddings": True}
            )
            
            # Initialize Qdrant client
            logger.info(f"Connecting to Qdrant at: {self.config['qdrant_path']}")
            self.qdrant_client = QdrantClient(path=self.config["qdrant_path"])
            
            # Initialize text splitter
            self.text_splitter = RecursiveCharacterTextSplitter(
                chunk_size=self.config["chunk_size"],
                chunk_overlap=self.config["chunk_overlap"],
                length_function=len,
                separators=["\n\n", "\n", " ", ""],
            )
            
            # Initialize LLM connection (optional - only if server URL provided)
            if self.config.get("llm_server_url"):
                logger.info(f"Initializing LLM connection: {self.config['llm_server_url']}")
                try:
                    from langchain_openai import ChatOpenAI
                    self.llm = ChatOpenAI(
                        base_url=self.config["llm_server_url"],
                        api_key=self.config["llm_api_key"],
                        model="gpt-3.5-turbo",  # Ignored by llama.cpp
                        max_tokens=self.config["llm_max_tokens"],
                        temperature=self.config["llm_temperature"],
                        timeout=self.config["llm_timeout"],
                    )
                    logger.info("LLM connection initialized")
                except Exception as e:
                    logger.warning(f"Failed to initialize LLM: {e}. Query will return context only.")
            
            self.initialized = True
            logger.info("Service initialized successfully")
            
            return {
                "status": "initialized",
                "config": self.config,
            }
            
        except Exception as e:
            logger.error(f"Initialization failed: {e}")
            raise RuntimeError(f"Failed to initialize service: {e}")
    
    def _ensure_initialized(self):
        """Ensure the service is initialized before operations."""
        if not self.initialized:
            # Auto-initialize with defaults
            self.initialize()
    
    def _load_document(self, file_path: str) -> List[Document]:
        """
        Load a document using the appropriate loader.
        
        Args:
            file_path: Path to the document file
            
        Returns:
            List of Document objects
        """
        path = Path(file_path)
        
        if not path.exists():
            raise FileNotFoundError(f"File not found: {file_path}")
        
        extension = path.suffix.lower()
        
        if extension == ".pdf":
            loader = PyPDFLoader(str(path))
        elif extension in [".docx", ".doc"]:
            loader = Docx2txtLoader(str(path))
        elif extension in [".txt", ".md", ".markdown"]:
            loader = TextLoader(str(path), encoding="utf-8")
        else:
            raise ValueError(f"Unsupported file type: {extension}")
        
        return loader.load()
    
    def _ensure_collection(self, collection_name: str, vector_size: int = 384):
        """
        Ensure a collection exists in Qdrant.
        
        Args:
            collection_name: Name of the collection
            vector_size: Size of the embedding vectors
        """
        collections = [c.name for c in self.qdrant_client.get_collections().collections]
        
        if collection_name not in collections:
            logger.info(f"Creating collection: {collection_name}")
            self.qdrant_client.create_collection(
                collection_name=collection_name,
                vectors_config=VectorParams(
                    size=vector_size,
                    distance=Distance.COSINE,
                ),
            )
    
    def health(self) -> Dict[str, Any]:
        """
        Health check endpoint.
        
        Returns:
            Health status information
        """
        uptime = (datetime.now() - self.start_time).total_seconds()
        
        return {
            "status": "healthy",
            "version": self.version,
            "uptime_seconds": uptime,
            "initialized": self.initialized,
            "langchain_available": LANGCHAIN_AVAILABLE,
        }
    
    def shutdown(self) -> Dict[str, Any]:
        """
        Graceful shutdown.
        
        Returns:
            Shutdown confirmation
        """
        logger.info("Shutdown requested")
        
        # Clean up resources
        if self.qdrant_client:
            try:
                self.qdrant_client.close()
            except Exception as e:
                logger.warning(f"Error closing Qdrant client: {e}")
        
        return {"status": "shutting_down"}
    
    def ingest(self, params: Dict[str, Any]) -> Dict[str, Any]:
        """
        Ingest a document into the RAG system.
        
        Args:
            params: Ingestion parameters
                - file_path: Path to the document
                - collection: Collection name
                - chunk_size: Optional chunk size override
                - chunk_overlap: Optional chunk overlap override
                - metadata: Optional additional metadata
        
        Returns:
            Ingestion results with chunk count and document IDs
        """
        self._ensure_initialized()
        start_time = time.time()
        
        file_path = params.get("file_path")
        collection = params.get("collection", "default")
        chunk_size = params.get("chunk_size", self.config["chunk_size"])
        chunk_overlap = params.get("chunk_overlap", self.config["chunk_overlap"])
        metadata = params.get("metadata", {})
        
        if not file_path:
            raise ValueError("file_path is required")
        
        logger.info(f"Ingesting document: {file_path} into collection: {collection}")
        
        # Load document
        documents = self._load_document(file_path)
        logger.info(f"Loaded {len(documents)} document pages")
        
        # Create splitter with custom settings if provided
        if chunk_size != self.config["chunk_size"] or chunk_overlap != self.config["chunk_overlap"]:
            splitter = RecursiveCharacterTextSplitter(
                chunk_size=chunk_size,
                chunk_overlap=chunk_overlap,
                length_function=len,
                separators=["\n\n", "\n", " ", ""],
            )
        else:
            splitter = self.text_splitter
        
        # Split into chunks
        chunks = splitter.split_documents(documents)
        logger.info(f"Split into {len(chunks)} chunks")
        
        # Add metadata
        file_name = Path(file_path).name
        for i, chunk in enumerate(chunks):
            chunk.metadata.update({
                "source_file": file_name,
                "source_path": str(file_path),
                "chunk_index": i,
                "ingestion_time": datetime.now().isoformat(),
                **metadata,
            })
        
        # Get vector size from embeddings
        sample_embedding = self.embeddings.embed_query("test")
        vector_size = len(sample_embedding)
        
        # Ensure collection exists
        self._ensure_collection(collection, vector_size)
        
        # Store in Qdrant
        vectorstore = Qdrant(
            client=self.qdrant_client,
            collection_name=collection,
            embeddings=self.embeddings,
        )
        
        # Add documents and get IDs
        doc_ids = vectorstore.add_documents(chunks)
        
        processing_time_ms = int((time.time() - start_time) * 1000)
        logger.info(f"Ingestion complete: {len(doc_ids)} chunks stored in {processing_time_ms}ms")
        
        return {
            "collection": collection,
            "file_name": file_name,
            "chunks_count": len(chunks),
            "doc_ids": doc_ids,
            "processing_time_ms": processing_time_ms,
        }
    
    def query(self, params: Dict[str, Any]) -> Dict[str, Any]:
        """
        Query the RAG system.
        
        Args:
            params: Query parameters
                - query: The question to answer
                - collection: Collection to search
                - k: Number of documents to retrieve
                - score_threshold: Minimum similarity score
                - filter: Optional metadata filter
                - stream: Whether to stream the response (default: false)
        
        Returns:
            Query results with answer and source documents
        """
        self._ensure_initialized()
        
        query_text = params.get("query")
        collection = params.get("collection", "default")
        k = params.get("k", self.config["top_k"])
        score_threshold = params.get("score_threshold", self.config["score_threshold"])
        filter_dict = params.get("filter")
        stream = params.get("stream", False)
        
        if not query_text:
            raise ValueError("query is required")
        
        logger.info(f"=== QUERY START ===")
        logger.info(f"Query text: '{query_text}'")
        logger.info(f"Collection: '{collection}'")
        logger.info(f"Retrieval count (k): {k}")
        logger.info(f"Score threshold: {score_threshold}")
        logger.info(f"Filter: {filter_dict}")
        
        # Check if collection exists
        collections = [c.name for c in self.qdrant_client.get_collections().collections]
        logger.info(f"Available collections: {collections}")
        
        if collection not in collections:
            logger.warning(f"Collection '{collection}' not found")
            return {
                "answer": f"Collection '{collection}' not found. Please ingest documents first.",
                "sources": [],
                "num_sources": 0,
                "query": query_text,
                "collection": collection,
                "processing_time_ms": 0,
            }
        
        logger.info(f"Collection '{collection}' found")
        
        # Create vectorstore for retrieval
        vectorstore = Qdrant(
            client=self.qdrant_client,
            collection_name=collection,
            embeddings=self.embeddings,
        )
        
        logger.info(f"Performing similarity search...")
        
        # Perform similarity search
        results = vectorstore.similarity_search_with_score(
            query_text,
            k=k,
            filter=filter_dict,
        )
        
        logger.info(f"Similarity search returned {len(results)} results")
        logger.info(f"Raw results scores: {[score for _, score in results]}")
        
        # Filter by score threshold
        filtered_results = [
            (doc, score) for doc, score in results
            if score >= score_threshold
        ]
        
        logger.info(f"After threshold filter ({score_threshold}): {len(filtered_results)} results")
        logger.info(f"Filtered scores: {[score for _, score in filtered_results]}")
        
        logger.info(f"Retrieved {len(filtered_results)} relevant documents")
        
        # Format sources
        sources = []
        context_parts = []
        
        for doc, score in filtered_results:
            source = {
                "content": doc.page_content,
                "metadata": doc.metadata,
                "score": score,
            }
            sources.append(source)
            context_parts.append(doc.page_content)
            logger.info(f"Source score: {score}, file: {doc.metadata.get('source_file', 'unknown')}, chunk: {doc.metadata.get('chunk_index', '?')}")
        
        # Combine context for answer generation
        context = "\n\n---\n\n".join(context_parts) if context_parts else ""
        
        logger.info(f"Combined context length: {len(context)} chars")
        
        # Generate answer using LLM if available
        if context and self.llm:
            try:
                logger.info("Generating answer with LLM...")
                from langchain_core.messages import HumanMessage, SystemMessage
                
                system_message = SystemMessage(
                    content="You are a helpful AI assistant. Answer the user's question based on the provided context. "
                           "If the context doesn't contain enough information to answer the question, say so honestly."
                )
                
                user_message = HumanMessage(
                    content=f"""Context:
{context}

Question: {query_text}

Answer:"""
                )
                
                # Handle streaming vs non-streaming
                if stream and self.llm:
                    # For streaming, we'll return a special marker
                    # The actual streaming will be handled separately
                    logger.info("Streaming mode requested but not yet implemented - using non-streaming")
                    response = self.llm.invoke([system_message, user_message])
                    answer = response.content
                else:
                    # Non-streaming: invoke and return complete response
                    response = self.llm.invoke([system_message, user_message])
                    answer = response.content
                
                logger.info(f"LLM generated answer (length: {len(answer)})")
                
            except Exception as e:
                logger.error(f"LLM generation failed: {e}")
                answer = f"Retrieved relevant context but LLM generation failed: {str(e)}\n\nContext:\n{context}"
        else:
            # Fallback: return context without LLM processing
            if context:
                answer = f"Retrieved relevant context (LLM not available for answer generation):\n\n{context}"
            else:
                answer = "No relevant documents found for your query. Please try rephrasing or ensure relevant documents have been ingested."
        
        result = {
            "answer": answer,
            "sources": sources,
            "num_sources": len(sources),
            "query": query_text,
            "collection": collection,
            "processing_time_ms": 0,
        }
        
        logger.info(f"=== QUERY END ===")
        logger.info(f"Final sources count: {len(sources)}")
        
        return result
    
    def get_info(self) -> Dict[str, Any]:
        """
        Get service and pipeline information.
        
        Returns:
            Service configuration and status
        """
        collections_info = []
        
        if self.qdrant_client:
            try:
                for collection in self.qdrant_client.get_collections().collections:
                    info = self.qdrant_client.get_collection(collection.name)
                    collections_info.append({
                        "name": collection.name,
                        "vectors_count": info.vectors_count,
                        "points_count": info.points_count,
                    })
            except Exception as e:
                logger.warning(f"Error getting collections info: {e}")
        
        return {
            "version": self.version,
            "initialized": self.initialized,
            "config": self.config,
            "collections": collections_info,
            "uptime_seconds": (datetime.now() - self.start_time).total_seconds(),
        }


def create_response(id: Any, result: Any = None, error: Dict = None) -> str:
    """Create a JSON-RPC 2.0 response."""
    response = {
        "jsonrpc": "2.0",
        "id": id,
    }
    
    if error:
        response["error"] = error
    else:
        response["result"] = result
    
    return json.dumps(response)


def create_error(code: int, message: str, data: Any = None) -> Dict:
    """Create a JSON-RPC 2.0 error object."""
    error = {
        "code": code,
        "message": message,
    }
    if data is not None:
        error["data"] = data
    return error


def main():
    """Main entry point for the LangChain service."""
    logger.info("Starting LangChain Service...")
    
    if not LANGCHAIN_AVAILABLE:
        logger.error("LangChain dependencies not available!")
        print(json.dumps({
            "jsonrpc": "2.0",
            "id": None,
            "error": create_error(
                JsonRpcError.INTERNAL_ERROR,
                "LangChain dependencies not installed"
            )
        }), flush=True)
        return
    
    service = LangChainService()
    
    # Method dispatch table
    methods = {
        "health": lambda params: service.health(),
        "shutdown": lambda params: service.shutdown(),
        "initialize": lambda params: service.initialize(params),
        "ingest": lambda params: service.ingest(params),
        "query": lambda params: service.query(params),
        "get_info": lambda params: service.get_info(),
    }
    
    logger.info("Service ready, listening for requests...")
    
    # Main request loop
    try:
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue
            
            try:
                # Parse request
                request = json.loads(line)
                
                # Validate JSON-RPC 2.0 structure
                if request.get("jsonrpc") != "2.0":
                    response = create_response(
                        request.get("id"),
                        error=create_error(JsonRpcError.INVALID_REQUEST, "Invalid JSON-RPC version")
                    )
                    print(response, flush=True)
                    continue
                
                method = request.get("method")
                params = request.get("params", {})
                request_id = request.get("id")
                
                logger.info(f"Received request: method={method}, id={request_id}")
                
                # Check method exists
                if method not in methods:
                    response = create_response(
                        request_id,
                        error=create_error(JsonRpcError.METHOD_NOT_FOUND, f"Method not found: {method}")
                    )
                    print(response, flush=True)
                    continue
                
                # Execute method
                try:
                    result = methods[method](params)
                    response = create_response(request_id, result=result)
                    
                    # Handle shutdown specially
                    if method == "shutdown":
                        print(response, flush=True)
                        logger.info("Shutdown complete, exiting")
                        break
                    
                except FileNotFoundError as e:
                    response = create_response(
                        request_id,
                        error=create_error(JsonRpcError.FILE_NOT_FOUND, str(e))
                    )
                except ValueError as e:
                    response = create_response(
                        request_id,
                        error=create_error(JsonRpcError.INVALID_PARAMS, str(e))
                    )
                except Exception as e:
                    logger.error(f"Error executing {method}: {e}\n{traceback.format_exc()}")
                    response = create_response(
                        request_id,
                        error=create_error(JsonRpcError.INTERNAL_ERROR, str(e))
                    )
                
                print(response, flush=True)
                
            except json.JSONDecodeError as e:
                response = create_response(
                    None,
                    error=create_error(JsonRpcError.PARSE_ERROR, f"Parse error: {e}")
                )
                print(response, flush=True)
            
    except KeyboardInterrupt:
        logger.info("Interrupted, shutting down")
    except Exception as e:
        logger.error(f"Fatal error: {e}\n{traceback.format_exc()}")
    finally:
        logger.info("Service stopped")


if __name__ == "__main__":
    main()
