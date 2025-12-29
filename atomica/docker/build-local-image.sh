#!/bin/bash
# Build Atomica Aptos validator image locally from source
#
# This script builds the validator Docker image from the local atomica-aptos source
# using sccache for fast incremental builds. The sccache data is persisted in a
# Docker volume so subsequent builds are much faster.
#
# Usage:
#   ./build-local-image.sh [OPTIONS]
#
# Options:
#   --profile <release|debug>   Build profile (default: release)
#   --features <features>       Cargo features (default: testing)
#   --tag <tag>                 Image tag (default: local)
#   --no-cache                  Disable Docker build cache
#   --help                      Show this help message
#
# Environment:
#   DOCKER_BUILDKIT=1           Required for cache mounts
#
# Example:
#   # Fast incremental build (uses BuildKit cache)
#   ./build-local-image.sh
#
#   # Clean build (no cache)
#   ./build-local-image.sh --no-cache
#
#   # Debug build
#   ./build-local-image.sh --profile debug

set -e

# Default values
PROFILE="release"
FEATURES="testing"
IMAGE_TAG="local"
NO_CACHE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --profile)
            PROFILE="$2"
            shift 2
            ;;
        --features)
            FEATURES="$2"
            shift 2
            ;;
        --tag)
            IMAGE_TAG="$2"
            shift 2
            ;;
        --no-cache)
            NO_CACHE="--no-cache"
            shift
            ;;
        --help)
            grep "^#" "$0" | sed 's/^# //g' | sed 's/^#//g'
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Run with --help for usage information"
            exit 1
            ;;
    esac
done

# Validate profile
if [[ "$PROFILE" != "release" && "$PROFILE" != "debug" ]]; then
    echo "ERROR: Invalid profile: $PROFILE (must be 'release' or 'debug')"
    exit 1
fi

# Find atomica-aptos source directory (repository root)
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
ATOMICA_APTOS_DIR="$SCRIPT_DIR/../.."

if [[ ! -d "$ATOMICA_APTOS_DIR" ]]; then
    echo "ERROR: atomica-aptos repository root not found"
    echo "This script should be run from: atomica-aptos/atomica/docker/"
    exit 1
fi

ATOMICA_APTOS_DIR=$(cd "$ATOMICA_APTOS_DIR" && pwd)
echo "Using atomica-aptos source: $ATOMICA_APTOS_DIR"

# Check for Dockerfile
DOCKERFILE="$SCRIPT_DIR/Dockerfile.local"
if [[ ! -f "$DOCKERFILE" ]]; then
    echo "ERROR: Dockerfile not found: $DOCKERFILE"
    exit 1
fi

# Get git commit hash
GIT_SHA=$(cd "$ATOMICA_APTOS_DIR" && git rev-parse --short HEAD 2>/dev/null || echo "unknown")
BUILD_DATE=$(date -u +'%Y-%m-%dT%H:%M:%SZ')

echo "=== Building Atomica Aptos Validator Image (Local) ==="
echo "  Source:   $ATOMICA_APTOS_DIR"
echo "  Git SHA:  $GIT_SHA"
echo "  Profile:  $PROFILE"
echo "  Features: $FEATURES"
echo "  Tag:      atomica-validator:$IMAGE_TAG"
echo ""

# Ensure BuildKit is enabled
export DOCKER_BUILDKIT=1

# Build the image
echo "Building image (this may take a while on first build)..."
echo "Subsequent builds will be faster thanks to BuildKit cache!"
echo ""

docker buildx build \
    -f "$DOCKERFILE" \
    -t "atomica-validator:$IMAGE_TAG" \
    --build-arg GIT_SHA="$GIT_SHA" \
    --build-arg BUILD_DATE="$BUILD_DATE" \
    --build-arg PROFILE="$PROFILE" \
    --build-arg FEATURES="$FEATURES" \
    --progress=plain \
    $NO_CACHE \
    "$ATOMICA_APTOS_DIR"

echo ""
echo "✓ Build complete!"
echo "  Image: atomica-validator:$IMAGE_TAG"
echo ""
echo "To use this image:"
echo "  docker run -it atomica-validator:$IMAGE_TAG"
echo ""
echo "Or with docker-testnet:"
echo "  IMAGE_NAME=atomica-validator:$IMAGE_TAG docker compose up -d"
