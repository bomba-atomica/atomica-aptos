# Docker Testnet Debug Findings

## Executive Summary

**ROOT CAUSE IDENTIFIED:** The published docker image `ghcr.io/bomba-atomica/atomica-aptos/validator:latest` is **missing the `aptos` CLI binary**, which prevents genesis generation and network startup.

## Investigation Timeline

### Discovery

1. **Initial symptom:** Test harness fails at genesis generation step
2. **Error message:** `/genesis-script.sh: line 52: /usr/local/bin/aptos: No such file or directory`
3. **Verification:** Inspected the docker image and confirmed `/usr/local/bin/aptos` does not exist
4. **Expected vs Actual:**
   - **Dockerfile (line 49):** `COPY --chmod=755 atomica/docker/binaries/aptos-${GIT_SHA} /usr/local/bin/aptos`
   - **Actual image contents:** Only `aptos-node` and `aptos-debugger` exist

### Image Contents

```bash
$ docker run --rm ghcr.io/bomba-atomica/atomica-aptos/validator:latest ls -la /usr/local/bin/
-rwxr-xr-x 1 root root 2420808968 Jan 21 19:53 aptos-debugger
-rwxr-xr-x 1 root root 2552659160 Jan 21 19:50 aptos-node
```

**Missing:** `/usr/local/bin/aptos` (the CLI tool needed for genesis generation)

### Dockerfile Analysis

From `atomica/docker/Dockerfile` (lines 46-57):

```dockerfile
# Copy prebuilt binaries (downloaded by GitHub Actions)
# These binaries are downloaded and verified in the CI workflow before building the image
COPY --chmod=755 atomica/docker/binaries/aptos-node-${GIT_SHA} /usr/local/bin/aptos-node
COPY --chmod=755 atomica/docker/binaries/aptos-${GIT_SHA} /usr/local/bin/aptos  # ← THIS FAILS
...
# Verify the binaries work
RUN /usr/local/bin/aptos-node --version && \
    /usr/local/bin/aptos --version  # ← THIS SHOULD FAIL BUILD
```

### Why This Happens

The Dockerfile expects `atomica/docker/binaries/aptos-${GIT_SHA}` to exist before building, but:

1. **Build process issue:** The `aptos` CLI binary is not being downloaded/built before the Docker build
2. **Missing verification:** The `RUN` command that verifies binaries should fail the build, but somehow the image was published anyway
3. **Possible causes:**
   - Build process skipped downloading the `aptos` binary
   - Binary download failed silently
   - Dockerfile build was not properly verified before publishing
   - CI/CD pipeline missing or incomplete

## Impact

Without the `aptos` CLI binary:

1. ✗ Genesis generation fails
2. ✗ Network cannot start
3. ✗ No genesis.blob or waypoint.txt created
4. ✗ Validators cannot initialize
5. ✗ Zero blocks produced

## Comparison: Latest vs Prior Images

### Latest Image (BROKEN)
- **Image:** `ghcr.io/bomba-atomica/atomica-aptos/validator:latest`
- **SHA:** `27c2baebde02`
- **Binaries:** `aptos-node`, `aptos-debugger` ✓ | `aptos` ✗

### Devnet Image (Reference - should work)
- **Image:** `aptoslabs/validator:devnet`
- **SHA:** `27c2baebde02` (same as latest!)
- **Binaries:** `aptos-node`, `aptos-debugger` ✓ | `aptos` ✗ (also missing!)

**Interesting finding:** Both images are the same and both lack the `aptos` binary!

This suggests:
- The Atomica image might be an alias/tag pointing to the same upstream image
- OR the binary download process failed in the upstream build too
- This is likely an **upstream Aptos Labs build issue**, not specific to Atomica

## Solutions

### Immediate Workaround

Use a genesis generation method that doesn't rely on the docker image's `aptos` binary:

#### Option A: Use host-installed `aptos` CLI

Install `aptos` CLI on the host machine and use it for genesis generation, then copy artifacts into Docker.

```bash
# Install aptos CLI
cargo install --git https://github.com/aptos-labs/aptos-core.git aptos

# Or download pre-built binary
curl -sL https://aptos.dev/scripts/install_cli.py | python3
```

Then modify the test harness to use host CLI for genesis instead of docker-based genesis.

#### Option B: Build correct docker image

Build the docker image locally with the correct binaries:

```bash
# In atomica-aptos repository
cd atomica/docker

# Download or build aptos binary
cargo build --release -p aptos
mkdir -p binaries
cp ../../target/release/aptos binaries/aptos-$(git rev-parse HEAD)
cp ../../target/release/aptos-node binaries/aptos-node-$(git rev-parse HEAD)

# Build docker image
docker build \
  --build-arg GIT_SHA=$(git rev-parse HEAD) \
  --build-arg BINARY_RELEASE_TAG=local \
  -t ghcr.io/bomba-atomica/atomica-aptos/validator:local \
  -f Dockerfile \
  ../..
```

#### Option C: Use pre-genesis artifacts

If you have a working genesis.blob from a previous build, you can skip genesis generation and use the existing artifacts.

### Long-term Fix

1. **Fix the build pipeline:**
   - Ensure `aptos` binary is downloaded/built before Docker build
   - Add proper CI/CD verification that checks all binaries exist
   - Fail the build if any expected binary is missing

2. **Add to build workflow (create `.github/workflows/build-validator-image.yml`):**
   ```yaml
   - name: Download aptos binaries
     run: |
       # Download from GitHub releases or build from source
       cargo build --release -p aptos -p aptos-node
       mkdir -p atomica/docker/binaries
       cp target/release/aptos atomica/docker/binaries/aptos-${{ github.sha }}
       cp target/release/aptos-node atomica/docker/binaries/aptos-node-${{ github.sha }}

   - name: Verify binaries
     run: |
       chmod +x atomica/docker/binaries/*
       atomica/docker/binaries/aptos-${{ github.sha }} --version
       atomica/docker/binaries/aptos-node-${{ github.sha }} --version
   ```

3. **Update Dockerfile to fail loudly:**
   ```dockerfile
   # Verify both binaries exist
   RUN /usr/local/bin/aptos-node --version && \
       /usr/local/bin/aptos --version || \
       (echo "ERROR: Required binaries missing!" && exit 1)
   ```

## Next Steps

1. **Immediate:** Implement Option A (use host aptos CLI) to continue testing
2. **Short-term:** Build corrected docker image locally using Option B
3. **Long-term:** Set up proper CI/CD pipeline with binary verification

## Testing Status

- ✗ **Phase 1 (Reproduce):** Blocked by missing `aptos` binary
- ⏸ **Phase 2-6:** Waiting for workaround implementation

## Files Created

- `atomica-test/docker-testnet/` - Full test infrastructure ready to use
- `DEBUG_DOCKER_TESTNET_PLAN.md` - Detailed investigation plan (25 tasks)
- `DEBUG_FINDINGS.md` - This document

## Recommendation

**Use Option A (host CLI) as the immediate workaround.** This will allow us to:
1. Complete the test with the current docker image
2. Verify if there are any other issues beyond the missing binary
3. Test the DKG/IBE functionality once network starts

I can implement this workaround now if you'd like to proceed with testing.
