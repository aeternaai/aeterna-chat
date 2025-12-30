#!/usr/bin/env python3
"""
Download models and LoRA adapters from HuggingFace Hub.

Automatically finds and downloads the smallest GGUF files for:
- Base model: Meta-Llama-3-8B-Instruct
- Adapter 1: Abliteration (removes safety guardrails)
- Adapter 2: PEFT-QLora fine-tuning adapter
"""
import sys
import click
from pathlib import Path
from typing import Optional, List, Tuple
from rich.console import Console
from rich.progress import track

console = Console()

try:
    from huggingface_hub import list_repo_files, hf_hub_download, HfApi
except ImportError:
    console.print("[bold red]Error: huggingface_hub not installed[/bold red]")
    console.print("Install with: pip install huggingface_hub")
    sys.exit(1)


def get_smallest_gguf(repo_id: str) -> Optional[str]:
    """
    Find the smallest GGUF file in a HuggingFace repository.
    
    Args:
        repo_id: Repository ID (e.g., 'lmstudio-community/Meta-Llama-3-8B-Instruct-GGUF')
        
    Returns:
        Filename of smallest GGUF, or None if not found
    """
    try:
        console.print(f"[yellow]Scanning {repo_id}...[/yellow]")
        api = HfApi()
        repo_files = api.list_repo_files(repo_id=repo_id)
        
        # Filter GGUF files
        gguf_files = [f for f in repo_files if f.endswith('.gguf')]
        
        if not gguf_files:
            console.print(f"[red]No GGUF files found in {repo_id}[/red]")
            return None
        
        # Get file info to find smallest
        file_info = {}
        for filename in gguf_files:
            try:
                info = api.file_info(repo_id=repo_id, filename=filename)
                size_gb = info.size / (1024**3)
                file_info[filename] = size_gb
                console.print(f"  {filename}: {size_gb:.2f} GB")
            except Exception as e:
                console.print(f"  [dim]{filename}: Could not get size ({e})[/dim]")
        
        if not file_info:
            # Fallback: return first GGUF
            return gguf_files[0]
        
        # Select smallest
        smallest_filename = min(file_info.keys(), key=lambda k: file_info[k])
        console.print(f"[green]Selected: {smallest_filename} ({file_info[smallest_filename]:.2f} GB)[/green]")
        
        return smallest_filename
        
    except Exception as e:
        console.print(f"[red]Error scanning {repo_id}: {e}[/red]")
        return None


def download_file(repo_id: str, filename: str, output_dir: Path) -> Optional[Path]:
    """
    Download a file from HuggingFace Hub.
    
    Args:
        repo_id: Repository ID
        filename: File to download
        output_dir: Directory to save to
        
    Returns:
        Path to downloaded file, or None if failed
    """
    try:
        console.print(f"[yellow]Downloading {filename}...[/yellow]")
        
        filepath = hf_hub_download(
            repo_id=repo_id,
            filename=filename,
            local_dir=str(output_dir),
            local_dir_use_symlinks=False
        )
        
        console.print(f"[green]✓ Downloaded to {filepath}[/green]")
        return Path(filepath)
        
    except Exception as e:
        console.print(f"[red]✗ Download failed: {e}[/red]")
        return None


@click.command()
@click.option(
    "--output-dir",
    type=click.Path(path_type=Path),
    default=None,
    help="Output directory (default: ~/.jan/)"
)
@click.option(
    "--model-repo",
    default="lmstudio-community/Meta-Llama-3-8B-Instruct-GGUF",
    help="Model repository ID"
)
@click.option(
    "--adapter1-repo",
    default="ggml-org/LoRA-Llama-3-Instruct-abliteration-8B-F16-GGUF",
    help="First adapter repository ID"
)
@click.option(
    "--adapter2-repo",
    default="bsbarkur/llama-3-peft-qlora-F16-GGUF",
    help="Second adapter repository ID"
)
def download_models(
    output_dir: Optional[Path],
    model_repo: str,
    adapter1_repo: str,
    adapter2_repo: str
):
    """
    Download model and LoRA adapters from HuggingFace Hub.
    
    Default repositories:
    - Model: lmstudio-community/Meta-Llama-3-8B-Instruct-GGUF
    - Adapter 1 (Abliteration): ggml-org/LoRA-Llama-3-Instruct-abliteration-8B-F16-GGUF
    - Adapter 2 (PEFT-QLora): bsbarkur/llama-3-peft-qlora-F16-GGUF
    
    Example:
        python download_models.py
        python download_models.py --output-dir /custom/path
    """
    # Setup directories
    if output_dir is None:
        output_dir = Path.home() / ".jan"
    
    output_dir = output_dir.resolve()
    models_dir = output_dir / "models" / "llama3-8b-instruct"
    adapters_dir = output_dir / "lora-adapters"
    
    models_dir.mkdir(parents=True, exist_ok=True)
    adapters_dir.mkdir(parents=True, exist_ok=True)
    
    console.print(f"[bold]LoRA Model & Adapter Downloader[/bold]")
    console.print(f"Models dir:  {models_dir}")
    console.print(f"Adapters dir: {adapters_dir}")
    console.print()
    
    # Download model
    console.print(f"[bold blue]1. Downloading Base Model[/bold blue]")
    model_filename = get_smallest_gguf(model_repo)
    if not model_filename:
        console.print("[bold red]FAILED: Could not find model[/bold red]")
        sys.exit(1)
    
    model_path = download_file(model_repo, model_filename, models_dir)
    if not model_path:
        console.print("[bold red]FAILED: Model download error[/bold red]")
        sys.exit(1)
    
    # Download adapter 1
    console.print(f"\n[bold blue]2. Downloading Adapter 1 (Abliteration)[/bold blue]")
    adapter1_filename = get_smallest_gguf(adapter1_repo)
    if not adapter1_filename:
        console.print("[bold red]FAILED: Could not find adapter 1[/bold red]")
        sys.exit(1)
    
    adapter1_path = download_file(adapter1_repo, adapter1_filename, adapters_dir)
    if not adapter1_path:
        console.print("[bold red]FAILED: Adapter 1 download error[/bold red]")
        sys.exit(1)
    
    # Download adapter 2
    console.print(f"\n[bold blue]3. Downloading Adapter 2 (PEFT-QLora)[/bold blue]")
    adapter2_filename = get_smallest_gguf(adapter2_repo)
    if not adapter2_filename:
        console.print("[bold red]FAILED: Could not find adapter 2[/bold red]")
        sys.exit(1)
    
    adapter2_path = download_file(adapter2_repo, adapter2_filename, adapters_dir)
    if not adapter2_path:
        console.print("[bold red]FAILED: Adapter 2 download error[/bold red]")
        sys.exit(1)
    
    # Success summary
    console.print(f"\n[bold green]✓ All downloads completed![/bold green]")
    console.print("\n[bold]Use these paths in test_hotswap.py:[/bold]")
    console.print(f"[cyan]python test_hotswap.py \\[/cyan]")
    console.print(f"[cyan]  --model-path {model_path} \\[/cyan]")
    console.print(f"[cyan]  --lora-path {adapter1_path} {adapter2_path}[/cyan]")
    console.print()


if __name__ == "__main__":
    download_models()
