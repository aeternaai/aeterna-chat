# LoRA Hot-Swap Prototype

Proof-of-concept for runtime LoRA (Low-Rank Adaptation) adapter hot-swapping in llama.cpp.

## Overview

This prototype validates three approaches for LoRA adapter switching:
1. **Process-level**: Restart llama-server with new `--lora` CLI arguments
2. **Runtime API**: Use HTTP endpoints to load/unload adapters dynamically
3. **Per-request**: Include `lora` parameter in chat completion payloads

## Goals

- Validate llama.cpp LoRA hot-swap capabilities in current version (b6399+)
- Benchmark adapter loading times and memory overhead
- Test context preservation during adapter switches
- Recommend integration approach for Jan's llamacpp extension

## Prerequisites

```bash
# Install dependencies
pip install -r requirements.txt

# Ensure llama-server binary is available
# Default location: ~/.jan/engines/llamacpp/{version}/
```

## Quick Start

```bash
# Test all three approaches
python test_hotswap.py --model-path /path/to/model.gguf --lora-path /path/to/adapter.gguf

# Benchmark specific approach
python test_hotswap.py --model-path /path/to/model.gguf --lora-path /path/to/adapter.gguf --method runtime-api

# Run with verbose logging
python test_hotswap.py --model-path /path/to/model.gguf --lora-path /path/to/adapter.gguf --verbose
```

## Test Scenarios

### 1. Process-Level Hot-Swap
- Spawn llama-server without LoRA
- Generate baseline response
- Kill process, restart with `--lora` argument
- Generate response with adapter
- Measure: Restart time, context loss

### 2. Runtime API Hot-Swap
- Spawn llama-server
- POST to `/lora/load` endpoint (if exists)
- Generate response with adapter
- POST to `/lora/unload` endpoint
- Measure: API latency, memory delta, context preservation

### 3. Per-Request Hot-Swap
- Spawn llama-server
- Send chat completion with `lora: []` field
- Send chat completion with `lora: [{id: 0, scale: 1.0}]`
- Measure: Request overhead, context consistency

## Directory Structure

```
prototype/lora-swap/
├── README.md                    # This file
├── requirements.txt             # Python dependencies
├── test_hotswap.py             # Main test script
├── benchmark.py                # Performance benchmarking utilities
├── llamacpp_client.py          # llama-server HTTP client wrapper
├── config.py                   # Configuration and paths
├── test_data/                  # Test prompts and expected outputs
│   ├── baseline_prompts.txt
│   └── adapter_prompts.txt
└── results/                    # Benchmark results and logs
    └── .gitkeep
```

## Research Findings

### Llama.cpp LoRA Support Status

**Version Tested**: b6399+

**API Endpoints Discovered**:
- [ ] `/lora/load` - Runtime adapter loading
- [ ] `/lora/unload` - Runtime adapter unloading
- [ ] `/lora/list` - List loaded adapters
- [x] `/v1/chat/completions` with `lora` field - Per-request adapters (from OpenAPI schema)

**CLI Arguments**:
- `--lora <path>` - Load adapter at startup
- `--lora-scaled <path> <scale>` - Load with custom scale
- Multiple `--lora` flags supported: **TBD**

### Performance Results

*Results will be populated after testing*

| Method | Load Time | Memory Overhead | Context Preserved | Recommended |
|--------|-----------|-----------------|-------------------|-------------|
| Process-level | TBD | TBD | ❌ (requires restart) | ⏳ Testing |
| Runtime API | TBD | TBD | ✅ (if supported) | ⏳ Testing |
| Per-request | TBD | TBD | ✅ (request-scoped) | ⏳ Testing |

## Integration Recommendations

*To be determined after testing completes*

### Option A: Process-Level (Fallback)
- **When**: Runtime API unsupported
- **Implementation**: Modify `LlamacppExtension.load()` to accept `loraAdapters` parameter
- **Trade-offs**: Simple, reliable, but loses conversation context

### Option B: Runtime API (Preferred)
- **When**: Llama.cpp supports `/lora/*` endpoints
- **Implementation**: Add Tauri commands `load_lora_adapter`, `unload_lora_adapter`
- **Trade-offs**: Fast, preserves context, requires version check

### Option C: Per-Request (Most Flexible)
- **When**: OpenAPI `lora` field is functional
- **Implementation**: Modify inference requests in `useChat` hook
- **Trade-offs**: Per-message adapters, no state tracking needed

## Next Steps

1. ✅ Create prototype structure
2. ⏳ Research upstream llama.cpp documentation
3. ⏳ Implement test harness
4. ⏳ Run benchmarks
5. ⏳ Document findings
6. ⏳ Propose integration PR

## References

- [llama.cpp GitHub](https://github.com/ggerganov/llama.cpp)
- [Jan's llamacpp extension](../../extensions/llamacpp-extension/)
- [OpenAPI schema with LoRA field](../../extensions/llamacpp-extension/resources/inference-openapi.json#L474-L488)
- [Jan's architecture guide](../../CONTRIBUTING.md)

## Notes

- This prototype is independent of Jan's main codebase
- Uses direct HTTP calls to llama-server binary
- Results will guide implementation in production extension
- Consider security implications for adapter file validation
