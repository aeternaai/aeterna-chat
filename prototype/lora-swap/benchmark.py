"""
Benchmarking utilities for LoRA hot-swap performance.
"""
import time
import logging
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, asdict
from pathlib import Path
import json

from llamacpp_client import LlamacppClient

logger = logging.getLogger(__name__)


@dataclass
class BenchmarkResult:
    """Single benchmark measurement."""
    method: str  # "process-level", "runtime-api", "per-request"
    operation: str  # "load", "inference", "unload", "switch"
    duration_ms: float
    memory_before_mb: Optional[float] = None
    memory_after_mb: Optional[float] = None
    memory_delta_mb: Optional[float] = None
    success: bool = True
    error: Optional[str] = None
    metadata: Optional[Dict[str, Any]] = None


class BenchmarkRunner:
    """Run and collect benchmarks for different hot-swap methods."""
    
    def __init__(self, client: LlamacppClient):
        self.client = client
        self.results: List[BenchmarkResult] = []
    
    def measure_time(self, operation: str, method: str):
        """Context manager for timing operations."""
        class Timer:
            def __init__(self, runner, operation, method):
                self.runner = runner
                self.operation = operation
                self.method = method
                self.start_time = None
                self.mem_before = None
                self.mem_after = None
                
            def __enter__(self):
                self.mem_before = self.runner.client.get_memory_usage()
                self.start_time = time.time()
                return self
                
            def __exit__(self, exc_type, exc_val, exc_tb):
                duration_ms = (time.time() - self.start_time) * 1000
                self.mem_after = self.runner.client.get_memory_usage()
                
                mem_before_mb = self.mem_before.get("rss_mb") if self.mem_before else None
                mem_after_mb = self.mem_after.get("rss_mb") if self.mem_after else None
                mem_delta_mb = None
                if mem_before_mb and mem_after_mb:
                    mem_delta_mb = mem_after_mb - mem_before_mb
                
                result = BenchmarkResult(
                    method=self.method,
                    operation=self.operation,
                    duration_ms=duration_ms,
                    memory_before_mb=mem_before_mb,
                    memory_after_mb=mem_after_mb,
                    memory_delta_mb=mem_delta_mb,
                    success=exc_type is None,
                    error=str(exc_val) if exc_val else None
                )
                
                self.runner.results.append(result)
                logger.info(f"{method} - {operation}: {duration_ms:.2f}ms, Δmem: {mem_delta_mb:.2f}MB" if mem_delta_mb else f"{method} - {operation}: {duration_ms:.2f}ms")
                
                return False  # Don't suppress exceptions
        
        return Timer(self, operation, method)
    
    def benchmark_inference(
        self,
        messages: List[Dict[str, str]],
        method: str,
        lora_adapters: Optional[List[Dict[str, Any]]] = None,
        iterations: int = 1
    ) -> List[Dict[str, Any]]:
        """
        Benchmark inference with optional LoRA adapters.
        
        Args:
            messages: Chat messages
            method: Benchmark method name
            lora_adapters: LoRA adapters for per-request method
            iterations: Number of iterations to average
            
        Returns:
            List of API responses
        """
        responses = []
        for i in range(iterations):
            with self.measure_time(f"inference_iter_{i+1}", method):
                response = self.client.chat_completion(
                    messages=messages,
                    lora_adapters=lora_adapters
                )
                responses.append(response)
        
        return responses
    
    def save_results(self, output_path: Path):
        """Save benchmark results to JSON file."""
        data = {
            "results": [asdict(r) for r in self.results],
            "summary": self.get_summary()
        }
        
        with open(output_path, "w") as f:
            json.dump(data, f, indent=2)
        
        logger.info(f"Saved results to {output_path}")
    
    def get_summary(self) -> Dict[str, Any]:
        """Calculate summary statistics."""
        if not self.results:
            return {}
        
        summary = {}
        
        # Group by method and operation
        by_method_op = {}
        for result in self.results:
            key = f"{result.method}_{result.operation}"
            if key not in by_method_op:
                by_method_op[key] = []
            by_method_op[key].append(result)
        
        # Calculate averages
        for key, results in by_method_op.items():
            durations = [r.duration_ms for r in results if r.success]
            memory_deltas = [r.memory_delta_mb for r in results if r.success and r.memory_delta_mb is not None]
            
            summary[key] = {
                "count": len(results),
                "success_count": sum(1 for r in results if r.success),
                "avg_duration_ms": sum(durations) / len(durations) if durations else None,
                "min_duration_ms": min(durations) if durations else None,
                "max_duration_ms": max(durations) if durations else None,
                "avg_memory_delta_mb": sum(memory_deltas) / len(memory_deltas) if memory_deltas else None,
            }
        
        return summary
    
    def print_summary(self):
        """Print human-readable summary to console."""
        summary = self.get_summary()
        
        print("\n" + "="*80)
        print("BENCHMARK SUMMARY")
        print("="*80)
        
        for key, stats in summary.items():
            method, operation = key.rsplit("_", 1)
            print(f"\n{method.upper()} - {operation}")
            print(f"  Iterations: {stats['count']} (success: {stats['success_count']})")
            if stats['avg_duration_ms']:
                print(f"  Duration: {stats['avg_duration_ms']:.2f}ms (min: {stats['min_duration_ms']:.2f}ms, max: {stats['max_duration_ms']:.2f}ms)")
            if stats['avg_memory_delta_mb']:
                print(f"  Memory Δ: {stats['avg_memory_delta_mb']:.2f}MB")
        
        print("\n" + "="*80)
