# Atomica Aptos Docker Images

This directory contains Docker configurations for building Atomica Aptos validator node images.

> [!IMPORTANT]
> You should always use the published images from the GitHub Container Registry.
> We DO NOT build binaries inside Docker. We build Linux binaries on GitHub Actions runners (outside Docker) and copy them into the Docker image.

## Build Strategy

We use a strictly defined two-stage build pipeline to ensure reproducibility and performance:

### 1. Binary Build (Outside Docker)
The `aptos` and `aptos-node` binaries are built directly on GitHub Actions Linux runners. This leverages caching more effectively and avoids the overhead and complexity of building inside a container.
- **Workflow:** `build-aptos-binary.yml`
- **Output:** GitHub Release artifacts (e.g., `aptos`, `aptos-node`)

### 2. Docker Image Assembly
Once the binaries are built and released, the Docker image is assembled by simply downloading the pre-built Linux binaries and placing them into the image.
- **Workflow:** `build-validator-image.yml`
- **File:** `Dockerfile`
- **Process:**
    1. Download `aptos` and `aptos-node` from the GitHub Release.
    2. Copy them into the Ubuntu base image.
    3. Configure the runtime environment.

## Usage

### Using Published Images
Always use the images published to our registry.

```bash
docker pull ghcr.io/bomba-atomica/atomica-aptos/validator:<tag>
```

### Tag Strategy

| Tag Format | Description | Example |
|------------|-------------|---------|
| `<sha>-<dockerfile-hash>` | Unique build identifier (SHA + Dockerfile hash) | `80cf17c75e-a1b2c3d4` |
| `<sha>` | Git commit hash | `80cf17c75e` |
| `binary-<sha>` | Matching the binary release tag | `binary-80cf17c75e` |
| `latest` | Latest non-prerelease build | `latest` |

## Troubleshooting

Since we do not support local builds, if you need to test changes:
1. Push your changes to a branch that triggers the build workflow.
2. Wait for the binary build and subsequent Docker image build to complete.
3. Pull the resulting image using the commit SHA tag.

If the Docker image build fails with "Binary release not found", ensure the binary build workflow for your commit has completed successfully.
