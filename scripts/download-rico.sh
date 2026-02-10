#!/bin/bash
# Download RICO dataset files for rico-mcp
#
# The RICO dataset is hosted on Google Cloud Storage.
# This script downloads the required files for similarity search.
#
# Usage: ./scripts/download-rico.sh [--sample|--full]
#   --sample: Create sample data for testing (default if no network)
#   --full:   Download full RICO dataset (~600MB with annotations)
#   (no flag): Download vectors and metadata only (~80MB)

set -euo pipefail

RICO_DIR="${RICO_DATA_DIR:-${HOME}/.rico-mcp/data}"

# RICO dataset URLs from Google Cloud Storage
BASE_URL="https://storage.googleapis.com/crowdstf-rico-uiuc-4540"

# File URLs
LAYOUT_VECTORS_URL="${BASE_URL}/rico_dataset_v0.1/ui_layout_vectors.zip"
SEMANTIC_ANNOTATIONS_URL="${BASE_URL}/rico_dataset_v0.1/semantic_annotations.zip"

echo "=== RICO Dataset Downloader ==="
echo "Data directory: $RICO_DIR"
echo ""

# Create directories
mkdir -p "$RICO_DIR"

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
        curl -L --progress-bar -o "$dest" "$url" || return 1
    elif command -v wget &> /dev/null; then
        wget --show-progress -q -O "$dest" "$url" || return 1
    else
        echo "Error: Neither curl nor wget found"
        exit 1
    fi
    echo "✓ Downloaded $desc"
}

# Function to create sample data
create_sample_data() {
    echo "Creating sample data for testing..."

    # Create sample NPY file with random vectors
    if [[ ! -f "$RICO_DIR/ui_layout_vectors.npy" ]]; then
        echo "Creating sample layout vectors..."
        python3 -c "
import numpy as np
# Create sample 64-dim vectors for 100 screens
vectors = np.random.randn(100, 64).astype(np.float32)
# Normalize to unit vectors
vectors = vectors / np.linalg.norm(vectors, axis=1, keepdims=True)
np.save('$RICO_DIR/ui_layout_vectors.npy', vectors)
print('Created sample vectors: 100 screens x 64 dimensions')
" 2>/dev/null || {
            echo "Error: Python/numpy not available"
            exit 1
        }
    fi

    # Create sample metadata JSON
    if [[ ! -f "$RICO_DIR/ui_metadata.json" ]]; then
        echo "Creating sample metadata..."
        python3 -c "
import json

apps = [
    ('com.example.social', 'Social App'),
    ('com.example.shopping', 'Shopping App'),
    ('com.example.news', 'News Reader'),
    ('com.example.music', 'Music Player'),
    ('com.example.fitness', 'Fitness Tracker'),
    ('com.example.banking', 'Mobile Banking'),
    ('com.example.travel', 'Travel Planner'),
    ('com.example.food', 'Food Delivery'),
    ('com.example.messaging', 'Messenger'),
    ('com.example.weather', 'Weather App'),
]

screens = []
for i in range(100):
    app = apps[i % len(apps)]
    screens.append({
        'screen_id': i,
        'app_package': app[0],
        'app_name': app[1]
    })

with open('$RICO_DIR/ui_metadata.json', 'w') as f:
    json.dump(screens, f, indent=2)

print('Created sample metadata: 100 screens')
"
    fi

    # Create sample annotations
    mkdir -p "$RICO_DIR/semantic_annotations"
    echo "✓ Sample data created"
}

# Function to download and process real RICO data
download_rico_data() {
    local include_annotations="${1:-false}"

    echo "Step 1: Downloading UI layout vectors..."

    local vectors_zip="$RICO_DIR/ui_layout_vectors.zip"

    if [[ ! -f "$RICO_DIR/ui_layout_vectors.npy" ]]; then
        if download_file "$LAYOUT_VECTORS_URL" "$vectors_zip" "UI layout vectors (~17MB)"; then
            echo "→ Extracting vectors..."
            unzip -q -o "$vectors_zip" -d "$RICO_DIR"
            rm -f "$vectors_zip"

            # Handle nested directory structure from zip
            if [[ -f "$RICO_DIR/ui_layout_vectors/ui_vectors.npy" ]]; then
                mv "$RICO_DIR/ui_layout_vectors/ui_vectors.npy" "$RICO_DIR/ui_layout_vectors.npy"
                rm -rf "$RICO_DIR/ui_layout_vectors" "$RICO_DIR/__MACOSX"
            fi
            echo "✓ Extracted layout vectors"
        else
            echo "⚠ Download failed, falling back to sample data"
            create_sample_data
            return
        fi
    else
        echo "✓ Layout vectors already exist"
    fi

    echo ""
    echo "Step 2: Creating metadata from RICO dataset..."

    # The RICO dataset doesn't have a single metadata JSON file.
    # We need to create one from the unique_uis data or use app info.
    # For now, create metadata that matches the vector count.
    if [[ ! -f "$RICO_DIR/ui_metadata.json" ]]; then
        echo "Generating metadata for vectors..."
        python3 << 'METADATA_SCRIPT'
import numpy as np
import json
import os

rico_dir = os.environ.get('RICO_DIR', os.path.expanduser('~/.rico-mcp/data'))
vectors_path = os.path.join(rico_dir, 'ui_layout_vectors.npy')
metadata_path = os.path.join(rico_dir, 'ui_metadata.json')

# Load vectors to get count
vectors = np.load(vectors_path)
num_screens = vectors.shape[0]
print(f"Found {num_screens} vectors")

# Generate basic metadata
# In a full setup, this would parse the RICO hierarchy files
screens = []
for i in range(num_screens):
    screens.append({
        'screen_id': i,
        'app_package': f'rico.app.{i // 100}',
        'app_name': f'RICO App {i // 100}'
    })

with open(metadata_path, 'w') as f:
    json.dump(screens, f)

print(f"Created metadata for {num_screens} screens")
METADATA_SCRIPT
        echo "✓ Generated metadata"
    else
        echo "✓ Metadata already exists"
    fi

    if [[ "$include_annotations" == "true" ]]; then
        echo ""
        echo "Step 3: Downloading semantic annotations..."

        local annotations_zip="$RICO_DIR/semantic_annotations.zip"

        if [[ ! -d "$RICO_DIR/semantic_annotations" ]] || [[ -z "$(ls -A "$RICO_DIR/semantic_annotations" 2>/dev/null)" ]]; then
            if download_file "$SEMANTIC_ANNOTATIONS_URL" "$annotations_zip" "Semantic annotations (~500MB)"; then
                echo "→ Extracting annotations..."
                unzip -q -o "$annotations_zip" -d "$RICO_DIR"
                rm -f "$annotations_zip"
                echo "✓ Extracted semantic annotations"
            else
                echo "⚠ Annotations download failed, continuing without them"
            fi
        else
            echo "✓ Semantic annotations already exist"
        fi
    fi
}

# Parse arguments
MODE="download"
INCLUDE_ANNOTATIONS="false"

while [[ $# -gt 0 ]]; do
    case $1 in
        --sample)
            MODE="sample"
            shift
            ;;
        --full)
            INCLUDE_ANNOTATIONS="true"
            shift
            ;;
        --help|-h)
            echo "Usage: $0 [--sample|--full]"
            echo ""
            echo "Options:"
            echo "  --sample  Create sample data for testing (100 screens)"
            echo "  --full    Download full dataset with annotations (~600MB)"
            echo "  (default) Download vectors and metadata only (~80MB)"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Execute based on mode
if [[ "$MODE" == "sample" ]]; then
    create_sample_data
else
    export RICO_DIR
    download_rico_data "$INCLUDE_ANNOTATIONS"
fi

echo ""
echo "=== Summary ==="
echo "Data directory: $RICO_DIR"
ls -lh "$RICO_DIR" 2>/dev/null || echo "(empty)"
echo ""

# Verify data
if [[ -f "$RICO_DIR/ui_layout_vectors.npy" ]]; then
    python3 -c "
import numpy as np
import json
import os

rico_dir = '$RICO_DIR'
vectors = np.load(os.path.join(rico_dir, 'ui_layout_vectors.npy'))
print(f'Vectors: {vectors.shape[0]} screens x {vectors.shape[1]} dimensions')

metadata_path = os.path.join(rico_dir, 'ui_metadata.json')
if os.path.exists(metadata_path):
    with open(metadata_path) as f:
        metadata = json.load(f)
    print(f'Metadata: {len(metadata)} entries')

    if vectors.shape[0] != len(metadata):
        print(f'⚠ WARNING: Vector count ({vectors.shape[0]}) != metadata count ({len(metadata)})')
" 2>/dev/null || echo "Could not verify data (Python/numpy not available)"
fi

echo ""
echo "To use with rico-mcp, set environment variable:"
echo "  export RICO_DATA_DIR=$RICO_DIR"
