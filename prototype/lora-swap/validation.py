"""
Response validation and comparison utilities for LoRA hot-swap testing.
"""
import logging
from typing import List, Dict, Any
from difflib import SequenceMatcher

logger = logging.getLogger(__name__)


def calculate_similarity(text1: str, text2: str) -> float:
    """
    Calculate text similarity between 0.0 (completely different) and 1.0 (identical).
    
    Args:
        text1: First text string
        text2: Second text string
        
    Returns:
        Similarity score as float
    """
    if not text1 or not text2:
        return 0.0
    
    return SequenceMatcher(None, text1.lower(), text2.lower()).ratio()


def is_refusal(response: str) -> bool:
    """
    Detect if model refused to answer (safety guardrails active).
    
    Args:
        response: Model response text
        
    Returns:
        True if response appears to be a refusal
    """
    response_lower = response.lower()
    
    refusal_phrases = [
        "i cannot",
        "i can't",
        "cannot provide",
        "not able to",
        "not appropriate",
        "against my programming",
        "i apologize",
        "not something i can",
        "i'm not able",
        "i cannot assist",
        "i don't think",
        "not ethical",
        "safety policy",
        "i'm unable to",
        "i cannot help",
    ]
    
    return any(phrase in response_lower for phrase in refusal_phrases)


def validate_adapter_effect(
    baseline_responses: List[str],
    lora_responses: List[str],
    threshold: float = 0.1
) -> bool:
    """
    Validate that LoRA adapter actually changes model behavior.
    
    Calculates average similarity between baseline and LoRA responses.
    If average similarity >= (1.0 - threshold), adapter is not having effect.
    
    Args:
        baseline_responses: Responses without LoRA (scale=0.0)
        lora_responses: Responses with LoRA (scale=1.0)
        threshold: Minimum difference required (default 0.1 = 10% different)
        
    Returns:
        True if adapter has measurable effect
        
    Raises:
        AssertionError: If responses too similar (adapter ineffective)
        ValueError: If response lists have different lengths
    """
    if len(baseline_responses) != len(lora_responses):
        raise ValueError("Response lists must have same length")
    
    if not baseline_responses:
        raise ValueError("Response lists cannot be empty")
    
    # Calculate similarity for each pair
    similarities = []
    for baseline, lora in zip(baseline_responses, lora_responses):
        sim = calculate_similarity(baseline, lora)
        similarities.append(sim)
    
    avg_similarity = sum(similarities) / len(similarities)
    min_similarity = min(similarities)
    max_similarity = max(similarities)
    
    logger.info(f"Adapter effectiveness: avg_similarity={avg_similarity:.2%}, min={min_similarity:.2%}, max={max_similarity:.2%}")
    
    # Adapter is effective if responses differ by more than threshold
    required_difference = 1.0 - threshold
    
    if avg_similarity >= required_difference:
        logger.error(f"LoRA adapter appears to have NO EFFECT! Responses too similar ({avg_similarity:.2%})")
        raise AssertionError(
            f"Adapter validation FAILED: Average similarity {avg_similarity:.2%} >= {required_difference:.2%}. "
            f"Adapter does not appear to be changing model behavior."
        )
    
    logger.info(f"✓ Adapter validation PASSED: Responses differ significantly ({(1-avg_similarity):.2%} different)")
    return True


def print_comparison_table(
    results: Dict[str, Any],
    title: str = "COMPARISON TABLE"
) -> None:
    """
    Print comparison table of different LoRA configurations.
    
    Args:
        results: Dictionary with format {
            "config_name": {
                "response": "full response text",
                "similarity_to_baseline": float (0.0-1.0)
            }
        }
        title: Table title
    """
    print("\n" + "=" * 120)
    print(f"{title:^120}")
    print("=" * 120)
    
    # Header
    config_width = 20
    response_width = 70
    similarity_width = 15
    
    print(f"{'Config':<{config_width}} | {'Response Preview':<{response_width}} | {'Similarity':<{similarity_width}}")
    print("-" * 120)
    
    # Rows
    for config_name, data in results.items():
        response = data.get("response", "")
        similarity = data.get("similarity_to_baseline", None)
        
        # Truncate response to width
        response_preview = (response[:response_width-3] + "...") if len(response) > response_width else response
        
        # Format similarity
        if similarity is not None:
            similarity_str = f"{similarity:.2%}"
        else:
            similarity_str = "N/A"
        
        print(f"{config_name:<{config_width}} | {response_preview:<{response_width}} | {similarity_str:>{similarity_width}}")
    
    print("=" * 120 + "\n")


def print_scale_gradient_table(
    scale_results: List[Dict[str, Any]],
    title: str = "SCALE GRADIENT ANALYSIS"
) -> None:
    """
    Print scale gradient table showing how responses change across scales.
    
    Args:
        scale_results: List of dicts with format {
            "scale": float,
            "response": str,
            "similarity_to_previous": float (or None for first entry)
        }
        title: Table title
    """
    print("\n" + "=" * 120)
    print(f"{title:^120}")
    print("=" * 120)
    
    # Header
    scale_width = 10
    response_width = 70
    similarity_width = 20
    
    print(f"{'Scale':<{scale_width}} | {'Response Preview':<{response_width}} | {'Similarity to Previous':<{similarity_width}}")
    print("-" * 120)
    
    # Rows
    for entry in scale_results:
        scale = entry.get("scale", 0.0)
        response = entry.get("response", "")
        similarity = entry.get("similarity_to_previous")
        
        # Truncate response to width
        response_preview = (response[:response_width-3] + "...") if len(response) > response_width else response
        
        # Format similarity
        if similarity is not None:
            similarity_str = f"{similarity:.2%}"
        else:
            similarity_str = "(baseline)"
        
        print(f"{scale:<{scale_width}.2f} | {response_preview:<{response_width}} | {similarity_str:>{similarity_width}}")
    
    print("=" * 120 + "\n")


def print_stability_summary(
    memory_stats: Dict[str, float],
    timing_stats: Dict[str, float],
    context_preserved: bool
) -> None:
    """
    Print stability test summary table.
    
    Args:
        memory_stats: Dict with keys: start_mb, end_mb, delta_mb, delta_percent, has_leak
        timing_stats: Dict with keys: mean_ms, stddev_ms, min_ms, max_ms
        context_preserved: Whether context test passed
    """
    print("\n" + "=" * 100)
    print(f"{'STABILITY TEST SUMMARY':^100}")
    print("=" * 100)
    
    # Memory section
    print("\nMEMORY USAGE:")
    print(f"  Start:           {memory_stats['start_mb']:.2f} MB")
    print(f"  End:             {memory_stats['end_mb']:.2f} MB")
    print(f"  Delta:           {memory_stats['delta_mb']:.2f} MB ({memory_stats['delta_percent']:.2%})")
    
    leak_status = "⚠ LEAK DETECTED" if memory_stats['has_leak'] else "✓ OK (no leak)"
    print(f"  Leak Status:     {leak_status}")
    
    # Timing section
    print("\nRESPONSE TIMING:")
    print(f"  Mean:            {timing_stats['mean_ms']:.2f} ms")
    print(f"  Std Dev:         {timing_stats['stddev_ms']:.2f} ms")
    print(f"  Min:             {timing_stats['min_ms']:.2f} ms")
    print(f"  Max:             {timing_stats['max_ms']:.2f} ms")
    
    # Context section
    print("\nCONTEXT PRESERVATION:")
    context_status = "✓ PRESERVED" if context_preserved else "✗ FAILED"
    print(f"  Status:          {context_status}")
    
    print("\n" + "=" * 100 + "\n")
