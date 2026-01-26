#!/bin/bash
# Test latest docker image
set -e

cd "$(dirname "$0")"

echo "Testing LATEST image..."
export IMAGE_NAME="ghcr.io/bomba-atomica/atomica-aptos/validator:latest"
export LOG_DIR="./logs/latest"

# Clean up logs directory
rm -rf "$LOG_DIR"
mkdir -p "$LOG_DIR"

# Run test
bun run test-network.ts

echo "Test completed. Logs saved to $LOG_DIR"
