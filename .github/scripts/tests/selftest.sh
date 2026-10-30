#!/usr/bin/env bash
# Self-test for the zero-test invariant. Exits non-zero if the guard misbehaves.
set -u
d="$(cd "$(dirname "$0")" && pwd)"; g="$d/../assert_tests_ran.py"; rc=0
expect() { want=$1; shift; python3 "$g" "$@" >/dev/null; got=$?
  if [ "$got" -ne "$want" ]; then echo "FAIL: $* -> $got, want $want"; rc=1; else echo "ok: $* -> $got"; fi; }
expect 0 "$d/ok.log" --min-tests 2
expect 1 "$d/ok.log" --min-tests 3
expect 1 "$d/ok.log" --max-ignored 0
expect 1 "$d/zero.log"
expect 1 "$d/nosummary.log"
expect 1 "$d/failed.log"
# end to end through the wrapper: a command that exits 0 but runs no tests must fail
"$d/../run-tests-guarded.sh" zero 1 -- cat "$d/zero.log" >/dev/null; [ $? -eq 1 ] && echo "ok: wrapper zero-test" || { echo "FAIL wrapper zero"; rc=1; }
"$d/../run-tests-guarded.sh" good 2 -- cat "$d/ok.log" >/dev/null; [ $? -eq 0 ] && echo "ok: wrapper good" || { echo "FAIL wrapper good"; rc=1; }
"$d/../run-tests-guarded.sh" bad 1 -- false >/dev/null; [ $? -ne 0 ] && echo "ok: wrapper cmd failure" || { echo "FAIL wrapper bad"; rc=1; }
exit $rc
