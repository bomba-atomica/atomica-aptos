#!/usr/bin/env bash
# Run a cargo test command, keep its exit status, then enforce the strict
# test-coverage invariant (see assert_tests_ran.py).
# Usage: run-tests-guarded.sh LABEL MIN_TESTS -- cargo test ...
set -uo pipefail
label="$1"; min="$2"; shift 2
[ "${1:-}" = "--" ] && shift
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
log="$(mktemp)"
"$@" 2>&1 | tee "$log"
status=${PIPESTATUS[0]}
python3 "$here/assert_tests_ran.py" "$log" --min-tests "$min" --label "$label"
guard=$?
rm -f "$log"
if [ "$status" -ne 0 ]; then echo "::error::$label: command exited $status"; exit "$status"; fi
exit "$guard"
