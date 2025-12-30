"""
Configuration for LoRA hot-swap testing.
"""
import os
from pathlib import Path
from typing import Optional

# Paths
HOME_DIR = Path.home()
JAN_DIR = HOME_DIR / ".jan"
ENGINES_DIR = JAN_DIR / "engines" / "llamacpp"

# Default llama-server path (will auto-detect latest version)
def find_llama_server() -> Optional[Path]:
    """Find the latest llama-server binary in Jan's engines directory."""
    if not ENGINES_DIR.exists():
        return None
    
    # Find all version directories
    version_dirs = [d for d in ENGINES_DIR.iterdir() if d.is_dir()]
    if not version_dirs:
        return None
    
    # Sort by directory name (versions like b6399, b6400, etc.)
    version_dirs.sort(reverse=True)
    
    # Check each version for llama-server binary
    for version_dir in version_dirs:
        binary_name = "llama-server" if os.name != "nt" else "llama-server.exe"
        binary_path = version_dir / binary_name
        if binary_path.exists():
            return binary_path
    
    return None

# Server configuration
DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 8080
DEFAULT_TIMEOUT = 300  # seconds

# Model configuration
DEFAULT_CONTEXT_SIZE = 2048
DEFAULT_GPU_LAYERS = 32
DEFAULT_THREADS = 4

# Benchmark configuration
WARMUP_REQUESTS = 2
BENCHMARK_ITERATIONS = 5
TEST_PROMPTS = [
    "What is the capital of France?",
    "Explain quantum computing in simple terms.",
    "Write a haiku about programming.",
]

# LoRA configuration
DEFAULT_LORA_SCALE = 1.0

# Logging
LOG_LEVEL = "INFO"
VERBOSE_LOG_LEVEL = "DEBUG"

# Results
RESULTS_DIR = Path(__file__).parent / "results"
RESULTS_DIR.mkdir(exist_ok=True)
