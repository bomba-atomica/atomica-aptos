#!/bin/bash
# Test prior working docker image
set -e

cd "$(dirname "$0")"

# Try to detect the prior image SHA
# You can override this by setting PRIOR_IMAGE_SHA environment variable
if [ -z "$PRIOR_IMAGE_SHA" ]; then
    echo "WARNING: PRIOR_IMAGE_SHA not set. Please set it to the last working image."
    echo "Example: export PRIOR_IMAGE_SHA=abc1234"
    echo ""
    echo "Available images:"
    docker images | grep -E "atomica|validator" | head -10
    echo ""
    read -p "Enter prior image tag or SHA (or press Enter to use 'devnet'): " input_sha

    if [ -z "$input_sha" ]; then
        PRIOR_IMAGE_SHA="devnet"
    else
        PRIOR_IMAGE_SHA="$input_sha"
    fi
fi

echo "Testing PRIOR image..."
export IMAGE_NAME="ghcr.io/bomba-atomica/atomica-aptos/validator:${PRIOR_IMAGE_SHA}"
export LOG_DIR="./logs/prior"

# Clean up logs directory
rm -rf "$LOG_DIR"
mkdir -p "$LOG_DIR"

# Try to pull the image
echo "Pulling image: $IMAGE_NAME"
if ! docker pull "$IMAGE_NAME"; then
    echo "WARNING: Failed to pull image. Will use local image if available."
fi

# Run test
bun run test-network.ts

echo "Test completed. Logs saved to $LOG_DIR"
