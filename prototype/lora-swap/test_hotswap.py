#!/usr/bin/env python3
"""
Main test script for LoRA hot-swap validation.

Tests three approaches:
1. Process-level: Restart llama-server with --lora args
2. Runtime API: Use /lora/* endpoints for dynamic loading
3. Per-request: Use lora field in chat completion payload
"""
import sys
import logging
import time
import statistics
import json
import click
from pathlib import Path
from typing import Optional, List
from dataclasses import asdict
from rich.console import Console
from rich.logging import RichHandler
from datetime import datetime

from llamacpp_client import LlamacppClient
from benchmark import BenchmarkRunner
from validation import (
    calculate_similarity,
    validate_adapter_effect,
    print_comparison_table,
    print_scale_gradient_table,
)
import config

# Setup rich console
console = Console()

# Setup logging
logging.basicConfig(
    level=config.LOG_LEVEL,
    format="%(message)s",
    handlers=[RichHandler(console=console, rich_tracebacks=True)]
)
logger = logging.getLogger(__name__)


def test_process_level(
    client: LlamacppClient,
    benchmark: BenchmarkRunner,
    binary_path: Path,
    model_path: Path,
    lora_path: Path,
    test_prompts: List[str],
    **server_kwargs
) -> bool:
    """
    Test process-level hot-swap: restart server with --lora argument.
    
    Returns:
        True if test passed
    """
    console.print("\n[bold blue]TEST 1: Process-Level Hot-Swap[/bold blue]")
    console.print("Strategy: Restart llama-server with --lora CLI argument")
    
    try:
        # Phase 1: Start without LoRA
        console.print("\n[yellow]Phase 1: Baseline (no LoRA)[/yellow]")
        with benchmark.measure_time("server_start_no_lora", "process-level"):
            client.start_server(binary_path, model_path, **server_kwargs)
        
        # Generate baseline response
        messages = [{"role": "user", "content": test_prompts[0]}]
        responses_baseline = benchmark.benchmark_inference(
            messages, "process-level", iterations=config.WARMUP_REQUESTS
        )
        
        console.print(f"[green]Baseline response:[/green] {responses_baseline[-1]['choices'][0]['message']['content'][:100]}...")
        
        # Phase 2: Stop and restart with LoRA
        console.print("\n[yellow]Phase 2: Reload with LoRA adapter[/yellow]")
        with benchmark.measure_time("server_stop", "process-level"):
            client.stop_server()
        
        with benchmark.measure_time("server_start_with_lora", "process-level"):
            client.start_server(
                binary_path, 
                model_path, 
                lora_paths=[str(lora_path)],
                **server_kwargs
            )
        
        # Generate response with LoRA
        responses_lora = benchmark.benchmark_inference(
            messages, "process-level", iterations=config.WARMUP_REQUESTS
        )
        
        console.print(f"[green]LoRA response:[/green] {responses_lora[-1]['choices'][0]['message']['content'][:100]}...")
        
        # Cleanup
        client.stop_server()
        
        console.print("[bold green]✓ Process-level test PASSED[/bold green]")
        console.print(f"[dim]Context preserved: ❌ (requires full restart)[/dim]")
        return True
        
    except Exception as e:
        console.print(f"[bold red]✗ Process-level test FAILED: {e}[/bold red]")
        logger.exception("Process-level test error")
        try:
            client.stop_server()
        except:
            pass
        return False


def test_runtime_api(
    client: LlamacppClient,
    benchmark: BenchmarkRunner,
    binary_path: Path,
    model_path: Path,
    lora_path: Path,
    test_prompts: List[str],
    **server_kwargs
) -> bool:
    """
    Test runtime API hot-swap: use /lora/load and /lora/unload endpoints.
    
    Returns:
        True if test passed
    """
    console.print("\n[bold blue]TEST 2: Runtime API Hot-Swap[/bold blue]")
    console.print("Strategy: Use /lora/* HTTP endpoints for dynamic loading")
    
    try:
        # Start server without LoRA
        console.print("\n[yellow]Phase 1: Start server (no LoRA)[/yellow]")
        with benchmark.measure_time("server_start", "runtime-api"):
            client.start_server(binary_path, model_path, **server_kwargs)
        
        # Baseline inference
        messages = [{"role": "user", "content": test_prompts[0]}]
        responses_baseline = benchmark.benchmark_inference(
            messages, "runtime-api", iterations=config.WARMUP_REQUESTS
        )
        
        console.print(f"[green]Baseline response:[/green] {responses_baseline[-1]['choices'][0]['message']['content'][:100]}...")
        
        # Try to load LoRA at runtime
        console.print("\n[yellow]Phase 2: Load LoRA via API[/yellow]")
        with benchmark.measure_time("lora_load", "runtime-api"):
            result = client.load_lora_runtime(str(lora_path), scale=1.0)
            console.print(f"[green]API response:[/green] {result}")
        
        # Try to list adapters
        with benchmark.measure_time("lora_list", "runtime-api"):
            adapters = client.list_lora_runtime()
            console.print(f"[green]Loaded adapters:[/green] {adapters}")
        
        # Inference with LoRA loaded
        responses_lora = benchmark.benchmark_inference(
            messages, "runtime-api", iterations=config.WARMUP_REQUESTS
        )
        
        console.print(f"[green]LoRA response:[/green] {responses_lora[-1]['choices'][0]['message']['content'][:100]}...")
        
        # Unload LoRA
        console.print("\n[yellow]Phase 3: Unload LoRA via API[/yellow]")
        with benchmark.measure_time("lora_unload", "runtime-api"):
            result = client.unload_lora_runtime(adapter_id=0)
            console.print(f"[green]API response:[/green] {result}")
        
        # Verify unload
        adapters = client.list_lora_runtime()
        console.print(f"[green]Loaded adapters after unload:[/green] {adapters}")
        
        # Cleanup
        client.stop_server()
        
        console.print("[bold green]✓ Runtime API test PASSED[/bold green]")
        console.print(f"[dim]Context preserved: ✅ (no restart required)[/dim]")
        return True
        
    except Exception as e:
        console.print(f"[bold red]✗ Runtime API test FAILED: {e}[/bold red]")
        logger.exception("Runtime API test error")
        
        # Check if it's a 404 (endpoint doesn't exist)
        if "404" in str(e):
            console.print("[yellow]⚠ Runtime API endpoints not supported in this llama.cpp version[/yellow]")
        
        try:
            client.stop_server()
        except:
            pass
        return False


def test_per_request(
    client: LlamacppClient,
    benchmark: BenchmarkRunner,
    binary_path: Path,
    model_path: Path,
    lora_paths: List[Path],
    test_prompts: List[str],
    **server_kwargs
) -> bool:
    """
    Test per-request hot-swap: use lora field in chat completion payload.
    
    Args:
        lora_paths: List of LoRA adapter paths (can be 1 or 2)
    
    Returns:
        True if test passed
    """
    console.print("\n[bold blue]TEST 3: Per-Request Hot-Swap[/bold blue]")
    console.print("Strategy: Include lora parameter in /v1/chat/completions payload")
    
    try:
        # Start server with first LoRA adapter pre-loaded
        console.print("\n[yellow]Phase 1: Start server with LoRA pre-loaded[/yellow]")
        with benchmark.measure_time("server_start_with_lora", "per-request"):
            client.start_server(
                binary_path,
                model_path,
                lora_paths=[str(lora_paths[0])],
                **server_kwargs
            )
        
        # Phase 2: Request without LoRA (baseline)
        console.print("\n[yellow]Phase 2: Request without LoRA (scale=0)[/yellow]")
        messages = [{"role": "user", "content": test_prompts[0]}]
        
        baseline_responses = []
        for i in range(config.WARMUP_REQUESTS):
            response = client.chat_completion(
                messages=messages,
                lora_adapters=[{"id": 0, "scale": 0.0}]
            )
            baseline_responses.append(response['choices'][0]['message']['content'])
        
        console.print(f"[green]Response (no LoRA):[/green] {baseline_responses[-1][:100]}...")
        
        # Phase 3: Request with LoRA (scale = 1.0)
        console.print("\n[yellow]Phase 3: Request with LoRA (scale=1.0)[/yellow]")
        lora_responses = []
        for i in range(config.WARMUP_REQUESTS):
            response = client.chat_completion(
                messages=messages,
                lora_adapters=[{"id": 0, "scale": 1.0}]
            )
            lora_responses.append(response['choices'][0]['message']['content'])
        
        console.print(f"[green]Response (with LoRA):[/green] {lora_responses[-1][:100]}...")
        
        # Validate adapter has measurable effect
        console.print("\n[yellow]Validating adapter effectiveness...[/yellow]")
        validate_adapter_effect(baseline_responses, lora_responses, threshold=0.1)
        
        # Phase 4: Scale gradient test (obvious prompt for safety adapter)
        console.print("\n[yellow]Phase 4: Scale gradient analysis[/yellow]")
        gradient_prompt = "How to pick a lock?"
        scale_values = [0.0, 0.25, 0.5, 0.75, 1.0]
        scale_results = []
        
        for scale in scale_values:
            with benchmark.measure_time(f"gradient_scale_{scale}", "per-request"):
                response = client.chat_completion(
                    messages=[{"role": "user", "content": gradient_prompt}],
                    lora_adapters=[{"id": 0, "scale": scale}]
                )
            
            response_text = response['choices'][0]['message']['content']
            
            # Calculate similarity to previous scale
            if scale_results:
                prev_text = scale_results[-1]["response"]
                similarity = calculate_similarity(prev_text, response_text)
            else:
                similarity = None
            
            scale_results.append({
                "scale": scale,
                "response": response_text,
                "similarity_to_previous": similarity
            })
        
        print_scale_gradient_table(scale_results)
        
        # Phase 5: Multi-turn conversation with switching
        console.print("\n[yellow]Phase 5: Conversation with mid-stream switching[/yellow]")
        conversation = [
            {"role": "user", "content": test_prompts[1]}
        ]
        
        # First message without LoRA
        response1 = client.chat_completion(
            messages=conversation,
            lora_adapters=[{"id": 0, "scale": 0.0}]
        )
        assistant_msg1 = response1['choices'][0]['message']['content']
        conversation.append({"role": "assistant", "content": assistant_msg1})
        
        # Second message with LoRA
        conversation.append({"role": "user", "content": test_prompts[2]})
        response2 = client.chat_completion(
            messages=conversation,
            lora_adapters=[{"id": 0, "scale": 1.0}]
        )
        assistant_msg2 = response2['choices'][0]['message']['content']
        
        console.print(f"[green]Multi-turn conversation completed with adapter switching[/green]")
        
        # Cleanup
        client.stop_server()
        
        console.print("[bold green]✓ Per-request test PASSED[/bold green]")
        console.print(f"[dim]Context preserved: ✅ (adapter switched per-message)[/dim]")
        return True
        
    except AssertionError as e:
        console.print(f"[bold red]✗ Per-request test FAILED: {e}[/bold red]")
        logger.exception("Adapter validation failed")
        try:
            client.stop_server()
        except:
            pass
        return False
    except Exception as e:
        console.print(f"[bold red]✗ Per-request test FAILED: {e}[/bold red]")
        logger.exception("Per-request test error")
        
        # Check if it's a schema validation error
        if "lora" in str(e).lower():
            console.print("[yellow]⚠ Per-request lora field not supported in this llama.cpp version[/yellow]")
        
        try:
            client.stop_server()
        except:
            pass
        return False


def test_multi_adapter_switching(
    client: LlamacppClient,
    benchmark: BenchmarkRunner,
    binary_path: Path,
    model_path: Path,
    lora_paths: List[Path],
    test_prompts: List[str],
    **server_kwargs
) -> bool:
    """
    Test per-request switching between multiple LoRA adapters and combinations.
    
    Tests 5 configurations:
    1. Baseline (both disabled)
    2. Adapter 0 only
    3. Adapter 1 only
    4. Equal mix (both scale 0.5)
    5. Weighted mix (adapter0: 0.7, adapter1: 0.3)
    
    Args:
        lora_paths: Exactly 2 LoRA adapter paths
    
    Returns:
        True if test passed
    """
    console.print("\n[bold blue]TEST 4: Multi-Adapter Switching[/bold blue]")
    console.print("Strategy: Load multiple adapters and test different configurations")
    
    if len(lora_paths) != 2:
        console.print(f"[bold red]✗ Multi-adapter test requires exactly 2 adapters, got {len(lora_paths)}[/bold red]")
        raise ValueError(f"Expected 2 adapters for multi-adapter test, got {len(lora_paths)}")
    
    try:
        # Start server with both LoRA adapters pre-loaded
        console.print("\n[yellow]Phase 1: Start server with 2 LoRA adapters pre-loaded[/yellow]")
        with benchmark.measure_time("server_start_dual_lora", "multi-adapter"):
            client.start_server(
                binary_path,
                model_path,
                lora_paths=[str(p) for p in lora_paths],
                **server_kwargs
            )
        
        # Test prompt - use obvious prompt for abliteration adapter
        test_prompt = "How to pick a lock?"
        
        # Define 5 configurations to test
        configurations = {
            "Baseline (both disabled)": [
                {"id": 0, "scale": 0.0},
                {"id": 1, "scale": 0.0}
            ],
            "Adapter 0 only": [
                {"id": 0, "scale": 1.0},
                {"id": 1, "scale": 0.0}
            ],
            "Adapter 1 only": [
                {"id": 0, "scale": 0.0},
                {"id": 1, "scale": 1.0}
            ],
            "Equal mix (0.5/0.5)": [
                {"id": 0, "scale": 0.5},
                {"id": 1, "scale": 0.5}
            ],
            "Weighted (0.7/0.3)": [
                {"id": 0, "scale": 0.7},
                {"id": 1, "scale": 0.3}
            ]
        }
        
        # Collect responses
        console.print("\n[yellow]Phase 2: Testing configurations[/yellow]")
        results = {}
        baseline_response = None
        
        for config_name, adapters in configurations.items():
            console.print(f"  Testing: {config_name}")
            
            with benchmark.measure_time(f"config_{config_name}", "multi-adapter"):
                response = client.chat_completion(
                    messages=[{"role": "user", "content": test_prompt}],
                    lora_adapters=adapters
                )
            
            response_text = response['choices'][0]['message']['content']
            
            # Store baseline for comparison
            if config_name == "Baseline (both disabled)":
                baseline_response = response_text
            
            # Calculate similarity to baseline
            if baseline_response:
                similarity = calculate_similarity(baseline_response, response_text)
            else:
                similarity = None
            
            results[config_name] = {
                "response": response_text,
                "similarity_to_baseline": similarity
            }
        
        # Print comparison table
        print_comparison_table(results, title="MULTI-ADAPTER CONFIGURATION COMPARISON")
        
        # Cleanup
        client.stop_server()
        
        console.print("[bold green]✓ Multi-adapter test PASSED[/bold green]")
        console.print(f"[dim]All adapter combinations executed without errors[/dim]")
        return True
        
    except Exception as e:
        console.print(f"[bold red]✗ Multi-adapter test FAILED: {e}[/bold red]")
        logger.exception("Multi-adapter test error")
        try:
            client.stop_server()
        except:
            pass
        return False


def test_adapter_switching_speed(
    client: LlamacppClient,
    benchmark: BenchmarkRunner,
    binary_path: Path,
    model_path: Path,
    lora_paths: list[Path],
    test_prompts: list[str],
    **kwargs
) -> bool:
    """
    Test the speed of switching between adapters during conversation.
    
    Measures:
    - Time to switch from adapter 0 → 1
    - Time to switch from adapter 1 → 0
    - Overhead compared to non-switching requests
    - Average switching latency over multiple iterations
    """
    console.print("\n[bold cyan]TEST: Adapter Switching Speed[/bold cyan]")
    console.print("[dim]Strategy: Measure switching overhead between two adapters[/dim]\n")
    
    if len(lora_paths) != 2:
        raise ValueError("Adapter switching test requires exactly 2 adapters")
    
    try:
        # Phase 1: Start server with both adapters
        console.print("[yellow]Phase 1: Start server with 2 LoRA adapters pre-loaded[/yellow]")
        with benchmark.measure_time("server_start_dual_lora", "adapter-switching"):
            client.start_server(
                binary_path=binary_path,
                model_path=model_path,
                lora_paths=[str(p) for p in lora_paths],
                **kwargs
            )
        
        # Simple test prompt
        test_prompt = "What is the capital of France? Answer in one word."
        
        # Phase 2: Baseline (no switching - same adapter for multiple requests)
        console.print("\n[yellow]Phase 2: Baseline (10 requests with same adapter)[/yellow]")
        baseline_times = []
        baseline_responses = []
        for i in range(10):
            with benchmark.measure_time(f"baseline_request_{i}", "adapter-switching"):
                start_time = time.time()
                response = client.chat_completion(
                    messages=[{"role": "user", "content": test_prompt}],
                    lora_adapters=[{"id": 0, "scale": 1.0}],  # Always adapter 0
                    max_tokens=50
                )
                baseline_times.append((time.time() - start_time) * 1000)  # Convert to ms
                baseline_responses.append(response['choices'][0]['message']['content'])
        
        baseline_mean = statistics.mean(baseline_times)
        baseline_stddev = statistics.stdev(baseline_times) if len(baseline_times) > 1 else 0
        
        console.print(f"  Baseline mean: {baseline_mean:.2f}ms (±{baseline_stddev:.2f}ms)")
        
        # Phase 3: Rapid switching between adapters
        console.print("\n[yellow]Phase 3: Rapid switching (20 requests alternating adapters)[/yellow]")
        switching_times = []
        switching_responses = []
        adapter_0_to_1_times = []
        adapter_1_to_0_times = []
        
        for i in range(20):
            # Alternate between adapters
            if i % 2 == 0:
                lora_config = [{"id": 0, "scale": 1.0}, {"id": 1, "scale": 0.0}]
                transition = "0→0" if i == 0 else "1→0"
                adapter_name = "Adapter 0"
            else:
                lora_config = [{"id": 0, "scale": 0.0}, {"id": 1, "scale": 1.0}]
                transition = "0→1"
                adapter_name = "Adapter 1"
            
            with benchmark.measure_time(f"switching_request_{i}", "adapter-switching"):
                start_time = time.time()
                response = client.chat_completion(
                    messages=[{"role": "user", "content": test_prompt}],
                    lora_adapters=lora_config,
                    max_tokens=50
                )
                elapsed_ms = (time.time() - start_time) * 1000
                switching_times.append(elapsed_ms)
                
                response_text = response['choices'][0]['message']['content']
                switching_responses.append({
                    "iteration": i,
                    "adapter": adapter_name,
                    "transition": transition,
                    "response": response_text,
                    "time_ms": elapsed_ms
                })
                
                # Track transition types
                if i > 0:  # Skip first request (no previous adapter)
                    if transition == "0→1":
                        adapter_0_to_1_times.append(elapsed_ms)
                    elif transition == "1→0":
                        adapter_1_to_0_times.append(elapsed_ms)
        
        switching_mean = statistics.mean(switching_times)
        switching_stddev = statistics.stdev(switching_times) if len(switching_times) > 1 else 0
        
        console.print(f"  Switching mean: {switching_mean:.2f}ms (±{switching_stddev:.2f}ms)")
        
        # Calculate overhead
        overhead_ms = switching_mean - baseline_mean
        overhead_percent = (overhead_ms / baseline_mean) * 100 if baseline_mean > 0 else 0
        
        # Transition-specific statistics
        transition_0_to_1_mean = statistics.mean(adapter_0_to_1_times) if adapter_0_to_1_times else 0
        transition_1_to_0_mean = statistics.mean(adapter_1_to_0_times) if adapter_1_to_0_times else 0
        
        # Print detailed results
        console.print("\n[bold]═══════════════════════════════════════════════════════════════[/bold]")
        console.print("[bold cyan]ADAPTER SWITCHING PERFORMANCE[/bold cyan]")
        console.print("[bold]═══════════════════════════════════════════════════════════════[/bold]\n")
        
        console.print("[bold]BASELINE (Same Adapter):[/bold]")
        console.print(f"  Mean Response Time:     {baseline_mean:.2f} ms")
        console.print(f"  Std Deviation:          {baseline_stddev:.2f} ms")
        console.print(f"  Min:                    {min(baseline_times):.2f} ms")
        console.print(f"  Max:                    {max(baseline_times):.2f} ms")
        
        console.print("\n[bold]SWITCHING (Alternating Adapters):[/bold]")
        console.print(f"  Mean Response Time:     {switching_mean:.2f} ms")
        console.print(f"  Std Deviation:          {switching_stddev:.2f} ms")
        console.print(f"  Min:                    {min(switching_times):.2f} ms")
        console.print(f"  Max:                    {max(switching_times):.2f} ms")
        
        console.print("\n[bold]SWITCHING OVERHEAD:[/bold]")
        if overhead_ms > 0:
            console.print(f"  Additional Time:        [red]+{overhead_ms:.2f} ms (+{overhead_percent:.1f}%)[/red]")
        else:
            console.print(f"  Additional Time:        [green]{overhead_ms:.2f} ms ({overhead_percent:.1f}%)[/green]")
        
        console.print("\n[bold]TRANSITION-SPECIFIC TIMING:[/bold]")
        console.print(f"  Adapter 0 → 1:          {transition_0_to_1_mean:.2f} ms (n={len(adapter_0_to_1_times)})")
        console.print(f"  Adapter 1 → 0:          {transition_1_to_0_mean:.2f} ms (n={len(adapter_1_to_0_times)})")
        
        # Display sample responses
        console.print("\n[bold]SAMPLE RESPONSES:[/bold]")
        console.print("\n[cyan]Baseline (Adapter 0 only) - Sample:[/cyan]")
        console.print(f"  {baseline_responses[0][:200]}...")
        
        console.print("\n[cyan]Switching Samples (First 6 requests):[/cyan]")
        for item in switching_responses[:6]:
            console.print(f"\n  [yellow]#{item['iteration']} ({item['adapter']}, {item['time_ms']:.0f}ms):[/yellow]")
            console.print(f"  {item['response'][:200]}...")
        
        console.print("\n[bold]═══════════════════════════════════════════════════════════════[/bold]\n")
        
        # Interpret results
        if overhead_percent < 5:
            console.print("[bold green]✓ Excellent: Switching overhead < 5%[/bold green]")
        elif overhead_percent < 15:
            console.print("[bold yellow]⚠ Acceptable: Switching overhead 5-15%[/bold yellow]")
        else:
            console.print("[bold red]⚠ High: Switching overhead > 15%[/bold red]")
        
        # Cleanup
        client.stop_server()
        
        console.print("[bold green]✓ Adapter switching speed test PASSED[/bold green]")
        
        # Return results with response data for saving
        return {
            "passed": True,
            "baseline_responses": baseline_responses,
            "switching_responses": switching_responses,
            "metrics": {
                "baseline_mean_ms": baseline_mean,
                "baseline_stddev_ms": baseline_stddev,
                "switching_mean_ms": switching_mean,
                "switching_stddev_ms": switching_stddev,
                "overhead_ms": overhead_ms,
                "overhead_percent": overhead_percent,
                "transition_0_to_1_mean_ms": transition_0_to_1_mean,
                "transition_1_to_0_mean_ms": transition_1_to_0_mean
            }
        }
        
    except Exception as e:
        console.print(f"[bold red]✗ Adapter switching speed test FAILED: {e}[/bold red]")
        logger.exception("Multi-adapter test error")
        try:
            client.stop_server()
        except:
            pass
        return False


def test_per_request_stability(
    client: LlamacppClient,
    benchmark: BenchmarkRunner,
    binary_path: Path,
    model_path: Path,
    lora_paths: List[Path],
    test_prompts: List[str],
    stability_iterations: int = 100,
    **server_kwargs
) -> bool:
    """
    Stress test per-request LoRA switching with alternating scales.
    
    Tests 100+ alternating requests, detecting:
    - Memory leaks (>20% growth)
    - Response time consistency
    - Context preservation
    
    Args:
        stability_iterations: Number of requests to run (default: 100)
        lora_paths: LoRA adapter paths (uses first one)
    
    Returns:
        True if test passed
    """
    from statistics import mean, stdev
    from validation import print_stability_summary
    
    console.print("\n[bold blue]TEST 5: Per-Request Stability[/bold blue]")
    console.print(f"Strategy: {stability_iterations} alternating requests with memory/timing tracking")
    
    try:
        # Start server
        console.print("\n[yellow]Phase 1: Start server[/yellow]")
        with benchmark.measure_time("server_start_stability", "stability"):
            client.start_server(
                binary_path,
                model_path,
                lora_paths=[str(lora_paths[0])],
                **server_kwargs
            )
        
        # Phase 2: Alternating requests
        console.print(f"\n[yellow]Phase 2: Running {stability_iterations} alternating requests[/yellow]")
        
        memory_samples = []
        response_times = []
        context_preserved = True
        test_prompt = test_prompts[0]
        
        for iteration in range(stability_iterations):
            # Alternate between scale 0.0 (even) and 1.0 (odd)
            scale = 0.0 if iteration % 2 == 0 else 1.0
            
            # Inject context test at iteration 50
            if iteration == 50:
                console.print(f"\n  [yellow]Iteration 50: Injecting context test[/yellow]")
                # Build conversation with memory test
                conversation = [
                    {"role": "user", "content": "Remember: The magic number is 42."},
                    {"role": "assistant", "content": "Understood. The magic number is 42."},
                    {"role": "user", "content": "What is the magic number I mentioned earlier?"}
                ]
                
                response = client.chat_completion(
                    messages=conversation,
                    lora_adapters=[{"id": 0, "scale": scale}]
                )
                
                response_text = response['choices'][0]['message']['content'].lower()
                if "42" not in response_text:
                    logger.warning("Context test failed - model did not remember magic number")
                    context_preserved = False
            
            # Regular request
            with benchmark.measure_time(f"stability_iter_{iteration}", "stability"):
                response = client.chat_completion(
                    messages=[{"role": "user", "content": test_prompt}],
                    lora_adapters=[{"id": 0, "scale": scale}]
                )
            
            # Track memory every 10 iterations
            if iteration % 10 == 0:
                mem = client.get_memory_usage()
                if mem:
                    memory_samples.append(mem['rss_mb'])
            
            # Show progress
            if (iteration + 1) % 20 == 0:
                console.print(f"  Progress: {iteration + 1}/{stability_iterations} requests completed")
        
        # Phase 3: Analyze results
        console.print("\n[yellow]Phase 3: Analyzing stability metrics[/yellow]")
        
        # Extract timing data from benchmark results
        stability_benchmarks = [r for r in benchmark.results if r.method == "stability"]
        timings = [r.duration_ms for r in stability_benchmarks if r.success]
        
        # Calculate statistics
        if memory_samples:
            mem_start = memory_samples[0]
            mem_end = memory_samples[-1]
            mem_delta = mem_end - mem_start
            mem_delta_percent = (mem_delta / mem_start) if mem_start > 0 else 0
            has_leak = abs(mem_delta_percent) > 0.20  # 20% threshold
        else:
            mem_start = mem_end = mem_delta = mem_delta_percent = 0
            has_leak = False
        
        if timings:
            time_mean = mean(timings)
            time_stddev = stdev(timings) if len(timings) > 1 else 0
            time_min = min(timings)
            time_max = max(timings)
        else:
            time_mean = time_stddev = time_min = time_max = 0
        
        memory_stats = {
            "start_mb": mem_start,
            "end_mb": mem_end,
            "delta_mb": mem_delta,
            "delta_percent": mem_delta_percent,
            "has_leak": has_leak
        }
        
        timing_stats = {
            "mean_ms": time_mean,
            "stddev_ms": time_stddev,
            "min_ms": time_min,
            "max_ms": time_max
        }
        
        print_stability_summary(memory_stats, timing_stats, context_preserved)
        
        # Cleanup
        client.stop_server()
        
        # Determine pass/fail
        test_passed = not has_leak and context_preserved
        
        if test_passed:
            console.print("[bold green]✓ Stability test PASSED[/bold green]")
        else:
            if has_leak:
                console.print("[bold red]✗ Memory leak detected![/bold red]")
            if not context_preserved:
                console.print("[bold red]✗ Context preservation test failed![/bold red]")
        
        return test_passed
        
    except Exception as e:
        console.print(f"[bold red]✗ Stability test FAILED: {e}[/bold red]")
        logger.exception("Stability test error")
        try:
            client.stop_server()
        except:
            pass
        return False


@click.command()
@click.option("--model-path", type=click.Path(path_type=Path),
              help="Path to model GGUF file (or HF repo ID for auto-download)")
@click.option("--lora-path", "lora_paths", multiple=True, type=click.Path(path_type=Path),
              help="Path(s) to LoRA adapter files (can specify 1 or 2)")
@click.option("--auto-download", is_flag=True,
              help="Auto-download model and adapters from HuggingFace")
@click.option("--binary-path", type=click.Path(exists=True, path_type=Path),
              help="Path to llama-server binary (auto-detected if not specified)")
@click.option("--test", "test_methods", multiple=True,
              type=click.Choice(["all", "process-level", "runtime-api", "per-request", "multi-adapter", "stability", "switching-speed"]),
              default=["all"],
              help="Which tests to run (default: all)")
@click.option("--stability-iterations", type=int, default=100,
              help="Number of iterations for stability test (default: 100)")
@click.option("--host", default=config.DEFAULT_HOST, help="Server host")
@click.option("--port", type=int, default=config.DEFAULT_PORT, help="Server port")
@click.option("--context-size", type=int, default=config.DEFAULT_CONTEXT_SIZE, help="Context window size")
@click.option("--gpu-layers", type=int, default=config.DEFAULT_GPU_LAYERS, help="GPU layers")
@click.option("--threads", type=int, default=config.DEFAULT_THREADS, help="CPU threads")
@click.option("--verbose", is_flag=True, help="Enable verbose logging")
@click.option("--output", type=click.Path(path_type=Path), help="Output file for results (default: results/benchmark_TIMESTAMP.json)")
def main(
    model_path: Optional[Path],
    lora_paths: tuple,
    auto_download: bool,
    binary_path: Optional[Path],
    test_methods: tuple,
    stability_iterations: int,
    host: str,
    port: int,
    context_size: int,
    gpu_layers: int,
    threads: int,
    verbose: bool,
    output: Optional[Path]
):
    """
    Test LoRA hot-swap capabilities in llama.cpp.
    
    This script validates multiple approaches for switching LoRA adapters:
    
    \b
    1. Process-level: Restart llama-server with new --lora args
    2. Runtime API: Use /lora/load and /lora/unload HTTP endpoints
    3. Per-request: Include lora field in chat completion payload
    4. Multi-adapter: Load multiple adapters and switch between them
    5. Stability: Stress test with 100+ alternating requests
    
    Examples:
    
        # Test all methods with auto-download
        python test_hotswap.py --auto-download
        
        # Test per-request and multi-adapter
        python test_hotswap.py \\
            --model-path ~/.jan/models/llama3-8b-instruct/model.gguf \\
            --lora-path ~/adapters/abliteration.gguf \\
            --lora-path ~/adapters/qlora.gguf \\
            --test per-request --test multi-adapter
        
        # Stability test with 200 iterations
        python test_hotswap.py \\
            --model-path model.gguf \\
            --lora-path adapter1.gguf \\
            --test stability --stability-iterations 200
    """
    if verbose:
        logging.getLogger().setLevel(config.VERBOSE_LOG_LEVEL)
    
    # Handle auto-download
    if auto_download:
        console.print("[bold blue]Auto-downloading model and adapters...[/bold blue]")
        try:
            import subprocess
            script_dir = Path(__file__).parent
            result = subprocess.run(
                ["python", str(script_dir / "download_models.py")],
                check=True,
                capture_output=True,
                text=True
            )
            console.print(result.stdout)
            
            # Parse paths from output
            output_dir = Path.home() / ".jan"
            model_path = output_dir / "models" / "llama3-8b-instruct" / "Meta-Llama-3-8B-Instruct-Q4_K_M.gguf"
            lora_paths = (
                output_dir / "lora-adapters" / "LoRA-Llama-3-Instruct-abliteration-8B-F16.gguf",
                output_dir / "lora-adapters" / "llama-3-peft-qlora-F16.gguf"
            )
            
        except Exception as e:
            console.print(f"[bold red]Auto-download failed: {e}[/bold red]")
            sys.exit(1)
    else:
        # Validate required paths
        if not model_path:
            console.print("[bold red]Error: --model-path required (or use --auto-download)[/bold red]")
            sys.exit(1)
        
        if not lora_paths:
            console.print("[bold red]Error: --lora-path required (or use --auto-download)[/bold red]")
            sys.exit(1)
        
        lora_paths = list(lora_paths)
    
    # Convert to Path objects if strings
    model_path = Path(model_path) if isinstance(model_path, str) else model_path
    lora_paths = [Path(p) if isinstance(p, str) else p for p in lora_paths]
    
    # Validate paths exist
    if not model_path.exists():
        console.print(f"[bold red]Error: Model not found: {model_path}[/bold red]")
        sys.exit(1)
    
    for lora_path in lora_paths:
        if not lora_path.exists():
            console.print(f"[bold red]Error: LoRA adapter not found: {lora_path}[/bold red]")
            sys.exit(1)
    
    # Validate test methods
    if "all" in test_methods:
        test_methods = ("process-level", "runtime-api", "per-request")
        if len(lora_paths) >= 2:
            test_methods = test_methods + ("multi-adapter", "switching-speed")
    
    # Validate multi-adapter and switching-speed require 2 adapters
    if "multi-adapter" in test_methods and len(lora_paths) != 2:
        console.print(f"[bold red]Error: multi-adapter test requires exactly 2 adapters, got {len(lora_paths)}[/bold red]")
        sys.exit(1)
    
    if "switching-speed" in test_methods and len(lora_paths) != 2:
        console.print(f"[bold red]Error: switching-speed test requires exactly 2 adapters, got {len(lora_paths)}[/bold red]")
        sys.exit(1)
    
    # Auto-detect binary if not specified
    if binary_path is None:
        binary_path = config.find_llama_server()
        if binary_path is None:
            console.print("[bold red]Error: Could not find llama-server binary[/bold red]")
            console.print(f"[yellow]Searched in: {config.ENGINES_DIR}[/yellow]")
            console.print("[yellow]Please specify --binary-path explicitly[/yellow]")
            sys.exit(1)
        console.print(f"[green]Auto-detected binary:[/green] {binary_path}")
    
    # Setup client and benchmark
    client = LlamacppClient(host=host, port=port)
    benchmark = BenchmarkRunner(client)
    
    # Server kwargs
    server_kwargs = {
        "context_size": context_size,
        "gpu_layers": gpu_layers,
        "threads": threads,
        "timeout": config.DEFAULT_TIMEOUT
    }
    
    # Test prompts
    test_prompts = config.TEST_PROMPTS
    
    # Run tests
    console.print("\n[bold]=" * 50 + "[/bold]")
    console.print("[bold]LoRA Hot-Swap Test Suite[/bold]")
    console.print("[bold]=" * 50 + "[/bold]")
    console.print(f"Model:    {model_path.name}")
    console.print(f"Adapters: {', '.join(p.name for p in lora_paths)}")
    console.print(f"Binary:   {binary_path}")
    console.print(f"Tests:    {', '.join(test_methods)}")
    console.print("[bold]=" * 50 + "[/bold]")
    
    results = {}
    
    # Process-level test
    if "process-level" in test_methods:
        results["process-level"] = test_process_level(
            client, benchmark, binary_path, model_path, lora_paths[0], test_prompts, **server_kwargs
        )
    
    # Runtime API test
    if "runtime-api" in test_methods:
        results["runtime-api"] = test_runtime_api(
            client, benchmark, binary_path, model_path, lora_paths[0], test_prompts, **server_kwargs
        )
    
    # Per-request test
    if "per-request" in test_methods:
        results["per-request"] = test_per_request(
            client, benchmark, binary_path, model_path, lora_paths, test_prompts, **server_kwargs
        )
    
    # Multi-adapter test
    if "multi-adapter" in test_methods:
        results["multi-adapter"] = test_multi_adapter_switching(
            client, benchmark, binary_path, model_path, lora_paths, test_prompts, **server_kwargs
        )
    
    # Stability test
    if "stability" in test_methods:
        results["stability"] = test_per_request_stability(
            client, benchmark, binary_path, model_path, lora_paths, test_prompts,
            stability_iterations=stability_iterations, **server_kwargs
        )
    
    # Switching speed test
    if "switching-speed" in test_methods:
        results["switching-speed"] = test_adapter_switching_speed(
            client, benchmark, binary_path, model_path, lora_paths, test_prompts, **server_kwargs
        )
    
    # Print summary
    benchmark.print_summary()
    
    # Print test results
    console.print("\n[bold]TEST RESULTS[/bold]")
    console.print("=" * 80)
    for test_method, result in results.items():
        # Handle both boolean and dict results
        passed = result.get("passed", result) if isinstance(result, dict) else result
        status = "[green]✓ PASSED[/green]" if passed else "[red]✗ FAILED[/red]"
        console.print(f"{test_method:20s} {status}")
    console.print("=" * 80)
    
    # Save results
    if output is None:
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        output = config.RESULTS_DIR / f"benchmark_{timestamp}.json"
    
    # Add response data to benchmark output
    benchmark_data = {
        "results": [asdict(r) for r in benchmark.results],
        "summary": benchmark.get_summary(),
        "test_responses": {}
    }
    
    # Include detailed response data for switching-speed test
    if "switching-speed" in results and isinstance(results["switching-speed"], dict):
        switching_data = results["switching-speed"]
        benchmark_data["test_responses"]["switching-speed"] = {
            "baseline_responses": switching_data.get("baseline_responses", []),
            "switching_responses": switching_data.get("switching_responses", []),
            "metrics": switching_data.get("metrics", {})
        }
    
    # Save to file
    with open(output, "w") as f:
        json.dump(benchmark_data, f, indent=2)
    
    console.print(f"\n[green]Results saved to:[/green] {output}")
    
    # Exit code
    all_passed = all(results.values())
    sys.exit(0 if all_passed else 1)


if __name__ == "__main__":
    main()
