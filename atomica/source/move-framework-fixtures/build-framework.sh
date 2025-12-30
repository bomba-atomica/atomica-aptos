#!/bin/bash
# Build Aptos framework fixtures with hash preservation
set -e

# Configuration
OUTPUT_DIR="${1:-./atomica/move-framework-fixtures}"
FRAMEWORK_TARGET="${2:-head}"
FRAMEWORK_BIN="${APTOS_FRAMEWORK_BIN:-aptos-framework}"

# Ensure output directory exists
mkdir -p "$OUTPUT_DIR"

echo "🔨 Building Aptos framework fixtures..."
echo "   Target: $FRAMEWORK_TARGET"
echo "   Output: $OUTPUT_DIR"
echo "   Binary: $FRAMEWORK_BIN"

# Get current git hash for source code
if git rev-parse --git-dir > /dev/null 2>&1; then
    SOURCE_HASH=$(git rev-parse HEAD)
    SOURCE_SHORT=$(git rev-parse --short HEAD)
    echo "   Source: $SOURCE_SHORT ($SOURCE_HASH)"
else
    SOURCE_HASH="unknown"
    SOURCE_SHORT="unknown"
    echo "   Source: not in git repository"
fi

# Build the framework
echo "🚀 Running: $FRAMEWORK_BIN release --target $FRAMEWORK_TARGET"
if $FRAMEWORK_BIN release --target "$FRAMEWORK_TARGET" --without-source-code > "$OUTPUT_DIR/head.mrb"; then
    echo "✅ Framework built successfully"
else
    echo "❌ Framework build failed"
    exit 1
fi

# Create metadata file with hash information
cat > "$OUTPUT_DIR/build-info.txt" << EOF
# Aptos Framework Build Info
# Built on: $(date -Iseconds)
# Target: $FRAMEWORK_TARGET
# Source hash: $SOURCE_HASH
# Source short: $SOURCE_SHORT
# Build command: $FRAMEWORK_BIN release --target $FRAMEWORK_TARGET
EOF

# Verify the output
if [ -f "$OUTPUT_DIR/head.mrb" ]; then
    SIZE=$(ls -lh "$OUTPUT_DIR/head.mrb" | awk '{print $5}')
    echo "📦 Framework file created: $OUTPUT_DIR/head.mrb ($SIZE)"
    echo "📝 Build info saved: $OUTPUT_DIR/build-info.txt"
else
    echo "❌ Framework file not found after build"
    exit 1
fi

echo "✅ Framework fixtures build complete"