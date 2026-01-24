#!/bin/bash
# Run both latest and prior tests and compare results
set -e

cd "$(dirname "$0")"

echo "========================================"
echo "Docker Testnet Comparison Test"
echo "========================================"
echo ""

# Test latest
echo "Step 1/2: Testing LATEST image..."
echo "----------------------------------------"
if ./test-latest.sh; then
    LATEST_RESULT="PASS"
else
    LATEST_RESULT="FAIL"
fi
echo ""

# Test prior
echo "Step 2/2: Testing PRIOR image..."
echo "----------------------------------------"
if ./test-prior.sh; then
    PRIOR_RESULT="PASS"
else
    PRIOR_RESULT="FAIL"
fi
echo ""

# Compare results
echo "========================================"
echo "Comparison Results"
echo "========================================"
echo ""
echo "Latest image: $LATEST_RESULT"
echo "Prior image:  $PRIOR_RESULT"
echo ""

if [ "$LATEST_RESULT" = "PASS" ] && [ "$PRIOR_RESULT" = "PASS" ]; then
    echo "✓ Both images working correctly"
    exit 0
elif [ "$LATEST_RESULT" = "FAIL" ] && [ "$PRIOR_RESULT" = "PASS" ]; then
    echo "✗ REGRESSION DETECTED: Latest image fails, prior image works"
    echo ""
    echo "Next steps:"
    echo "  1. Check logs in ./logs/latest/ for errors"
    echo "  2. Compare with ./logs/prior/"
    echo "  3. Search for DKG, consensus, or feature flag errors"
    exit 1
elif [ "$LATEST_RESULT" = "PASS" ] && [ "$PRIOR_RESULT" = "FAIL" ]; then
    echo "⚠ Latest image works but prior image fails"
    echo "  This is unexpected. Check PRIOR_IMAGE_SHA setting."
    exit 1
else
    echo "✗ Both images failing"
    echo "  Check docker daemon, network, or test configuration"
    exit 1
fi
