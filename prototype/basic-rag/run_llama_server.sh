#!/bin/bash
set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Default values
MODELS_DIR="./models"
LLAMA_CPP_DIR="./llama.cpp"
LLAMA_SERVER="$LLAMA_CPP_DIR/llama-server"
PORT=8080
CONTEXT_LENGTH=8192
N_GPU_LAYERS=99  # Offload all layers to GPU if available

# Function to print colored messages
print_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Check if model filename is provided
if [ -z "$1" ]; then
    print_error "Usage: $0 <model-filename> [options]"
    echo ""
    echo "Options:"
    echo "  --port <port>        Server port (default: 8080)"
    echo "  --context <length>   Context length (default: 8192)"
    echo "  --cpu                Force CPU-only mode (no GPU offloading)"
    echo ""
    echo "Example:"
    echo "  $0 Nemotron-3-Nano-30B-A3B-Q4_K_M.gguf"
    echo "  $0 Nemotron-3-Nano-30B-A3B-Q4_K_M.gguf --port 8081 --context 4096"
    exit 1
fi

MODEL_FILENAME="$1"
shift

# Parse optional arguments
CPU_ONLY=false
while [[ $# -gt 0 ]]; do
    case $1 in
        --port)
            PORT="$2"
            shift 2
            ;;
        --context)
            CONTEXT_LENGTH="$2"
            shift 2
            ;;
        --cpu)
            CPU_ONLY=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

MODEL_PATH="$MODELS_DIR/$MODEL_FILENAME"

# Check if model exists
if [ ! -f "$MODEL_PATH" ]; then
    print_error "Model not found: $MODEL_PATH"
    echo ""
    print_info "Available models in $MODELS_DIR:"
    ls -lh "$MODELS_DIR"/*.gguf 2>/dev/null || echo "  (none)"
    echo ""
    print_info "To download a model, run:"
    echo "  ./download_model.sh <huggingface-url>"
    exit 1
fi

# Check if llama-server exists
if [ ! -f "$LLAMA_SERVER" ]; then
    print_error "llama-server not found: $LLAMA_SERVER"
    print_info "Please run download_model.sh first to build llama-server"
    exit 1
fi

# Check if port is already in use
if lsof -Pi :$PORT -sTCP:LISTEN -t >/dev/null 2>&1; then
    print_error "Port $PORT is already in use"
    print_info "Either stop the existing process or use a different port:"
    echo "  $0 $MODEL_FILENAME --port <different-port>"
    exit 1
fi

# Build command
CMD="$LLAMA_SERVER -m $MODEL_PATH -c $CONTEXT_LENGTH --port $PORT"

if [ "$CPU_ONLY" = false ]; then
    # Add GPU offloading
    CMD="$CMD -ngl $N_GPU_LAYERS"
    print_info "GPU offloading enabled (all layers)"
else
    print_warning "Running in CPU-only mode"
fi

# Display configuration
print_info "Starting llama.cpp server..."
echo ""
echo "Configuration:"
echo "  Model: $MODEL_PATH"
echo "  Port: $PORT"
echo "  Context Length: $CONTEXT_LENGTH"
echo "  GPU Layers: $([ "$CPU_ONLY" = true ] && echo "0 (CPU only)" || echo "$N_GPU_LAYERS")"
echo "  API Endpoint: http://localhost:$PORT/v1"
echo "  Health Check: http://localhost:$PORT/health"
echo ""
print_info "Server logs will appear below. Press Ctrl+C to stop."
echo "=========================================="
echo ""

# Run the server
exec $CMD
