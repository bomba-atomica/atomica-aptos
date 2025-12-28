# Atomica Aptos Docker Images

This directory contains Docker configurations for building Atomica Aptos validator node images.

## Build Strategy

We use a two-stage build strategy to minimize resource usage and improve build times:

### 1. Binary Build Workflow (`build-aptos-binary.yml`)

This workflow builds the `aptos` binary directly on GitHub Actions runners and publishes it as a GitHub release.

**Benefits:**
- Leverages `Swatinem/rust-cache` action for efficient Rust dependency caching
- Runs on bare GitHub Actions runners without Docker overhead
- Creates reusable binary artifacts via GitHub releases
- Faster incremental builds with shared cache across workflow runs
- Chained job architecture ensures cache is saved even on partial failures

**Chained Job Architecture:**

The workflow uses three sequential jobs to build dependencies incrementally:

1. **Job 1: `build-core-dependencies`** (150 min timeout)
   - Builds `move-core-types` package
   - Builds `aptos-framework` package
   - Saves cache with key `aptos-core-deps`
   - Cache is saved even if job fails (`cache-on-failure: true`)

2. **Job 2: `build-aptos-node`** (150 min timeout)
   - Depends on Job 1
   - Restores cache from Job 1
   - Builds `aptos-node` binary with testing features
   - Saves incremental cache with key `aptos-node`
   - Cache is saved even if job fails

3. **Job 3: `build-aptos-cli`** (150 min timeout)
   - Depends on Job 2
   - Restores cache from Job 2
   - Builds `aptos` CLI binary
   - Saves final cache with key `aptos-cli`
   - Publishes binary to GitHub releases

**Why chained jobs?**
- Each job builds on the previous job's cache
- If a job fails, earlier caches are still saved
- Better visibility into which build stage is failing
- More efficient resource usage per job
- Allows for easier debugging and restarting from failed stage

**Triggers:**
- Push to `main`, `dev-atomica`, or `docker-testnet-org-refactor` branches
- Pull requests to those branches
- Manual workflow dispatch

**Outputs:**
- GitHub release with tag `binary-<short-sha>`
- Binary artifact: `aptos-<short-sha>`
- SHA256 checksum: `aptos-<short-sha>.sha256`

### 2. Docker Image Build Workflow (`build-validator-image-v2.yml`)

This workflow builds lightweight Docker images that download prebuilt binaries from GitHub releases.

**Benefits:**
- Minimal build time (no Rust compilation in Docker)
- Reduced resource usage and disk space requirements
- Faster Docker builds focused only on OS setup and binary installation
- Automatic dependency on binary build workflow

**Triggers:**
- Automatically after successful binary build workflow completion
- Push to branches (for Dockerfile changes only)
- Pull requests (for Dockerfile changes only)
- Manual workflow dispatch

**Process:**
1. Verifies that a prebuilt binary exists for the target commit
2. Downloads binary from GitHub releases
3. Builds minimal Docker image with runtime dependencies
4. Publishes to GitHub Container Registry

## Dockerfiles

### `Dockerfile` (Legacy)

Multi-stage build that compiles Rust code inside Docker. This approach:
- Has high resource requirements
- Takes longer to build
- Is more prone to OOM errors on CI
- **Status:** Kept for reference but superseded by the new approach

### `Dockerfile.prebuilt` (Current)

Single-stage build that downloads prebuilt binaries. This approach:
- Has minimal resource requirements
- Builds quickly (typically < 5 minutes)
- Downloads verified binaries from GitHub releases
- **Status:** Active, used by `build-validator-image-v2.yml`

## Build Arguments

### `Dockerfile.prebuilt`

| Argument | Required | Description | Example |
|----------|----------|-------------|---------|
| `GIT_SHA` | Yes | Short Git commit hash | `abc1234` |
| `BINARY_RELEASE_TAG` | Yes | GitHub release tag | `binary-abc1234` |
| `GITHUB_REPOSITORY` | Yes | Repository path | `owner/repo` |
| `BUILD_DATE` | No | Build timestamp | `2024-01-01T00:00:00Z` |

## Manual Builds

### Building Binary Locally

```bash
# Build the aptos binary
cargo build --release --package aptos

# Verify the binary
./target/release/aptos --version
```

### Building Docker Image Locally

First, ensure you have a binary built or downloaded:

```bash
# Option 1: Build locally
cargo build --release --package aptos

# Option 2: Download from GitHub releases
GIT_SHA="abc1234"
RELEASE_TAG="binary-${GIT_SHA}"
curl -L -o aptos \
  "https://github.com/OWNER/REPO/releases/download/${RELEASE_TAG}/aptos-${GIT_SHA}"
chmod +x aptos
```

Then build the Docker image:

```bash
# Using prebuilt binary from GitHub releases
docker build \
  -f atomica/docker/Dockerfile.prebuilt \
  --build-arg GIT_SHA=abc1234 \
  --build-arg BINARY_RELEASE_TAG=binary-abc1234 \
  --build-arg GITHUB_REPOSITORY=owner/repo \
  -t atomica-aptos:latest \
  .
```

## Image Registry

Images are published to GitHub Container Registry:

```
ghcr.io/<owner>/<repo>/validator:<tag>
```

### Tag Strategy

| Tag Format | Description | Example |
|------------|-------------|---------|
| `<sha>-<dockerfile-hash>` | Unique build identifier | `abc1234-def5678` |
| `<sha>` | Git commit | `abc1234` |
| `latest` | Latest build from default branch | `latest` |
| `<branch>` | Latest build from branch | `dev-atomica` |
| `pr-<number>` | Pull request build | `pr-123` |

## Troubleshooting

### Binary Not Found Error

If the Docker image workflow fails with "Binary release not found":

1. Check that the binary build workflow completed successfully
2. Verify the release exists: `https://github.com/<owner>/<repo>/releases/tag/binary-<sha>`
3. Manually trigger the binary build workflow if needed

### Cache Issues

If builds are slow or cache isn't working:

1. Check the `Swatinem/rust-cache` step in binary workflow
2. Ensure `save-if` condition allows caching for your branch (must be `main`, `dev-atomica`, or `docker-testnet-org-refactor`)
3. Verify all three cache keys are being used:
   - `aptos-core-deps` (Job 1: core dependencies)
   - `aptos-node` (Job 2: aptos-node)
   - `aptos-cli` (Job 3: aptos CLI)
4. Clear cache and rebuild if corrupted: manually delete GitHub Actions cache
5. Note: Even if a job fails, the cache is saved due to `cache-on-failure: true`

### Resource Exhaustion

The new approach should prevent resource exhaustion. If issues persist:

1. Verify you're using `Dockerfile.prebuilt`, not the legacy `Dockerfile`
2. Check that the workflow is `build-validator-image-v2.yml`
3. Review timeout settings (should be ~30 min for Docker, ~90 min for binary)

## Migration from Legacy Workflow

The legacy `build-validator-image.yml` workflow is still present but should be considered deprecated.

**Migration steps:**

1. Ensure both new workflows are enabled
2. Monitor the first few builds to verify success
3. Once stable, consider removing or archiving the legacy workflow
4. Update any external references to use the new image tags

## Future Enhancements

Potential improvements to consider:

- [ ] Build additional binaries (`aptos-node`, `aptos-faucet-service`)
- [ ] Support multi-architecture builds (ARM64)
- [ ] Add smoke tests to workflow
- [ ] Implement binary signing/verification
- [ ] Add performance benchmarks to binary builds
