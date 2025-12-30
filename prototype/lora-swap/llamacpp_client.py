"""
HTTP client for llama-server API.
"""
import requests
import subprocess
import time
import psutil
from pathlib import Path
from typing import Dict, List, Optional, Any
from dataclasses import dataclass
import logging

logger = logging.getLogger(__name__)


@dataclass
class ServerInfo:
    """Information about running llama-server instance."""
    pid: int
    port: int
    host: str
    process: subprocess.Popen
    model_path: str
    api_key: str = "jan"


class LlamacppClient:
    """Client for interacting with llama-server via HTTP API."""
    
    def __init__(self, host: str = "127.0.0.1", port: int = 8080, api_key: str = "jan"):
        self.host = host
        self.port = port
        self.api_key = api_key
        self.base_url = f"http://{host}:{port}"
        self.server_info: Optional[ServerInfo] = None
        
    def start_server(
        self,
        binary_path: Path,
        model_path: Path,
        context_size: int = 2048,
        gpu_layers: int = 32,
        threads: int = 4,
        lora_paths: Optional[List[str]] = None,
        lora_scales: Optional[List[float]] = None,
        timeout: int = 60
    ) -> ServerInfo:
        """
        Start llama-server subprocess.
        
        Args:
            binary_path: Path to llama-server binary
            model_path: Path to model GGUF file
            context_size: Context window size
            gpu_layers: Number of layers to offload to GPU
            threads: Number of CPU threads
            lora_paths: List of LoRA adapter paths to load at startup
            lora_scales: Corresponding scales for each adapter (default: 1.0)
            timeout: Seconds to wait for server startup
            
        Returns:
            ServerInfo with process details
        """
        if self.server_info is not None:
            raise RuntimeError("Server already running. Call stop_server() first.")
        
        # Build command-line arguments
        args = [
            str(binary_path),
            "--host", self.host,
            "--port", str(self.port),
            "-m", str(model_path),
            "-c", str(context_size),
            "-ngl", str(gpu_layers),
            "-t", str(threads),
            "--api-key", self.api_key,
            "--log-format", "text",
        ]
        
        # Add LoRA adapters if specified
        if lora_paths:
            if lora_scales is None:
                lora_scales = [1.0] * len(lora_paths)
            elif len(lora_scales) != len(lora_paths):
                raise ValueError("Number of lora_scales must match lora_paths")
            
            for lora_path, scale in zip(lora_paths, lora_scales):
                if scale == 1.0:
                    args.extend(["--lora", str(lora_path)])
                else:
                    args.extend(["--lora-scaled", str(lora_path), str(scale)])
        
        logger.info(f"Starting llama-server: {' '.join(args)}")
        
        # Spawn process
        process = subprocess.Popen(
            args,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            universal_newlines=True
        )
        
        # Wait for server to be ready
        start_time = time.time()
        while time.time() - start_time < timeout:
            try:
                response = requests.get(f"{self.base_url}/health", timeout=1)
                if response.status_code == 200:
                    logger.info(f"Server ready (PID: {process.pid}, port: {self.port})")
                    self.server_info = ServerInfo(
                        pid=process.pid,
                        port=self.port,
                        host=self.host,
                        process=process,
                        model_path=str(model_path),
                        api_key=self.api_key
                    )
                    return self.server_info
            except requests.exceptions.RequestException:
                pass
            time.sleep(0.5)
        
        # Timeout - kill process
        process.kill()
        raise TimeoutError(f"Server failed to start within {timeout} seconds")
    
    def stop_server(self, graceful: bool = True, timeout: int = 10):
        """
        Stop llama-server subprocess.
        
        Args:
            graceful: Try SIGTERM before SIGKILL (Unix only)
            timeout: Seconds to wait for graceful shutdown
        """
        if self.server_info is None:
            logger.warning("No server running")
            return
        
        process = self.server_info.process
        logger.info(f"Stopping server (PID: {process.pid})")
        
        if graceful:
            process.terminate()
            try:
                process.wait(timeout=timeout)
                logger.info("Server stopped gracefully")
            except subprocess.TimeoutExpired:
                logger.warning("Graceful shutdown timeout, forcing kill")
                process.kill()
                process.wait()
        else:
            process.kill()
            process.wait()
        
        self.server_info = None
    
    def chat_completion(
        self,
        messages: List[Dict[str, str]],
        lora_adapters: Optional[List[Dict[str, Any]]] = None,
        temperature: float = 0.7,
        max_tokens: int = 512,
        stream: bool = False
    ) -> Dict[str, Any]:
        """
        Send chat completion request.
        
        Args:
            messages: List of message dicts with 'role' and 'content'
            lora_adapters: List of LoRA adapters [{"id": int, "scale": float}]
            temperature: Sampling temperature
            max_tokens: Maximum tokens to generate
            stream: Enable streaming (not implemented here)
            
        Returns:
            API response JSON
        """
        if self.server_info is None:
            raise RuntimeError("Server not running. Call start_server() first.")
        
        payload = {
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
            "stream": stream,
        }
        
        # Add LoRA adapters if specified (per-request switching)
        if lora_adapters:
            payload["lora"] = lora_adapters
        
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        
        logger.debug(f"Sending chat completion: {payload}")
        
        response = requests.post(
            f"{self.base_url}/v1/chat/completions",
            json=payload,
            headers=headers,
            timeout=300
        )
        response.raise_for_status()
        return response.json()
    
    def load_lora_runtime(self, lora_path: str, scale: float = 1.0) -> Dict[str, Any]:
        """
        Attempt to load LoRA adapter at runtime via API.
        
        This will test if llama-server supports /lora/load endpoint.
        
        Args:
            lora_path: Path to LoRA adapter file
            scale: Adapter scale factor
            
        Returns:
            API response JSON
            
        Raises:
            requests.HTTPError: If endpoint doesn't exist or fails
        """
        if self.server_info is None:
            raise RuntimeError("Server not running. Call start_server() first.")
        
        payload = {
            "path": lora_path,
            "scale": scale
        }
        
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        
        logger.info(f"Attempting runtime LoRA load: {lora_path} (scale={scale})")
        
        response = requests.post(
            f"{self.base_url}/lora/load",
            json=payload,
            headers=headers,
            timeout=30
        )
        response.raise_for_status()
        return response.json()
    
    def unload_lora_runtime(self, adapter_id: int) -> Dict[str, Any]:
        """
        Attempt to unload LoRA adapter at runtime via API.
        
        Args:
            adapter_id: ID of adapter to unload
            
        Returns:
            API response JSON
            
        Raises:
            requests.HTTPError: If endpoint doesn't exist or fails
        """
        if self.server_info is None:
            raise RuntimeError("Server not running. Call start_server() first.")
        
        payload = {"id": adapter_id}
        
        headers = {
            "Authorization": f"Bearer {self.api_key}",
            "Content-Type": "application/json"
        }
        
        logger.info(f"Attempting runtime LoRA unload: adapter_id={adapter_id}")
        
        response = requests.post(
            f"{self.base_url}/lora/unload",
            json=payload,
            headers=headers,
            timeout=30
        )
        response.raise_for_status()
        return response.json()
    
    def list_lora_runtime(self) -> Dict[str, Any]:
        """
        List currently loaded LoRA adapters via API.
        
        Returns:
            API response JSON with adapter list
            
        Raises:
            requests.HTTPError: If endpoint doesn't exist or fails
        """
        if self.server_info is None:
            raise RuntimeError("Server not running. Call start_server() first.")
        
        headers = {
            "Authorization": f"Bearer {self.api_key}",
        }
        
        response = requests.get(
            f"{self.base_url}/lora/list",
            headers=headers,
            timeout=10
        )
        response.raise_for_status()
        return response.json()
    
    def get_memory_usage(self) -> Optional[Dict[str, float]]:
        """
        Get memory usage of llama-server process.
        
        Returns:
            Dict with 'rss_mb' and 'vms_mb' keys, or None if no server running
        """
        if self.server_info is None:
            return None
        
        try:
            process = psutil.Process(self.server_info.pid)
            mem_info = process.memory_info()
            return {
                "rss_mb": mem_info.rss / 1024 / 1024,  # Resident Set Size
                "vms_mb": mem_info.vms / 1024 / 1024,  # Virtual Memory Size
            }
        except psutil.NoSuchProcess:
            logger.warning(f"Process {self.server_info.pid} not found")
            return None
