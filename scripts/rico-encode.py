#!/usr/bin/env python3
"""
RICO Screenshot Encoder

Encodes a screenshot into a 64-dimensional layout vector compatible with RICO dataset.
Uses a simple grid-based feature extraction approach.

Usage:
    python rico-encode.py <image_path> [--json]

Output:
    64 comma-separated float values (or JSON array with --json flag)
"""

import sys
import json
import argparse
from pathlib import Path

try:
    from PIL import Image
    import numpy as np
except ImportError:
    print("Error: Required packages not installed. Run: pip install pillow numpy", file=sys.stderr)
    sys.exit(1)


def encode_screenshot(image_path: str, grid_size: int = 8) -> np.ndarray:
    """
    Encode a screenshot into a 64-dimensional layout vector.

    Uses a grid-based approach to extract layout features:
    - Divides image into 8x8 grid (64 cells)
    - For each cell, computes normalized intensity/complexity features
    - Returns unit-normalized 64-dim vector

    Args:
        image_path: Path to the screenshot image
        grid_size: Grid divisions (8x8 = 64 features)

    Returns:
        64-dimensional numpy array (unit normalized)
    """
    # Load and convert to grayscale
    img = Image.open(image_path).convert('L')

    # Resize to fixed size for consistent features
    target_size = (grid_size * 32, grid_size * 32)  # 256x256
    img = img.resize(target_size, Image.Resampling.LANCZOS)

    # Convert to numpy array
    pixels = np.array(img, dtype=np.float32) / 255.0

    # Extract grid features
    features = []
    cell_h = target_size[1] // grid_size
    cell_w = target_size[0] // grid_size

    for row in range(grid_size):
        for col in range(grid_size):
            # Extract cell
            y1, y2 = row * cell_h, (row + 1) * cell_h
            x1, x2 = col * cell_w, (col + 1) * cell_w
            cell = pixels[y1:y2, x1:x2]

            # Compute features for this cell:
            # - Mean intensity (brightness)
            # - Variance (texture complexity)
            # - Edge density (using simple gradient)
            mean_val = np.mean(cell)
            var_val = np.var(cell)

            # Simple edge detection via gradient magnitude
            grad_x = np.abs(np.diff(cell, axis=1)).mean()
            grad_y = np.abs(np.diff(cell, axis=0)).mean()
            edge_val = (grad_x + grad_y) / 2

            # Combine features (weighted)
            # This creates a feature that captures layout structure
            feature = 0.4 * mean_val + 0.3 * np.sqrt(var_val) + 0.3 * edge_val
            features.append(feature)

    # Convert to numpy and normalize to unit vector
    vector = np.array(features, dtype=np.float32)

    # Add small epsilon to avoid division by zero
    norm = np.linalg.norm(vector)
    if norm > 1e-8:
        vector = vector / norm
    else:
        # Fallback: uniform distribution
        vector = np.ones(64, dtype=np.float32) / np.sqrt(64)

    return vector


def main():
    parser = argparse.ArgumentParser(
        description='Encode a screenshot into a RICO-compatible 64-dim layout vector'
    )
    parser.add_argument('image', help='Path to screenshot image')
    parser.add_argument('--json', action='store_true', help='Output as JSON array')
    parser.add_argument('--pretty', action='store_true', help='Pretty print JSON output')

    args = parser.parse_args()

    # Validate input
    image_path = Path(args.image)
    if not image_path.exists():
        print(f"Error: Image not found: {image_path}", file=sys.stderr)
        sys.exit(1)

    try:
        vector = encode_screenshot(str(image_path))

        if args.json:
            output = vector.tolist()
            if args.pretty:
                print(json.dumps(output, indent=2))
            else:
                print(json.dumps(output))
        else:
            # CSV format
            print(','.join(f'{v:.6f}' for v in vector))

    except Exception as e:
        print(f"Error encoding image: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
