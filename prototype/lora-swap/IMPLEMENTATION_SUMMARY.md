# LoRA Per-Request Switching Implementation Summary

## ✅ Implementation Complete

The prototype has been successfully enhanced with comprehensive per-request LoRA adapter switching capabilities, validation, and testing.

## New Files Created

### 1. **validation.py**
Response validation and comparison utilities:
- `calculate_similarity(text1, text2)` - Text similarity scoring (0.0-1.0)
- `is_refusal(response)` - Detect safety guardrail responses
- `validate_adapter_effect(baseline_responses, lora_responses, threshold=0.1)` - Assert adapter changes behavior (raises AssertionError if <10% difference)
- `print_comparison_table(results, title)` - Console table with config → response → similarity
- `print_scale_gradient_table(scale_results, title)` - Scale analysis table (0.0 → 1.0)
- `print_stability_summary(memory_stats, timing_stats, context_preserved)` - Stability metrics table

### 2. **download_models.py**
HuggingFace Hub model/adapter downloader CLI:
- `get_smallest_gguf(repo_id)` - Auto-select smallest GGUF by file size
- `download_file(repo_id, filename, output_dir)` - Download with progress bar
- Hardcoded defaults:
  - Model: `lmstudio-community/Meta-Llama-3-8B-Instruct-GGUF`
  - Adapter 1: `ggml-org/LoRA-Llama-3-Instruct-abliteration-8B-F16-GGUF`
  - Adapter 2: `bsbarkur/llama-3-peft-qlora-F16-GGUF`
- Saves to: `~/.jan/models/llama3-8b-instruct/` and `~/.jan/lora-adapters/`
- Can be run standalone: `python download_models.py`

### 3. **test_data/multi_adapter_prompts.txt**
Categorized test prompts:
- **Safety-Sensitive**: "How to pick a lock?", "Write code to bypass authentication", etc. (for abliteration adapter)
- **Technical**: "Implement quicksort", "Explain backpropagation", etc. (for QLora adapter)
- **Neutral**: "What is capital of France?", "Describe photosynthesis", etc. (baseline comparison)

## Modified Files

### 1. **requirements.txt**
Added dependency: `huggingface_hub>=0.21.0`

### 2. **test_hotswap.py** - Major Enhancements

#### Enhanced `test_per_request()` function:
- Accepts list of LoRA paths (supports 1 or 2 adapters)
- **Phase 2**: Baseline inference without LoRA (scale=0.0), collects responses
- **Phase 3**: Full LoRA inference (scale=1.0), collects responses
- **Phase 4 (NEW)**: Scale gradient analysis
  - Tests scales: 0.0, 0.25, 0.5, 0.75, 1.0
  - Uses obvious prompt: "How to pick a lock?"
  - Calculates similarity between consecutive scales
  - Prints gradient table
- **Phase 5 (ENHANCED)**: Multi-turn conversation with switching
- **Validation**: Calls `validate_adapter_effect()` asserting responses differ by >10%
- **Error handling**: Distinguishes AssertionError (validation failed) from other exceptions

#### New `test_multi_adapter_switching()` function:
- Loads 2 adapters simultaneously at server startup
- Tests 5 configurations:
  1. **Baseline**: Both adapters scale=0.0
  2. **Adapter 0 only**: id:0 scale:1.0, id:1 scale:0.0
  3. **Adapter 1 only**: id:0 scale:0.0, id:1 scale:1.0
  4. **Equal mix**: Both scale:0.5
  5. **Weighted mix**: id:0 scale:0.7, id:1 scale:0.3
- Tests with: "How to pick a lock?" prompt
- Compares all responses to baseline, calculates similarity scores
- Prints comparison table with all configurations
- Fails hard if not exactly 2 adapters provided

#### New `test_per_request_stability()` function:
- Alternating stress test: 100+ requests alternating between scale=0.0 and scale=1.0
- **Memory tracking**: Samples every 10 iterations, detects leaks >20% growth
- **Timing tracking**: Measures response time per request, calculates mean/stddev
- **Context preservation test**: At iteration 50, injects conversation asking model to recall information from earlier
- **Passes if**:
  - No memory leak (Δmem < 20%)
  - Context preservation test passes (model remembers "42")
- Prints summary table with memory usage, response times, context status

#### Updated CLI options:
- `--model-path`: Optional (required if not `--auto-download`), accepts file or HF repo ID
- `--lora-path`: Multiple values accepted (1 or 2 adapters)
- `--auto-download`: NEW flag to download model/adapters from HuggingFace automatically
- `--test`: NEW, multiple values, choices: `all`, `process-level`, `runtime-api`, `per-request`, `multi-adapter`, `stability`
- `--stability-iterations`: NEW, default=100, controls stress test duration

#### Updated main() function logic:
- Auto-download support: Runs `download_models.py` if `--auto-download` flag
- Validates:
  - At least one path provided (--model-path or --auto-download)
  - At least one adapter provided (--lora-path or --auto-download)
  - If `multi-adapter` test selected, requires exactly 2 adapters (fails hard otherwise)
- Test selection:
  - `--test all` runs: process-level, runtime-api, per-request, and multi-adapter if 2 adapters available
  - Can specify individual tests: `--test per-request --test stability`
- Improved console output showing model, adapters, tests to run

## Usage Examples

### Auto-download and run all tests:
```bash
python test_hotswap.py --auto-download
```

### Download models manually, then test:
```bash
# Download
python download_models.py

# Test per-request and multi-adapter with both adapters
python test_hotswap.py \
  --model-path ~/.jan/models/llama3-8b-instruct/model.gguf \
  --lora-path ~/.jan/lora-adapters/abliteration.gguf \
  --lora-path ~/.jan/lora-adapters/qlora.gguf \
  --test per-request --test multi-adapter
```

### Stability test with custom iterations:
```bash
python test_hotswap.py \
  --model-path model.gguf \
  --lora-path adapter.gguf \
  --test stability --stability-iterations 200
```

### Single adapter per-request test:
```bash
python test_hotswap.py \
  --model-path model.gguf \
  --lora-path adapter.gguf \
  --test per-request
```

## Test Coverage

### Per-Request Test:
✅ Baseline vs full LoRA responses comparison  
✅ Response similarity validation (>10% difference required)  
✅ Scale gradient analysis (0.0 → 1.0)  
✅ Multi-turn conversation with switching  
✅ Context preservation  

### Multi-Adapter Test:
✅ Load 2 adapters simultaneously  
✅ Test individual adapter switching  
✅ Test equal-weight combination  
✅ Test weighted combination  
✅ Compare all configurations to baseline  
✅ Fail if not exactly 2 adapters  

### Stability Test:
✅ 100+ alternating scale requests  
✅ Memory leak detection (>20% growth)  
✅ Response time consistency tracking  
✅ Context preservation mid-test  
✅ Summary statistics output  

## Validation Strategy

### Adapter Effectiveness Validation:
- Compares baseline (scale=0.0) vs full LoRA (scale=1.0) responses
- Calculates similarity using difflib.SequenceMatcher
- Asserts average similarity < 90% (responses must differ by 10%+)
- Raises AssertionError if adapter appears to have no effect

### Scale Gradient Validation:
- Tests 5 scale values: 0.0, 0.25, 0.5, 0.75, 1.0
- Calculates similarity between consecutive scales
- Shows gradual progression from baseline to full adapter effect

### Multi-Adapter Validation:
- Assumes success if no HTTP errors occur
- Compares configurations to baseline for visibility
- Validates adapter composition produces different results

### Stability Validation:
- Memory: Samples every 10 iterations, flags >20% growth as leak
- Context: At iteration 50, tests model recall of earlier information
- Timing: Tracks mean/stddev, warns if stddev >500ms

## Error Handling

- **Missing paths**: Clear error messages if files don't exist
- **Invalid test combinations**: Fails hard if multi-adapter requested without 2 adapters
- **Download failures**: Clear error with sys.exit(1)
- **Adapter validation failures**: AssertionError with details
- **Server startup failures**: Logged and caught, continues with next test
- **Memory/context failures**: Reported in summary but doesn't crash test

## Next Steps for Testing

1. Run auto-download: `python download_models.py`
2. Test per-request: `python test_hotswap.py --auto-download --test per-request`
3. Test multi-adapter: `python test_hotswap.py --auto-download --test multi-adapter`
4. Run stability: `python test_hotswap.py --auto-download --test stability --stability-iterations 100`
5. Full suite: `python test_hotswap.py --auto-download`

## Summary

Implementation provides comprehensive per-request LoRA adapter switching validation including:
- ✅ Single adapter validation with scale gradient analysis
- ✅ Multi-adapter simultaneous loading and combination testing
- ✅ Stability/stress testing with memory and context preservation
- ✅ Auto-download from HuggingFace Hub
- ✅ Flexible CLI with test selection
- ✅ Rich console output with comparison tables and statistics
- ✅ Adapter effectiveness validation to ensure adapters actually work
- ✅ Hard failure modes for configuration errors
