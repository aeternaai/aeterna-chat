"""Configuration management for the RAG system."""

import os
from pathlib import Path
from typing import Optional
from dotenv import load_dotenv
from huggingface_hub import hf_hub_download

# Load environment variables from .env file
load_dotenv()

# Get the base directory (where config.py is located)
BASE_DIR = Path(__file__).parent.resolve()


class Settings:
    """Global settings for the RAG system."""

    # Qdrant Configuration
    QDRANT_STORAGE_PATH: Path = BASE_DIR / Path(
        os.getenv("QDRANT_STORAGE_PATH", "./qdrant_storage")
    )

    # Model Configuration
    MODELS_DIR: Path = BASE_DIR / Path(os.getenv("MODELS_DIR", "./models"))
    NEMOTRON_MODEL_NAME: str = os.getenv(
        "NEMOTRON_MODEL_NAME", "TheBloke/Llama-2-7B-Chat-GGUF"
    )
    NEMOTRON_MODEL_FILE: str = os.getenv(
        "NEMOTRON_MODEL_FILE", "llama-2-7b-chat.Q4_K_M.gguf"
    )
    EMBEDDINGS_MODEL_NAME: str = os.getenv(
        "EMBEDDINGS_MODEL_NAME", "nvidia/llama-nemotron-embed-1b-v2"
    )

    # Text Processing Configuration
    CHUNK_SIZE: int = int(os.getenv("CHUNK_SIZE", "512"))
    CHUNK_OVERLAP: int = int(os.getenv("CHUNK_OVERLAP", "64"))

    # LLM Generation Configuration
    LLM_CONTEXT_LENGTH: int = int(os.getenv("LLM_CONTEXT_LENGTH", "8192"))
    LLM_MAX_TOKENS: int = int(os.getenv("LLM_MAX_TOKENS", "1024"))
    LLM_TEMPERATURE: float = float(os.getenv("LLM_TEMPERATURE", "0.7"))

    # Retrieval Configuration
    RETRIEVAL_TOP_K: int = int(os.getenv("RETRIEVAL_TOP_K", "3"))
    RETRIEVAL_SCORE_THRESHOLD: float = float(
        os.getenv("RETRIEVAL_SCORE_THRESHOLD", "0.0")
    )

    # LLM Server Configuration (llama.cpp HTTP server)
    LLAMA_SERVER_URL: str = os.getenv("LLAMA_SERVER_URL", "http://127.0.0.1:8080/v1")
    LLAMA_API_KEY: str = os.getenv("LLAMA_API_KEY", "not-needed")
    LLAMA_SERVER_TIMEOUT: int = int(os.getenv("LLAMA_SERVER_TIMEOUT", "600"))
    LLAMA_STREAMING_ENABLED: bool = os.getenv("LLAMA_STREAMING_ENABLED", "true").lower() == "true"
    LLAMA_MAX_RETRIES: int = int(os.getenv("LLAMA_MAX_RETRIES", "3"))

    @classmethod
    def get_nemotron_model_path(cls) -> Path:
        """Get the path to the Nemotron model file."""
        return cls.MODELS_DIR / cls.NEMOTRON_MODEL_FILE

    @classmethod
    def ensure_directories(cls) -> None:
        """Ensure all required directories exist."""
        cls.MODELS_DIR.mkdir(parents=True, exist_ok=True)
        cls.QDRANT_STORAGE_PATH.mkdir(parents=True, exist_ok=True)

    @classmethod
    def check_llama_server_health(cls) -> tuple[bool, str]:
        """Check if llama.cpp server is running and healthy.
        
        Returns:
            Tuple of (is_healthy: bool, message: str)
        """
        import requests
        
        try:
            # Try multiple endpoints to check server health
            # First try /health endpoint
            health_url = cls.LLAMA_SERVER_URL.replace("/v1", "/health")
            try:
                response = requests.get(health_url, timeout=5)
                if response.status_code == 200:
                    return True, "llama.cpp server is running and healthy"
            except:
                pass
            
            # Fallback: try the /v1/models endpoint (OpenAI-compatible)
            models_url = f"{cls.LLAMA_SERVER_URL}/models"
            response = requests.get(models_url, timeout=5)
            
            if response.status_code == 200:
                return True, "llama.cpp server is running and healthy"
            else:
                return False, f"Server returned status code: {response.status_code}"
        except requests.exceptions.ConnectionError:
            return False, f"Cannot connect to server at {cls.LLAMA_SERVER_URL}"
        except requests.exceptions.Timeout:
            return False, "Connection timeout - server may be overloaded"
        except Exception as e:
            return False, f"Health check failed: {str(e)}"


def download_model(
    repo_id: str,
    filename: str,
    local_dir: Optional[Path] = None,
    force_download: bool = False,
) -> Path:
    """
    Download a model from HuggingFace Hub.

    Args:
        repo_id: The repository ID on HuggingFace (e.g., 'unsloth/Nemotron-3-Nano-30B-A3B-GGUF')
        filename: The specific file to download (e.g., 'model.gguf')
        local_dir: Local directory to save the model (defaults to Settings.MODELS_DIR)
        force_download: Whether to force re-download even if file exists

    Returns:
        Path to the downloaded model file
    """
    if local_dir is None:
        local_dir = Settings.MODELS_DIR

    local_dir.mkdir(parents=True, exist_ok=True)
    local_file = local_dir / filename

    # Check if file already exists
    if local_file.exists() and not force_download:
        print(f"✓ Model already exists: {local_file}")
        return local_file

    print(f"⬇ Downloading {filename} from {repo_id}...")
    print(f"  This may take a while depending on model size...")

    try:
        downloaded_path = hf_hub_download(
            repo_id=repo_id,
            filename=filename,
            local_dir=local_dir,
            local_dir_use_symlinks=False,
            force_download=force_download,
        )
        print(f"✓ Downloaded successfully to: {downloaded_path}")
        return Path(downloaded_path)
    except Exception as e:
        print(f"✗ Error downloading model: {e}")
        raise


def download_all_models(force: bool = False) -> Path:
    """
    DEPRECATED: This function is no longer needed when using llama.cpp HTTP server.
    
    The llama.cpp server manages model loading independently.
    Only embeddings model needs to be downloaded, which happens automatically.
    
    Note: Embeddings model (nvidia/llama-nemotron-embed-1b-v2) will be
    automatically downloaded by HuggingFace on first use.

    Args:
        force: Whether to force re-download even if files exist

    Returns:
        Path to models directory
    """
    Settings.ensure_directories()

    print("=" * 60)
    print("Model Setup")
    print("=" * 60)
    print("\n⚠ Note: When using llama.cpp HTTP server, models are managed")
    print("by the server independently. You need to:")
    print("\n1. Start llama.cpp server with your chosen model:")
    print("   ./llama-server -m /path/to/model.gguf -c 8192 --port 8080")
    print("\n2. Configure LLAMA_SERVER_URL in .env file")
    print("   LLAMA_SERVER_URL=http://localhost:8080/v1")
    print("\n✓ Embeddings model will be automatically downloaded")
    print(f"  from HuggingFace on first use: {Settings.EMBEDDINGS_MODEL_NAME}")
    print("\n" + "=" * 60)

    return Settings.MODELS_DIR


# Initialize settings
settings = Settings()
