"""Command-line interface for the RAG system."""

import sys
from pathlib import Path
import click
from typing import Optional
from config import settings, download_all_models
from rag import RAGPipeline, ProjectManager


@click.group()
def cli():
    """
    Basic RAG - A RAG system using Langchain, Qdrant, and LlamaCpp.
    
    Project-based document organization with intelligent retrieval.
    """
    pass


@cli.command()
@click.option(
    "--force", is_flag=True, help="Force re-download even if models already exist"
)
@click.option(
    "--server-url", help="Override llama.cpp server URL (default: from .env)"
)
def setup(force: bool, server_url: Optional[str]):
    """
    Setup the RAG system: check server connection and initialize storage.
    
    IMPORTANT: You must start llama.cpp server separately before running this command:
    
      ./llama-server -m /path/to/model.gguf -c 8192 --port 8080
    
    Downloads:
    - Embeddings model (auto-downloaded on first use, ~2 GB)
    """
    click.echo("=" * 60)
    click.echo("RAG SYSTEM SETUP")
    click.echo("=" * 60)
    
    try:
        # Ensure directories exist
        settings.ensure_directories()
        click.echo(f"✓ Created directories:")
        click.echo(f"  - Models: {settings.MODELS_DIR}")
        click.echo(f"  - Qdrant: {settings.QDRANT_STORAGE_PATH}")
        
        # Override server URL if provided
        if server_url:
            settings.LLAMA_SERVER_URL = server_url
            click.echo(f"\n  Using custom server URL: {server_url}")
        
        # Check llama.cpp server health
        click.echo("\n" + "=" * 60)
        click.echo("Checking llama.cpp server connection...")
        click.echo("=" * 60)
        
        is_healthy, message = settings.check_llama_server_health()
        
        if is_healthy:
            click.echo(f"✓ {message}")
            click.echo(f"  Server URL: {settings.LLAMA_SERVER_URL}")
        else:
            click.echo(f"✗ {message}", err=True)
            click.echo("\n⚠ llama.cpp server is not running!", err=True)
            click.echo("\nPlease start the server first:", err=True)
            click.echo(f"  ./llama-server -m /path/to/model.gguf -c {settings.LLM_CONTEXT_LENGTH} --port 8080", err=True)
            click.echo("\nOr update LLAMA_SERVER_URL in .env file.", err=True)
            sys.exit(1)
        
        # Note about embeddings
        click.echo("\n" + "=" * 60)
        click.echo("Model Configuration")
        click.echo("=" * 60)
        click.echo(f"✓ LLM: Connected via HTTP server")
        click.echo(f"  Embeddings: {settings.EMBEDDINGS_MODEL_NAME}")
        click.echo(f"  (Will be auto-downloaded on first use)")
        
        click.echo("\n" + "=" * 60)
        click.echo("✓ SETUP COMPLETE")
        click.echo("=" * 60)
        click.echo("\nYou can now:")
        click.echo("  1. Ingest documents: python cli.py ingest <file>")
        click.echo("  2. Query system: python cli.py query '<question>'")
        click.echo("  3. List collections: python cli.py collections list")
        
    except Exception as e:
        click.echo(f"\n✗ Setup failed: {e}", err=True)
        sys.exit(1)


@cli.command()
@click.argument("file_path", type=click.Path(exists=True))
@click.option("--project-id", help="Project ID (omit for global collection)")
@click.option("--project-name", help="Project name (required if project-id provided)")
@click.option("--chat-id", help="Chat ID for chat-specific context")
def ingest(
    file_path: str,
    project_id: Optional[str],
    project_name: Optional[str],
    chat_id: Optional[str],
):
    """
    Ingest a document into the RAG system.
    
    Examples:
    
      # Ingest to global collection
      python cli.py ingest document.pdf
      
      # Ingest to project collection
      python cli.py ingest doc.pdf --project-id=proj1 --project-name="Project Alpha"
      
      # Ingest with chat context
      python cli.py ingest doc.pdf --project-id=proj1 --project-name="Alpha" --chat-id=chat123
    """
    # Validate project arguments
    if project_id and not project_name:
        click.echo(
            "✗ Error: --project-name is required when --project-id is provided",
            err=True,
        )
        sys.exit(1)
    
    if project_name and not project_id:
        click.echo(
            "✗ Error: --project-id is required when --project-name is provided",
            err=True,
        )
        sys.exit(1)
    
    try:
        # Initialize pipeline with context manager
        with RAGPipeline() as pipeline:
            # Ingest document
            result = pipeline.ingest_document(
                file_path=file_path,
                project_id=project_id,
                project_name=project_name,
                chat_id=chat_id,
            )
            
            # Display results
            click.echo("\n📊 Ingestion Summary:")
            click.echo(f"  Collection: {result['collection']}")
            click.echo(f"  File: {result['file_name']}")
            click.echo(f"  Chunks: {result['chunks_count']}")
            click.echo(f"  Project: {result['project_name']}")
            if result['chat_id']:
                click.echo(f"  Chat ID: {result['chat_id']}")
        
    except Exception as e:
        click.echo(f"\n✗ Ingestion failed: {e}", err=True)
        sys.exit(1)


@cli.command()
@click.argument("question")
@click.option("--project-id", help="Project ID to query (omit for global)")
@click.option("--chat-id", help="Chat ID to filter results")
@click.option("--top-k", type=int, help="Number of documents to retrieve")
def query(
    question: str,
    project_id: Optional[str],
    chat_id: Optional[str],
    top_k: Optional[int],
):
    """
    Query the RAG system with a question.
    
    Examples:
    
      # Query global collection
      python cli.py query "What is machine learning?"
      
      # Query specific project
      python cli.py query "What is the budget?" --project-id=proj1
      
      # Query specific chat in project
      python cli.py query "What did we discuss?" --project-id=proj1 --chat-id=chat123
    """
    try:
        # Initialize pipeline with context manager
        with RAGPipeline() as pipeline:
            # Execute query
            result = pipeline.query(
                question=question,
                project_id=project_id,
                chat_id=chat_id,
                k=top_k,
            )
            
            # Display results
            click.echo("\n" + "=" * 60)
            click.echo("ANSWER")
            click.echo("=" * 60)
            click.echo(result["answer"])
        
        if result["sources"]:
            click.echo("\n" + "=" * 60)
            click.echo(f"SOURCES ({result['num_sources']})")
            click.echo("=" * 60)
            
            for idx, source in enumerate(result["sources"], 1):
                click.echo(f"\n{idx}. {source['file']} (Project: {source['project']})")
                click.echo(f"   Score: {source['score']:.3f} | Chunk: {source['chunk_index']}")
                click.echo(f"   Preview: {source['chunk']}")
        else:
            click.echo("\n(No sources found)")
        
        click.echo("\n" + "=" * 60)
        
    except Exception as e:
        click.echo(f"\n✗ Query failed: {e}", err=True)
        sys.exit(1)


@cli.group()
def collections():
    """Manage RAG collections."""
    pass


@collections.command("list")
def list_collections():
    """List all collections with statistics."""
    try:
        manager = ProjectManager()
        collections = manager.list_collections()
        
        # Close connection
        if hasattr(manager, 'client'):
            manager.client.close()
        
        if not collections:
            click.echo("No collections found.")
            return
        
        click.echo("\n" + "=" * 60)
        click.echo("COLLECTIONS")
        click.echo("=" * 60)
        
        for col in collections:
            click.echo(f"\n📂 {col['collection_name']}")
            click.echo(f"   Type: {col['type']}")
            if col['project_id']:
                click.echo(f"   Project ID: {col['project_id']}")
            click.echo(f"   Project: {col['project_name']}")
            click.echo(f"   Documents: {col['points_count']}")
            click.echo(f"   Vectors: {col['vectors_count']}")
        
        click.echo("\n" + "=" * 60)
        
    except Exception as e:
        click.echo(f"\n✗ Failed to list collections: {e}", err=True)
        sys.exit(1)


@collections.command("delete")
@click.argument("collection_name")
@click.option("--yes", is_flag=True, help="Skip confirmation prompt")
def delete_collection(collection_name: str, yes: bool):
    """Delete a collection."""
    try:
        manager = ProjectManager()
        
        # Verify collection exists
        if not manager.collection_exists(collection_name):
            click.echo(f"✗ Collection '{collection_name}' does not exist", err=True)
            sys.exit(1)
        
        # Get collection info
        info = manager.get_collection_info(collection_name)
        
        # Confirm deletion
        if not yes:
            click.echo(f"\n⚠ About to delete collection: {collection_name}")
            click.echo(f"   Type: {info['type']}")
            click.echo(f"   Project: {info['project_name']}")
            click.echo(f"   Documents: {info['points_count']}")
            
            if not click.confirm("\nAre you sure?"):
                click.echo("Cancelled.")
                return
        
        # Delete collection
        manager.delete_collection(collection_name)
        
        # Close connection
        if hasattr(manager, 'client'):
            manager.client.close()
        
        click.echo(f"\n✓ Deleted collection: {collection_name}")
        
    except Exception as e:
        click.echo(f"\n✗ Failed to delete collection: {e}", err=True)
        sys.exit(1)


@collections.command("info")
@click.argument("collection_name")
def collection_info(collection_name: str):
    """Get detailed information about a collection."""
    try:
        manager = ProjectManager()
        info = manager.get_collection_info(collection_name)
        
        # Close connection
        if hasattr(manager, 'client'):
            manager.client.close()
        
        click.echo("\n" + "=" * 60)
        click.echo("COLLECTION INFO")
        click.echo("=" * 60)
        click.echo(f"Name: {info['collection_name']}")
        click.echo(f"Type: {info['type']}")
        if info['project_id']:
            click.echo(f"Project ID: {info['project_id']}")
        click.echo(f"Project: {info['project_name']}")
        click.echo(f"Documents: {info['points_count']}")
        click.echo(f"Vectors: {info['vectors_count']}")
        click.echo(f"Vector Size: {info['vector_size']}")
        click.echo("=" * 60)
        
    except Exception as e:
        click.echo(f"\n✗ Failed to get collection info: {e}", err=True)
        sys.exit(1)


@cli.command()
def info():
    """Display system information and configuration."""
    try:
        click.echo("\n" + "=" * 60)
        click.echo("RAG SYSTEM INFORMATION")
        click.echo("=" * 60)
        
        # Configuration
        click.echo("\nConfiguration:")
        click.echo(f"  Qdrant Storage: {settings.QDRANT_STORAGE_PATH}")
        click.echo(f"  Models Directory: {settings.MODELS_DIR}")
        click.echo(f"  Chunk Size: {settings.CHUNK_SIZE}")
        click.echo(f"  Chunk Overlap: {settings.CHUNK_OVERLAP}")
        click.echo(f"  Retrieval Top-K: {settings.RETRIEVAL_TOP_K}")
        click.echo(f"  LLM Context: {settings.LLM_CONTEXT_LENGTH}")
        click.echo(f"  LLM Max Tokens: {settings.LLM_MAX_TOKENS}")
        click.echo(f"  LLM Temperature: {settings.LLM_TEMPERATURE}")
        
        # LLM Server
        click.echo("\nLLM Server:")
        click.echo(f"  URL: {settings.LLAMA_SERVER_URL}")
        click.echo(f"  Timeout: {settings.LLAMA_SERVER_TIMEOUT}s")
        click.echo(f"  Streaming: {settings.LLAMA_STREAMING_ENABLED}")
        click.echo(f"  Max Retries: {settings.LLAMA_MAX_RETRIES}")
        
        # Check server health
        is_healthy, message = settings.check_llama_server_health()
        click.echo(f"  Status: {'✓ ' + message if is_healthy else '✗ ' + message}")
        
        # Models
        click.echo("\nModels:")
        click.echo(f"  LLM: via HTTP server at {settings.LLAMA_SERVER_URL}")
        click.echo(f"  Embeddings: {settings.EMBEDDINGS_MODEL_NAME}")
        click.echo(f"    Type: HuggingFace (auto-downloaded on first use)")
        
        # Collections
        manager = ProjectManager()
        collections = manager.list_collections()
        
        # Close connection
        if hasattr(manager, 'client'):
            manager.client.close()
        
        click.echo(f"\nCollections: {len(collections)}")
        for col in collections:
            click.echo(f"  - {col['collection_name']} ({col['points_count']} documents)")
        
        click.echo("\n" + "=" * 60)
        
    except Exception as e:
        click.echo(f"\n✗ Failed to get system info: {e}", err=True)
        sys.exit(1)


if __name__ == "__main__":
    cli()
