#!/bin/bash

# Enable Flash Attention for speed
export OLLAMA_FLASH_ATTENTION=1

# Set default context to 128k (131072)
export OLLAMA_CONTEXT_LENGTH=131072

# Keep the model loaded for 1 hour so you don't wait for reloads
export OLLAMA_KEEP_ALIVE=60m

# Allow 2 massive models to run effectively if needed
export OLLAMA_MAX_LOADED_MODELS=2

echo "Starting Ollama on M3 Ultra Beast Mode..."
ollama serve
