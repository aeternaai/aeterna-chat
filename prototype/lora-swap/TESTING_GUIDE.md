# LoRA Hot-Swap Testing Guide

## Quick Start

### 1. Install Dependencies

```bash
cd prototype/lora-swap
pip install -r requirements.txt
```

### 2. Prepare Test Assets

You'll need:
- A GGUF model file (e.g., Llama-7B, Phi-3, etc.)
- A LoRA adapter in GGUF format

**Where to get LoRA adapters**:
- HuggingFace models with GGUF conversions
- Convert your own with `convert-lora-to-gguf.py` from llama.cpp
- Use any fine-tuned adapter compatible with your base model

### 3. Run Tests

```bash
# Test all three methods
python test_hotswap.py \
  --model-path ~/.jan/models/your-model/model.gguf \
  --lora-path /path/to/adapter.gguf

# Test specific method
python test_hotswap.py \
  --model-path ~/.jan/models/your-model/model.gguf \
  --lora-path /path/to/adapter.gguf \
  --method per-request

# With custom binary location
python test_hotswap.py \
  --model-path ~/.jan/models/your-model/model.gguf \
  --lora-path /path/to/adapter.gguf \
  --binary-path /custom/path/to/llama-server

# Verbose logging
python test_hotswap.py \
  --model-path ~/.jan/models/your-model/model.gguf \
  --lora-path /path/to/adapter.gguf \
  --verbose
```

### 4. Review Results

Results are saved to `results/benchmark_TIMESTAMP.json` and displayed in the console.

## Test Methods Explained

### Process-Level Hot-Swap
- **How**: Restart llama-server with `--lora` CLI argument
- **Pros**: Simple, reliable, always works
- **Cons**: Slow (full restart), loses conversation context
- **Use case**: Fallback if other methods unsupported

### Runtime API Hot-Swap
- **How**: Use `/lora/load` and `/lora/unload` HTTP endpoints
- **Pros**: Fast, preserves context, no restart
- **Cons**: May not be available in current llama.cpp version
- **Use case**: Ideal if supported

### Per-Request Hot-Swap
- **How**: Include `lora` field in `/v1/chat/completions` payload
- **Pros**: Per-message adapters, preserves context, very flexible
- **Cons**: Adapter must be pre-loaded at startup
- **Use case**: Most likely to work, recommended for Jan integration

## Understanding the Output

### Success Indicators
```
✓ Process-level test PASSED
✓ Runtime API test PASSED
✓ Per-request test PASSED
```

### Failure Indicators
```
✗ Runtime API test FAILED: 404 Client Error
⚠ Runtime API endpoints not supported in this llama.cpp version
```

This is **expected** - most versions only support CLI + per-request methods.

### Performance Metrics
```
BENCHMARK SUMMARY
============================================================

PROCESS-LEVEL - server_start_with_lora
  Iterations: 1 (success: 1)
  Duration: 2847.32ms
  Memory Δ: 156.45MB

PER-REQUEST - inference_iter_1
  Iterations: 2 (success: 2)
  Duration: 1234.56ms
  Memory Δ: 2.34MB
```

**Key metrics to compare**:
- **Duration**: How long operations take
- **Memory Δ**: RAM overhead for loading adapters
- **Context preserved**: Whether conversation history survives

## Interpreting Results

### Scenario 1: Only Process-Level Works
```
✓ Process-level: PASSED
✗ Runtime API: FAILED (404)
✗ Per-request: FAILED (schema error)
```

**Action**: Use process-level in Jan, warn users about context loss

### Scenario 2: Per-Request Works (Expected)
```
✓ Process-level: PASSED
✗ Runtime API: FAILED (404)
✓ Per-request: PASSED
```

**Action**: ✅ Use per-request method in Jan - this is the recommended approach

### Scenario 3: All Methods Work (Ideal)
```
✓ Process-level: PASSED
✓ Runtime API: PASSED
✓ Per-request: PASSED
```

**Action**: ✅ Use runtime API for best UX, fallback to per-request

## Troubleshooting

### "Could not find llama-server binary"
```bash
# Check Jan's engine directory
ls ~/.jan/engines/llamacpp/

# Specify path explicitly
python test_hotswap.py --binary-path /path/to/llama-server ...
```

### "Server failed to start within 60 seconds"
- Model may be too large for your system
- GPU layers misconfigured
- Binary incompatible with your OS/architecture

Try:
```bash
python test_hotswap.py --gpu-layers 0 ...  # CPU only
python test_hotswap.py --context-size 1024 ...  # Smaller context
```

### "404 Client Error" for Runtime API
✅ **This is normal** - most llama.cpp versions don't have `/lora/*` endpoints yet.

The test will continue with other methods.

### LoRA Adapter Not Working
- Verify adapter is compatible with base model
- Check adapter format (must be GGUF, not SafeTensors/PyTorch)
- Try with `scale: 1.0` first
- Compare outputs with/without adapter to verify differences

## Next Steps After Testing

1. **Document findings** in `RESEARCH.md`
2. **Update README.md** with actual performance numbers
3. **Choose integration approach** based on test results
4. **Create PR for Jan** with LoRA support implementation

## Example Test Session

```bash
$ python test_hotswap.py \
    --model-path ~/.jan/models/llama-3-8b/model.gguf \
    --lora-path ~/lora-adapters/math-expert.gguf

========================================
LoRA Hot-Swap Test Suite
========================================
Model: model.gguf
LoRA: math-expert.gguf
Binary: /Users/user/.jan/engines/llamacpp/b6399/llama-server
========================================

TEST 1: Process-Level Hot-Swap
Strategy: Restart llama-server with --lora CLI argument

Phase 1: Baseline (no LoRA)
✓ Server started in 2.8s
✓ Baseline response: "The capital of France is Paris..."

Phase 2: Reload with LoRA adapter
✓ Server restarted with adapter in 3.1s
✓ LoRA response: "The capital of France is Paris, located..."

✓ Process-level test PASSED
Context preserved: ❌ (requires full restart)

[... more tests ...]

BENCHMARK SUMMARY
============================================================
[... results ...]
```

## Support

For questions or issues:
1. Check `RESEARCH.md` for detailed findings
2. Review `results/` directory for benchmark data
3. Consult Jan's architecture docs: `../../CONTRIBUTING.md`
4. Search llama.cpp issues: https://github.com/ggerganov/llama.cpp/issues
