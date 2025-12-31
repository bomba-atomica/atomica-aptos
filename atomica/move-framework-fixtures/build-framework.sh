#!/bin/bash
# Build Aptos framework release bundle with content-addressable caching
#
# Usage:
#   ./build-framework.sh [FRAMEWORK_PATH] [OUTPUT_DIR]
#
# Output:
#   - head-{HASH}.mrb  (the actual build artifact)
#   - head.mrb         (symlink to latest build)
#
# Caching:
#   If head-{HASH}.mrb already exists, the build is skipped.
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRAMEWORK_PATH="${1:-}"
OUTPUT_DIR="${2:-$SCRIPT_DIR}"
FRAMEWORK_BIN="${APTOS_FRAMEWORK_BIN:-aptos-framework}"

# Auto-detect framework path
if [[ -z "$FRAMEWORK_PATH" ]]; then
    CANDIDATE="$SCRIPT_DIR/../../aptos-move/framework"
    if [[ -d "$CANDIDATE" ]]; then
        FRAMEWORK_PATH="$(cd "$CANDIDATE" && pwd)"
    else
        echo "error: FRAMEWORK_PATH not provided and could not auto-detect" >&2
        exit 1
    fi
fi

FRAMEWORK_PATH="$(cd "$FRAMEWORK_PATH" && pwd)"
mkdir -p "$OUTPUT_DIR"
OUTPUT_DIR="$(cd "$OUTPUT_DIR" && pwd)"

# Packages to include (matches ReleaseTarget::Head)
PACKAGES=(
    "move-stdlib"
    "aptos-stdlib"
    "aptos-framework"
    "aptos-token"
    "aptos-token-objects"
    "aptos-experimental"
)

# Compute fast hash of framework directory
# Uses ls -lR to capture file names, sizes, and timestamps
# Fast (<200ms) and works in any directory (git or non-git)
compute_hash() {
    local framework_dir="$1"
    ls -lR "$framework_dir" | shasum -a 256 | cut -c1-12
}

# Validate binary
if ! command -v "$FRAMEWORK_BIN" &>/dev/null; then
    echo "error: '$FRAMEWORK_BIN' not found in PATH" >&2
    exit 1
fi

# Validate framework structure
for pkg_dir in "${PACKAGES[@]}"; do
    if [[ ! -d "$FRAMEWORK_PATH/$pkg_dir" ]]; then
        echo "error: package not found: $FRAMEWORK_PATH/$pkg_dir" >&2
        exit 1
    fi
done

# Compute hash
HASH=$(compute_hash "$FRAMEWORK_PATH")
VERSIONED_OUTPUT="$OUTPUT_DIR/head-${HASH}.mrb"
SYMLINK_PATH="$OUTPUT_DIR/head.mrb"

echo "Framework: $FRAMEWORK_PATH"
echo "Hash: $HASH"

# Check cache
if [[ -f "$VERSIONED_OUTPUT" ]]; then
    echo "Cache hit: $VERSIONED_OUTPUT"
    # Update symlink if needed
    if [[ ! -L "$SYMLINK_PATH" ]] || [[ "$(readlink "$SYMLINK_PATH")" != "head-${HASH}.mrb" ]]; then
        ln -sf "head-${HASH}.mrb" "$SYMLINK_PATH"
        echo "Updated symlink: head.mrb -> head-${HASH}.mrb"
    fi
    exit 0
fi

echo "Cache miss, building..."

# Build arguments
CMD_ARGS=("--skip-attribute-checks")
for pkg_dir in "${PACKAGES[@]}"; do
    CMD_ARGS+=("--packages" "$FRAMEWORK_PATH/$pkg_dir")
    CMD_ARGS+=("--rust-bindings=")
    CMD_ARGS+=("--package-use-latest-language=false")
done
CMD_ARGS+=("--output" "$VERSIONED_OUTPUT")

# Build
"$FRAMEWORK_BIN" custom "${CMD_ARGS[@]}"

# Verify
if [[ ! -f "$VERSIONED_OUTPUT" ]]; then
    echo "error: build failed, output not found" >&2
    exit 1
fi

# Create symlink
ln -sf "head-${HASH}.mrb" "$SYMLINK_PATH"

SIZE=$(du -h "$VERSIONED_OUTPUT" | cut -f1)
echo "Built: $VERSIONED_OUTPUT ($SIZE)"
echo "Symlink: head.mrb -> head-${HASH}.mrb"
