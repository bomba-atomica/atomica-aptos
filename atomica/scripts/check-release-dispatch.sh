#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR/../.." rev-parse --show-toplevel)"
BINARY_WORKFLOW="${1:-$REPO_ROOT/.github/workflows/build-aptos-binary.yml}"
IMAGE_WORKFLOW="${2:-$REPO_ROOT/.github/workflows/build-validator-image.yml}"

fail() {
  printf 'check-release-dispatch: %s\n' "$1" >&2
  exit 1
}

[[ -f "$BINARY_WORKFLOW" ]] || fail "binary workflow not found: $BINARY_WORKFLOW"
[[ -f "$IMAGE_WORKFLOW" ]] || fail "validator image workflow not found: $IMAGE_WORKFLOW"

build_job="$(awk '
  /^  build-aptos-binaries:$/ { in_job = 1 }
  in_job && /^  [A-Za-z0-9_-]+:$/ && $0 != "  build-aptos-binaries:" { exit }
  in_job { print }
' "$BINARY_WORKFLOW")"
dispatch_job="$(awk '
  /^  dispatch-validator-image:$/ { in_job = 1 }
  in_job && /^  [A-Za-z0-9_-]+:$/ && $0 != "  dispatch-validator-image:" { exit }
  in_job { print }
' "$BINARY_WORKFLOW")"

[[ -n "$dispatch_job" ]] || fail "dispatch-validator-image job is missing"
grep -Fq 'release_tag: ${{ steps.git-commit.outputs.release_tag }}' <<<"$build_job" \
  || fail "binary build job does not expose the release tag output"
grep -Fq 'echo "release_tag=binary-$GIT_SHA"' <<<"$build_job" \
  || fail "git-commit step does not emit the exact binary release tag"
grep -Fq '} >> "$GITHUB_OUTPUT"' <<<"$build_job" \
  || fail "git-commit outputs are not appended to GITHUB_OUTPUT"
grep -Fq 'needs: build-aptos-binaries' <<<"$dispatch_job" \
  || fail "image dispatch job does not wait for the binary release job"
grep -Fq "if: github.ref == 'refs/heads/dev-atomica'" <<<"$dispatch_job" \
  || fail "image dispatch job is not gated on dev-atomica"
grep -Eq '^      actions: write$' <<<"$dispatch_job" \
  || fail "image dispatch job lacks actions: write permission"
grep -Fq 'GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}' <<<"$dispatch_job" \
  || fail "image dispatch job does not use GITHUB_TOKEN for gh"
grep -Fq 'RELEASE_TAG: ${{ needs.build-aptos-binaries.outputs.release_tag }}' <<<"$dispatch_job" \
  || fail "image dispatch job does not pass the published release tag"
grep -Fq 'gh workflow run build-validator-image.yml' <<<"$dispatch_job" \
  || fail "image dispatch job does not dispatch build-validator-image.yml"
grep -Fq -- '--ref dev-atomica' <<<"$dispatch_job" \
  || fail "image dispatch job does not target dev-atomica"
grep -Fq -- '-f release_tag="$RELEASE_TAG"' <<<"$dispatch_job" \
  || fail "image dispatch job does not pass the exact release tag input"

grep -Fq 'CI_CLASS: ignore' "$BINARY_WORKFLOW" \
  || fail "binary workflow lost CI_CLASS: ignore"
grep -Fq 'CI_CLASS: ignore' "$IMAGE_WORKFLOW" \
  || fail "validator image workflow lost CI_CLASS: ignore"
grep -Eq '^  workflow_dispatch:$' "$IMAGE_WORKFLOW" \
  || fail "validator image workflow lost workflow_dispatch"
grep -Eq '^      release_tag:$' "$IMAGE_WORKFLOW" \
  || fail "validator image workflow lost its release_tag input"

printf 'check-release-dispatch: release dispatch wiring is valid\n'
