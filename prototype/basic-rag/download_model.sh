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

# Check if URL is provided
if [ -z "$1" ]; then
    print_error "Usage: $0 <huggingface-model-url>"
    echo "Example: $0 https://huggingface.co/unsloth/Nemotron-3-Nano-30B-A3B-GGUF/resolve/main/Nemotron-3-Nano-30B-A3B-Q4_K_M.gguf"
    exit 1
fi

MODEL_URL="$1"

# Extract model filename from URL
MODEL_FILENAME=$(basename "$MODEL_URL")

# Create models directory if it doesn't exist
mkdir -p "$MODELS_DIR"

# Check if model already exists
MODEL_PATH="$MODELS_DIR/$MODEL_FILENAME"
if [ -f "$MODEL_PATH" ]; then
    print_warning "Model already exists: $MODEL_PATH"
    echo "Skipping download. Use --force to re-download."
else
    print_info "Downloading model: $MODEL_FILENAME"
    print_info "Destination: $MODEL_PATH"
    
    # Download with wget or curl
    if command -v wget &> /dev/null; then
        wget -O "$MODEL_PATH" "$MODEL_URL" --progress=bar:force:noscroll
    elif command -v curl &> /dev/null; then
        curl -L -o "$MODEL_PATH" "$MODEL_URL" --progress-bar
    else
        print_error "Neither wget nor curl found. Please install one of them."
        exit 1
    fi
    
    print_info "Download complete: $MODEL_PATH"
fi

# Check if llama.cpp is already cloned
if [ -d "$LLAMA_CPP_DIR" ]; then
    print_warning "llama.cpp directory already exists: $LLAMA_CPP_DIR"
    print_info "Pulling latest changes..."
    cd "$LLAMA_CPP_DIR"
    git pull
    cd ..
else
    print_info "Cloning llama.cpp repository..."
    git clone https://github.com/ggerganov/llama.cpp.git "$LLAMA_CPP_DIR"
fi

# Build llama-server
print_info "Building llama-server..."

# Check if CMake is installed
if ! command -v cmake &> /dev/null; then
    print_error "CMake is not installed. Please install it first:"
    echo ""
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "  brew install cmake"
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        echo "  sudo apt-get install cmake    # Debian/Ubuntu"
        echo "  sudo yum install cmake        # CentOS/RHEL"
    fi
    echo ""
    print_info "Or download from: https://cmake.org/download/"
    exit 1
fi

cd "$LLAMA_CPP_DIR"

# Create build directory
mkdir -p build
cd build

# Detect OS and build accordingly with CMake
if [[ "$OSTYPE" == "darwin"* ]]; then
    print_info "Detected macOS - building with Metal support..."
    cmake .. -DGGML_METAL=ON
elif command -v nvcc &> /dev/null; then
    print_info "Detected CUDA - building with CUDA support..."
    cmake .. -DGGML_CUDA=ON
else
    print_info "Building with CPU only..."
    cmake ..
fi

# Build
print_info "Compiling (this may take a few minutes)..."
cmake --build . --config Release --target llama-server

# Move binary to parent directory for easier access
if [ -f "bin/llama-server" ]; then
    cp bin/llama-server ../llama-server
elif [ -f "llama-server" ]; then
    cp llama-server ../llama-server
fi

cd ../..

print_info "✓ Setup complete!"
echo ""
print_info "Model location: $MODEL_PATH"
print_info "llama-server location: $LLAMA_CPP_DIR/llama-server"
echo ""
print_info "To start the server, run:"
echo "  ./run_llama_server.sh $MODEL_FILENAME"
