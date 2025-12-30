# Quick Start Guide - LoRA Per-Request Switching Tests

## Installation

```bash
cd prototype/lora-swap
pip install -r requirements.txt
```

## Option 1: Auto-Download Everything (Easiest)

```bash
# Download model + both adapters and run all tests
python test_hotswap.py --auto-download

# Just download (don't run tests)
python download_models.py
```

## Option 2: Manual Download + Custom Tests

```bash
# Step 1: Download model and adapters
python download_models.py

# Step 2: Run specific test
python test_hotswap.py --auto-download --test per-request

# Step 3: Multi-adapter test
python test_hotswap.py --auto-download --test multi-adapter

# Step 4: Stability test (100 iterations)
python test_hotswap.py --auto-download --test stability

# Step 5: All tests
python test_hotswap.py --auto-download
```

## Test Descriptions

### Per-Request Test (TEST 3)
Tests single adapter switching with scale parameter (0.0 → 1.0).
- **Phase 1**: Server startup with adapter pre-loaded
- **Phase 2**: Baseline responses (scale=0.0)
- **Phase 3**: Full LoRA responses (scale=1.0)
- **Phase 4**: Scale gradient (0.0, 0.25, 0.5, 0.75, 1.0)
- **Phase 5**: Multi-turn conversation with mid-stream switching
- **Validation**: Asserts responses differ by >10%

### Multi-Adapter Test (TEST 4)
Tests 2 adapters simultaneously with different configurations.
- **Config 1**: Baseline (both disabled)
- **Config 2**: Adapter 0 only (abliteration)
- **Config 3**: Adapter 1 only (QLora)
- **Config 4**: Equal mix (0.5/0.5)
- **Config 5**: Weighted mix (0.7/0.3)
- **Output**: Comparison table with all responses and similarity scores

### Stability Test (TEST 5)
Stress test with 100+ alternating adapter requests.
- **Memory**: Detects leaks (>20% growth)
- **Timing**: Tracks response times (mean/stddev)
- **Context**: Tests memory recall at iteration 50
- **Output**: Summary table with all metrics

## Understanding the Output

### Scale Gradient Table (Per-Request Test, Phase 4)
```
SCALE GRADIENT ANALYSIS
Scale | Response Preview | Similarity to Previous
0.00  | How to pick... | (baseline)
0.25  | "I can provide information..." | 65.23%
0.50  | "While generally illegal..." | 54.12%
0.75  | "I must emphasize that lock picking..." | 42.89%
1.00  | "Here's detailed information..." | 31.45%
```
→ Shows how response changes as adapter scale increases

### Multi-Adapter Comparison Table (Multi-Adapter Test)
```
MULTI-ADAPTER CONFIGURATION COMPARISON
Config | Response Preview | Similarity to Baseline
Baseline (both disabled) | "I cannot assist..." | N/A
Adapter 0 only | "Lock picking can be useful..." | 35.67%
Adapter 1 only | "Lock mechanisms work..." | 42.13%
Equal mix (0.5/0.5) | "A combination answer..." | 45.89%
Weighted (0.7/0.3) | "The dominant approach..." | 38.21%
```
→ Shows how different adapter combinations affect responses

### Stability Summary (Stability Test)
```
STABILITY TEST SUMMARY
MEMORY USAGE:
  Start:           4124.50 MB
  End:             4156.78 MB
  Delta:           32.28 MB (0.78%)
  Leak Status:     ✓ OK (no leak)

RESPONSE TIMING:
  Mean:            1245.32 ms
  Std Dev:         128.45 ms
  Min:             987.12 ms
  Max:             2103.56 ms

CONTEXT PRESERVATION:
  Status:          ✓ PRESERVED
```
→ Shows system stability over many requests

## Interpreting Results

### ✅ Adapter Works if:
- Scale gradient shows increasing difference from baseline as scale → 1.0
- Response similarity decreases with higher scales
- Multi-adapter configurations show different responses
- No memory leaks detected in stability test
- Context preserved throughout testing

### ❌ Adapter Not Working if:
- Scale gradient responses are all identical (similarity ~100%)
- Validation fails with "responses too similar" assertion
- Multi-adapter all produce baseline-like responses
- Memory leak >20% detected
- Context not preserved (model forgets information)

### ⚠️ API Supports Feature if:
- No 400/404 HTTP errors
- No schema validation errors
- No "lora field not supported" warnings
- Test completes without API errors

## Troubleshooting

### Server Won't Start
```bash
python test_hotswap.py --gpu-layers 0 --auto-download  # CPU only
python test_hotswap.py --context-size 1024 --auto-download  # Smaller context
```

### Out of Memory
```bash
python test_hotswap.py --gpu-layers 16 --auto-download  # Fewer GPU layers
```

### Slow Download
```bash
# Network issue, try again or manually specify cached path
python test_hotswap.py --model-path /local/path/model.gguf --auto-download
```

## Configuration Options

```bash
--auto-download              # Download model + adapters
--model-path PATH            # Model GGUF file
--lora-path PATH             # Adapter file (can specify 1-2 times)
--test TYPE                  # per-request, multi-adapter, stability, all (can specify multiple)
--stability-iterations N     # Stress test iterations (default: 100)
--gpu-layers N              # GPU layers (default: 32)
--context-size N            # Context window (default: 2048)
--threads N                 # CPU threads (default: 4)
--binary-path PATH          # llama-server binary (auto-detected)
--verbose                   # Debug logging
--output FILE               # Save results to JSON
```

## Result Files

Results saved to `results/benchmark_TIMESTAMP.json` with format:
```json
{
  "results": [
    {"method": "per-request", "operation": "gradient_scale_0.0", "duration_ms": 1234.56, ...},
    ...
  ],
  "summary": {
    "per-request_inference_iter_1": {
      "count": 2,
      "success_count": 2,
      "avg_duration_ms": 1456.23,
      ...
    }
  }
}
```

## Next: Integration into Jan

After validation, results inform integration approach:
1. If per-request works → Use `lora` field in `/v1/chat/completions`
2. If runtime API works → Add Tauri commands for dynamic loading
3. If process-level only → Restart server with new adapters (context loss)

See `RESEARCH.md` for detailed findings and recommendations.
