#!/usr/bin/env python3
"""Fail when a cargo test log shows that too few tests actually ran.

Policy (strict test coverage): exit code 0 from `cargo test` does not prove
anything was tested. A log with no `test result:` line, or with zero executed
tests, or fewer executed tests than --min-tests, is a failure. Ignored
(skipped) tests are never silent: each one is printed as a workflow warning.

Usage: assert_tests_ran.py LOG --min-tests N [--max-ignored M] [--label NAME]
"""
import argparse
import os
import re
import sys

RESULT_RE = re.compile(
    r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;"
    r"(?: (\d+) measured;)?(?: (\d+) filtered out)?"
)
IGNORED_RE = re.compile(r"^test (\S+) \.\.\. ignored(?:, (.*))?$")


def summarize(text):
    passed = failed = ignored = 0
    binaries = 0
    for m in RESULT_RE.finditer(text):
        binaries += 1
        passed += int(m.group(1))
        failed += int(m.group(2))
        ignored += int(m.group(3))
    ignored_names = []
    for line in text.splitlines():
        m = IGNORED_RE.match(line.strip())
        if m:
            ignored_names.append((m.group(1), m.group(2) or ""))
    return {
        "binaries": binaries,
        "passed": passed,
        "failed": failed,
        "ignored": ignored,
        "executed": passed + failed,
        "collected": passed + failed + ignored,
        "ignored_names": ignored_names,
    }


def check(s, min_tests, max_ignored):
    errors = []
    if s["binaries"] == 0:
        errors.append("no `test result:` line found: zero tests were collected")
    if s["executed"] == 0:
        errors.append("zero tests executed")
    if s["executed"] < min_tests:
        errors.append(f"executed {s['executed']} tests, below required minimum {min_tests}")
    if s["failed"] > 0:
        errors.append(f"{s['failed']} tests failed")
    if max_ignored is not None and s["ignored"] > max_ignored:
        errors.append(f"{s['ignored']} ignored tests, above allowed maximum {max_ignored}")
    return errors


def main(argv=None):
    p = argparse.ArgumentParser()
    p.add_argument("log")
    p.add_argument("--min-tests", type=int, default=1)
    p.add_argument("--max-ignored", type=int, default=None)
    p.add_argument("--label", default="tests")
    a = p.parse_args(argv)
    with open(a.log, encoding="utf-8", errors="replace") as f:
        s = summarize(f.read())
    for name, reason in s["ignored_names"]:
        print(f"::warning title=skipped test ({a.label})::{name} ignored {reason}".rstrip())
    line = (
        f"[{a.label}] collected={s['collected']} executed={s['executed']} "
        f"skipped={s['ignored']} failed={s['failed']} test-binaries={s['binaries']}"
    )
    print(line)
    summary = os.environ.get("GITHUB_STEP_SUMMARY")
    if summary:
        with open(summary, "a", encoding="utf-8") as f:
            f.write(line + "\n")
    errors = check(s, a.min_tests, a.max_ignored)
    for e in errors:
        print(f"::error title=test coverage invariant ({a.label})::{e}")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
