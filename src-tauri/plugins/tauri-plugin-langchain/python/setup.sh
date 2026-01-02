#!/bin/bash

# LangChain Service Setup Script
# This script sets up the Python environment for the LangChain service

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VENV_DIR="$SCRIPT_DIR/venv"

echo "🔧 Setting up LangChain Service..."
echo "   Script directory: $SCRIPT_DIR"

# Check if Python 3 is available
if ! command -v python3 &> /dev/null; then
    echo "❌ Error: Python 3 is required but not found"
    echo "   Please install Python 3.10 or later"
    exit 1
fi

PYTHON_VERSION=$(python3 --version 2>&1 | cut -d' ' -f2)
echo "   Python version: $PYTHON_VERSION"

# Check minimum Python version (3.10+)
MAJOR=$(echo "$PYTHON_VERSION" | cut -d. -f1)
MINOR=$(echo "$PYTHON_VERSION" | cut -d. -f2)

if [ "$MAJOR" -lt 3 ] || ([ "$MAJOR" -eq 3 ] && [ "$MINOR" -lt 10 ]); then
    echo "❌ Error: Python 3.10 or later is required (found $PYTHON_VERSION)"
    exit 1
fi

# Create virtual environment
if [ -d "$VENV_DIR" ]; then
    echo "📂 Virtual environment already exists at $VENV_DIR"
    read -p "   Do you want to recreate it? (y/N) " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo "   Removing existing environment..."
        rm -rf "$VENV_DIR"
    fi
fi

if [ ! -d "$VENV_DIR" ]; then
    echo "📦 Creating virtual environment..."
    python3 -m venv "$VENV_DIR"
fi

# Activate virtual environment
echo "🔄 Activating virtual environment..."
source "$VENV_DIR/bin/activate"

# Upgrade pip
echo "⬆️  Upgrading pip..."
pip install --upgrade pip

# Install dependencies
echo "📥 Installing dependencies..."
pip install -r "$SCRIPT_DIR/requirements.txt"

# Verify installation
echo "✅ Verifying installation..."
python3 -c "
import langchain
import langchain_community
import langchain_huggingface
import qdrant_client
print('  LangChain:', langchain.__version__)
print('  Qdrant client: OK')
print('  HuggingFace embeddings: OK')
"

echo ""
echo "✨ LangChain Service setup complete!"
echo ""
echo "To run the service manually:"
echo "  source $VENV_DIR/bin/activate"
echo "  python3 $SCRIPT_DIR/langchain_service.py"
echo ""
echo "The service will be automatically started by the Jan application."
