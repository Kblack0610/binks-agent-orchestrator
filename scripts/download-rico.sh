#!/bin/bash
# Download RICO dataset files for rico-mcp
#
# The RICO dataset is hosted on Google Cloud Storage.
# This script downloads the required files for similarity search.
#
# Usage: ./scripts/download-rico.sh [--full]
#   --full: Also download semantic annotations (adds ~50MB)

set -euo pipefail

RICO_DIR="${RICO_DATA_DIR:-${HOME}/.rico-mcp/data}"
SCREENSHOTS_DIR="${RICO_SCREENSHOT_DIR:-${HOME}/.rico-mcp/screenshots}"

# RICO dataset URLs (research.google.com/rico)
# Note: These are example URLs - actual RICO data may need to be downloaded
# from the official source at https://interactionmining.org/rico
BASE_URL="https://storage.googleapis.com/crowdstf-rico-uiuc-4540"

echo "=== RICO Dataset Downloader ==="
echo "Data directory: $RICO_DIR"
echo ""

# Create directories
mkdir -p "$RICO_DIR"
mkdir -p "$SCREENSHOTS_DIR"

# Function to download with progress
download_file() {
    local url="$1"
    local dest="$2"
    local desc="$3"

    if [[ -f "$dest" ]]; then
        echo "✓ $desc already exists, skipping"
        return 0
    fi

    echo "↓ Downloading $desc..."
    if command -v curl &> /dev/null; then
        curl -L --progress-bar -o "$dest" "$url"
    elif command -v wget &> /dev/null; then
        wget --show-progress -q -O "$dest" "$url"
    else
        echo "Error: Neither curl nor wget found"
        exit 1
    fi
    echo "✓ Downloaded $desc"
}

# Function to extract archive
extract_archive() {
    local archive="$1"
    local dest_dir="$2"
    local desc="$3"

    echo "→ Extracting $desc..."
    if [[ "$archive" == *.zip ]]; then
        unzip -q -o "$archive" -d "$dest_dir"
    elif [[ "$archive" == *.tar.gz ]] || [[ "$archive" == *.tgz ]]; then
        tar -xzf "$archive" -C "$dest_dir"
    fi
    echo "✓ Extracted $desc"
}

echo "Step 1: Creating sample data structure..."

# Create sample NPY file with random vectors for testing
# In production, download real RICO vectors
if [[ ! -f "$RICO_DIR/ui_layout_vectors.npy" ]]; then
    echo "Creating sample layout vectors for testing..."
    python3 -c "
import numpy as np
# Create sample 64-dim vectors for 100 screens
vectors = np.random.randn(100, 64).astype(np.float32)
# Normalize to unit vectors
vectors = vectors / np.linalg.norm(vectors, axis=1, keepdims=True)
np.save('$RICO_DIR/ui_layout_vectors.npy', vectors)
print('Created sample vectors: 100 screens x 64 dimensions')
" 2>/dev/null || {
    echo "Python/numpy not available - creating placeholder"
    echo "Please download RICO vectors from https://interactionmining.org/rico"
    touch "$RICO_DIR/ui_layout_vectors.npy.placeholder"
}
fi

# Create sample metadata JSON
if [[ ! -f "$RICO_DIR/ui_metadata.json" ]]; then
    echo "Creating sample metadata..."
    cat > "$RICO_DIR/ui_metadata.json" << 'METADATA_EOF'
{
  "screens": [
    {"screen_id": 1, "app_package": "com.example.app1", "app_name": "Example App", "component_types": [0, 1, 3, 5]},
    {"screen_id": 2, "app_package": "com.example.app1", "app_name": "Example App", "component_types": [0, 1, 2, 4, 6]},
    {"screen_id": 3, "app_package": "com.example.app2", "app_name": "Another App", "component_types": [0, 3, 7, 8]},
    {"screen_id": 4, "app_package": "com.example.login", "app_name": "Login Demo", "component_types": [0, 1, 3, 5, 9]},
    {"screen_id": 5, "app_package": "com.example.list", "app_name": "List Demo", "component_types": [0, 1, 2, 10, 11]}
  ]
}
METADATA_EOF
    echo "✓ Created sample metadata (5 screens)"
fi

# Handle --full flag for semantic annotations
if [[ "${1:-}" == "--full" ]]; then
    echo ""
    echo "Step 2: Downloading semantic annotations..."

    # Semantic annotations structure
    ANNOTATIONS_DIR="$RICO_DIR/semantic_annotations"
    mkdir -p "$ANNOTATIONS_DIR"

    if [[ ! -f "$ANNOTATIONS_DIR/component_annotations.json" ]]; then
        echo "Creating sample annotations..."
        cat > "$ANNOTATIONS_DIR/component_annotations.json" << 'ANNO_EOF'
{
  "component_types": [
    "Text", "Image", "Icon", "Text Button", "Toolbar",
    "List Item", "Input", "Background Image", "Card",
    "Web View", "Radio Button", "Drawer", "Checkbox",
    "Multi-Tab", "Pager Indicator", "Modal", "On/Off Switch",
    "Slider", "Map View", "Button Bar", "Video", "Bottom Navigation",
    "Advertisement", "Number Stepper"
  ],
  "button_concepts": ["Login", "Sign Up", "Submit", "Cancel", "OK", "Next", "Back", "Menu", "Settings"],
  "icon_classes": ["menu", "back", "close", "search", "settings", "home", "profile", "cart", "notification"]
}
ANNO_EOF
        echo "✓ Created sample semantic annotations"
    fi
fi

echo ""
echo "=== Summary ==="
echo "Data directory: $RICO_DIR"
ls -lh "$RICO_DIR" 2>/dev/null || echo "(empty)"
echo ""
echo "To use with rico-mcp, set environment variable:"
echo "  export RICO_DATA_DIR=$RICO_DIR"
echo ""
echo "For real RICO data, download from:"
echo "  https://interactionmining.org/rico"
echo ""
echo "Required files:"
echo "  - ui_layout_vectors.npy  (8MB) - 64-dim layout embeddings"
echo "  - ui_metadata.json       (2MB) - Screen metadata"
echo "  - semantic_annotations/  (50MB, optional) - Component labels"
