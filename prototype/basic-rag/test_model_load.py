"""Test script to debug model loading."""

from pathlib import Path
from llama_cpp import Llama

model_path = Path("models/Nemotron-3-Nano-30B-A3B-Q4_K_M.gguf").resolve()
print(f"Attempting to load: {model_path}")
print(f"File exists: {model_path.exists()}")
print(f"File size: {model_path.stat().st_size / (1024**3):.2f} GB")

try:
    print("\nLoading model with minimal settings...")
    llm = Llama(
        model_path=str(model_path),
        n_ctx=512,  # Small context
        n_batch=128,  # Small batch
        n_threads=4,  # Limited threads
        verbose=True,
    )
    print("✓ Model loaded successfully!")
except Exception as e:
    print(f"✗ Failed to load model: {e}")
    import traceback
    traceback.print_exc()
