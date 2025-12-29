"""RAG pipeline orchestration for document ingestion and querying."""

import time
from typing import Optional, List, Dict, Any
from pathlib import Path
from langchain_openai import ChatOpenAI
from langchain_core.messages import HumanMessage, SystemMessage
from qdrant_client import QdrantClient
from config import settings
from rag.project_manager import ProjectManager
from rag.document_processor import DocumentProcessor
from rag.vector_store import QdrantStore


class RAGPipeline:
    """
    Orchestrates the complete RAG workflow: ingestion and querying.
    
    Combines document processing, vector storage, and LLM generation
    to provide context-aware question answering.
    """

    def __init__(
        self,
        llm_model_path: Optional[Path] = None,
        embeddings_model_name: Optional[str] = None,
    ):
        """
        Initialize the RAG pipeline.

        Args:
            llm_model_path: DEPRECATED - Not used with HTTP server
            embeddings_model_name: HuggingFace model name for embeddings (defaults to settings)
        """
        print("🚀 Initializing RAG Pipeline...")

        # Create a single shared Qdrant client
        print("📦 Initializing Qdrant client...")
        self.qdrant_client = QdrantClient(path=str(settings.QDRANT_STORAGE_PATH))

        # Initialize components with shared client
        self.project_manager = ProjectManager(client=self.qdrant_client)
        self.document_processor = DocumentProcessor()
        self.vector_store = QdrantStore(
            embeddings_model_name=embeddings_model_name,
            client=self.qdrant_client,
        )

        # Initialize LLM with OpenAI-compatible client for llama.cpp server
        print(f"🤖 Connecting to llama.cpp server...")
        print(f"   Server URL: {settings.LLAMA_SERVER_URL}")

        # Check server health before initializing client
        is_healthy, message = settings.check_llama_server_health()
        if not is_healthy:
            raise ConnectionError(
                f"Cannot connect to llama.cpp server: {message}\n\n"
                f"Please ensure the server is running:\n"
                f"  ./llama-server -m /path/to/model.gguf -c {settings.LLM_CONTEXT_LENGTH} --port 8080\n\n"
                f"Or update LLAMA_SERVER_URL in .env file."
            )
        
        print(f"   ✓ {message}")

        self.llm = ChatOpenAI(
            base_url=settings.LLAMA_SERVER_URL,
            api_key=settings.LLAMA_API_KEY,
            model="gpt-3.5-turbo",  # Ignored by llama.cpp, but required by client
            max_tokens=settings.LLM_MAX_TOKENS,
            temperature=settings.LLM_TEMPERATURE,
            timeout=settings.LLAMA_SERVER_TIMEOUT,
            max_retries=settings.LLAMA_MAX_RETRIES,
            streaming=settings.LLAMA_STREAMING_ENABLED,
        )

        print(f"✓ RAG Pipeline initialized successfully")

    def _get_or_create_collection(
        self, project_id: Optional[str], project_name: Optional[str]
    ) -> str:
        """
        Get or create the appropriate collection for a project.

        Args:
            project_id: Project identifier (None for global)
            project_name: Project name (None for global)

        Returns:
            Collection name to use
        """
        if project_id is None:
            # Use global collection
            self.project_manager.ensure_global_collection()
            return "global"
        else:
            # Use or create project collection
            collection_name = f"project_{project_id}"
            if not self.project_manager.collection_exists(collection_name):
                self.project_manager.create_project_collection(
                    project_id=project_id, project_name=project_name or project_id
                )
            return collection_name

    def ingest_document(
        self,
        file_path: str | Path,
        project_id: Optional[str] = None,
        project_name: Optional[str] = None,
        chat_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        """
        Ingest a document into the RAG system.

        Processes the document, generates embeddings, and stores in appropriate collection.

        Args:
            file_path: Path to the document file
            project_id: Project identifier (None for global collection)
            project_name: Project name (None for global collection)
            chat_id: Chat identifier (None for project-wide context)

        Returns:
            Dictionary with ingestion results (collection, chunks_count, doc_ids)

        Raises:
            FileNotFoundError: If file doesn't exist
            ValueError: If processing or storage fails
        """
        print("=" * 60)
        print("DOCUMENT INGESTION")
        print("=" * 60)

        # Determine target collection
        collection_name = self._get_or_create_collection(project_id, project_name)
        print(f"📂 Target collection: {collection_name}")

        # Process document
        chunks = self.document_processor.process_file(
            file_path=file_path,
            project_id=project_id,
            project_name=project_name,
            chat_id=chat_id,
        )

        # Store in vector database
        doc_ids = self.vector_store.add_documents(
            collection_name=collection_name, documents=chunks
        )

        result = {
            "collection": collection_name,
            "file_name": Path(file_path).name,
            "chunks_count": len(chunks),
            "doc_ids": doc_ids,
            "project_id": project_id,
            "project_name": project_name or "Global",
            "chat_id": chat_id,
        }

        print("=" * 60)
        print("✓ INGESTION COMPLETE")
        print("=" * 60)

        return result

    def _build_prompt(self, query: str, context_docs: List[Any]) -> List:
        """
        Build chat messages for the LLM with retrieved context.

        Args:
            query: User's question
            context_docs: Retrieved documents from vector store

        Returns:
            List of chat messages (system + user)
        """
        # Build context string from retrieved documents
        context_parts = []
        for idx, doc in enumerate(context_docs, 1):
            source = doc.metadata.get("source_file", "Unknown")
            project = doc.metadata.get("project_name", "Unknown")
            score = doc.metadata.get("similarity_score", 0.0)

            context_parts.append(
                f"[Source {idx}: {source} (Project: {project}, Score: {score:.3f})]\n"
                f"{doc.page_content}\n"
            )

        context_str = "\n".join(context_parts)

        # Build chat messages
        system_message = SystemMessage(
            content="You are a helpful AI assistant. Answer the user's question based on the provided context. "
                    "If the context doesn't contain enough information to answer the question, say so honestly."
        )
        
        user_message = HumanMessage(
            content=f"""Context:
{context_str}

Question: {query}

Answer:"""
        )

        return [system_message, user_message]

    def query(
        self,
        question: str,
        project_id: Optional[str] = None,
        chat_id: Optional[str] = None,
        k: Optional[int] = None,
    ) -> Dict[str, Any]:
        """
        Query the RAG system with a question.

        Retrieves relevant context and generates an answer using the LLM.

        Args:
            question: User's question
            project_id: Project identifier (None for global collection)
            chat_id: Chat identifier for filtering (None for all chats in project)
            k: Number of documents to retrieve (defaults to settings)

        Returns:
            Dictionary with answer and source information

        Raises:
            ValueError: If collection doesn't exist or query fails
        """
        print("=" * 60)
        print("RAG QUERY")
        print("=" * 60)
        print(f"❓ Question: {question}")

        # Determine collection to query
        if project_id is None:
            collection_name = "global"
        else:
            collection_name = f"project_{project_id}"

        # Verify collection exists
        if not self.project_manager.collection_exists(collection_name):
            raise ValueError(
                f"Collection '{collection_name}' does not exist. "
                f"Please ingest documents first."
            )

        print(f"📂 Searching collection: {collection_name}")

        # Retrieve relevant documents
        relevant_docs = self.vector_store.similarity_search(
            collection_name=collection_name,
            query=question,
            k=k,
            chat_id_filter=chat_id,
        )

        if not relevant_docs:
            print("⚠ No relevant documents found")
            return {
                "answer": "I couldn't find any relevant information to answer your question.",
                "sources": [],
                "question": question,
                "collection": collection_name,
            }

        # Build prompt with context
        print(f"📝 Building prompt with {len(relevant_docs)} context documents...")
        messages = self._build_prompt(question, relevant_docs)

        # Generate answer with retry logic
        print(f"🤖 Generating answer...")
        max_retries = settings.LLAMA_MAX_RETRIES
        retry_delay = 1  # Start with 1 second delay
        
        for attempt in range(max_retries):
            try:
                # Invoke LLM (supports both streaming and non-streaming)
                if settings.LLAMA_STREAMING_ENABLED:
                    # For streaming, collect chunks
                    answer_chunks = []
                    for chunk in self.llm.stream(messages):
                        if hasattr(chunk, 'content'):
                            answer_chunks.append(chunk.content)
                    answer = "".join(answer_chunks)
                else:
                    # Non-streaming mode
                    response = self.llm.invoke(messages)
                    answer = response.content if hasattr(response, 'content') else str(response)
                
                break  # Success, exit retry loop
                
            except Exception as e:
                if attempt < max_retries - 1:
                    print(f"⚠ Attempt {attempt + 1} failed: {e}")
                    print(f"  Retrying in {retry_delay} seconds...")
                    time.sleep(retry_delay)
                    retry_delay *= 2  # Exponential backoff
                else:
                    # Final attempt failed
                    raise ConnectionError(
                        f"Failed to generate answer after {max_retries} attempts: {e}\n"
                        f"Please check if llama.cpp server is running and accessible."
                    )

        # Extract sources
        sources = []
        for doc in relevant_docs:
            sources.append(
                {
                    "file": doc.metadata.get("source_file", "Unknown"),
                    "project": doc.metadata.get("project_name", "Unknown"),
                    "chunk": doc.page_content[:200] + "..."
                    if len(doc.page_content) > 200
                    else doc.page_content,
                    "score": doc.metadata.get("similarity_score", 0.0),
                    "chunk_index": doc.metadata.get("chunk_index", 0),
                }
            )

        result = {
            "answer": answer.strip(),
            "sources": sources,
            "question": question,
            "collection": collection_name,
            "num_sources": len(sources),
        }

        print("=" * 60)
        print("✓ QUERY COMPLETE")
        print("=" * 60)

        return result

    def get_pipeline_info(self) -> Dict[str, Any]:
        """
        Get information about the RAG pipeline configuration.

        Returns:
            Dictionary with pipeline configuration details
        """
        return {
            "llm_server_url": settings.LLAMA_SERVER_URL,
            "llm_streaming": settings.LLAMA_STREAMING_ENABLED,
            "llm_max_retries": settings.LLAMA_MAX_RETRIES,
            "embeddings_model": self.vector_store.embeddings_model_name,
            "chunk_size": self.document_processor.chunk_size,
            "chunk_overlap": self.document_processor.chunk_overlap,
            "retrieval_top_k": settings.RETRIEVAL_TOP_K,
            "llm_context_length": settings.LLM_CONTEXT_LENGTH,
            "llm_max_tokens": settings.LLM_MAX_TOKENS,
            "llm_temperature": settings.LLM_TEMPERATURE,
        }

    def cleanup(self):
        """Clean up resources and close connections."""
        try:
            # Close the shared Qdrant client
            if hasattr(self, 'qdrant_client') and self.qdrant_client:
                self.qdrant_client.close()
                print("✓ Closed Qdrant connection")
        except Exception as e:
            print(f"Warning: Error during cleanup: {e}")

    def __enter__(self):
        """Context manager entry."""
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        """Context manager exit."""
        self.cleanup()
        return False
