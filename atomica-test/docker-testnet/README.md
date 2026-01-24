# Docker Testnet Debugging Tool

Automated test harness for debugging Atomica Aptos docker image issues.

## Quick Start

```bash
# Install dependencies
npm install

# Test latest image (will likely fail)
./test-latest.sh

# Test prior working image
PRIOR_IMAGE_SHA=abc1234 ./test-prior.sh

# Run comparison test
PRIOR_IMAGE_SHA=abc1234 ./test-comparison.sh
```

## What It Tests

1. **Genesis Generation** - Uses the docker image to generate genesis artifacts
2. **Validator Startup** - Starts 4 validators with docker compose
3. **Health Check** - Waits for all validators to respond to API calls
4. **Block Production** - Monitors if the network produces blocks for 2 minutes
5. **Log Collection** - Saves all validator logs for analysis

## Test Results

- **PASS**: Network produces blocks
- **FAIL**: Network is stuck at block 0

## Directory Structure

```
atomica-test/docker-testnet/
├── test-network.ts              # Main test harness
├── test-latest.sh               # Test latest image
├── test-prior.sh                # Test prior image
├── test-comparison.sh           # Compare both
├── docker-compose.yaml          # 4-validator testnet config
├── generate-genesis.sh          # Genesis generation script
├── genesis-workspace/           # Genesis generation working directory
├── genesis-artifacts/           # Generated genesis files
├── validators/                  # Validator configs and keys
└── logs/
    ├── latest/                  # Logs from latest image test
    │   ├── validator-0.log
    │   ├── validator-1.log
    │   ├── validator-2.log
    │   ├── validator-3.log
    │   ├── genesis-output.log
    │   └── test-results.json
    └── prior/                   # Logs from prior image test
        └── (same structure)
```

## Finding the Prior Image SHA

### Option 1: Check Docker Images

```bash
docker images | grep validator
```

Look for an image tagged with `devnet` or a specific commit SHA that you know worked.

### Option 2: Check GitHub Container Registry

Visit: https://github.com/bomba-atomica/atomica-aptos/pkgs/container/atomica-aptos%2Fvalidator

Find the last known-good image SHA from the packages list.

### Option 3: Check Git History

```bash
# Find recent commits
git log --oneline -20

# Check which commit the current image is from
docker inspect ghcr.io/bomba-atomica/atomica-aptos/validator:latest | grep GIT_SHA
```

## Analyzing Test Failures

### Step 1: Check Test Results

```bash
cat logs/latest/test-results.json
```

Look at:
- `success`: Did the test pass?
- `blockProgress.blocksProduced`: How many blocks were produced?
- `validators`: Are all validators healthy?
- `errors`: What errors were reported?

### Step 2: Search Validator Logs for Errors

```bash
# Search for DKG errors
grep -i "dkg.*error\|dkg.*failed" logs/latest/validator-*.log

# Search for consensus issues
grep -i "consensus.*stuck\|consensus.*timeout" logs/latest/validator-*.log

# Search for feature flag issues
grep -i "feature.*not.*enabled\|missing.*feature" logs/latest/validator-*.log

# Search for genesis/randomness config
grep -i "randomness_config\|ibe_config" logs/latest/validator-*.log
```

### Step 3: Compare Working vs Failing

```bash
# Compare first 200 lines of startup logs
head -200 logs/prior/validator-0.log > /tmp/prior-startup.log
head -200 logs/latest/validator-0.log > /tmp/latest-startup.log
diff -u /tmp/prior-startup.log /tmp/latest-startup.log
```

### Step 4: Check Genesis Differences

```bash
# Compare waypoints
diff logs/prior/genesis-output.log logs/latest/genesis-output.log

# Check if genesis.blob differs
ls -lh genesis-artifacts/  # after each test
```

## Common Issues and Solutions

### Issue 1: Missing DKG Feature Flags

**Symptoms:**
- Network stuck at block 0
- Logs show "feature not enabled" or "randomness disabled"

**Solution:**
Modify `generate-genesis.sh` to enable DKG features:

```bash
# In layout.yaml generation, add feature flags
# Check aptos-move/framework/src/natives/features.rs for correct flag names
```

### Issue 2: DKG Transcript Verification Failure

**Symptoms:**
- Logs show "DKG transcript invalid" or "verification failed"
- May see "scalar_elgamal" or "chunked elgamal" errors

**Solution:**
This indicates a bug in the DKG implementation. Check:
- `crates/aptos-dkg/src/pvss/scalar_elgamal/`
- Verification logic for aggregated transcripts

### Issue 3: Binary Version Mismatch

**Symptoms:**
- Genesis generation fails
- "framework version mismatch" errors

**Solution:**
```bash
# Check binary versions
docker run --rm ghcr.io/bomba-atomica/atomica-aptos/validator:latest /usr/local/bin/aptos-node --version
docker run --rm ghcr.io/bomba-atomica/atomica-aptos/validator:latest /usr/local/bin/aptos --version

# Rebuild docker image with matching versions
```

## Advanced Debugging

### Enable Debug Logging

Edit `docker-compose.yaml` to add more verbose logging:

```yaml
environment:
  - RUST_LOG=debug,aptos_dkg=trace,consensus=trace
  - RUST_BACKTRACE=full
```

### Query On-Chain State

```bash
# Check DKG state
curl -s http://localhost:8080/v1/accounts/0x1/resource/0x1::dkg::DKGState | jq

# Check randomness config
curl -s http://localhost:8080/v1/accounts/0x1/resource/0x1::randomness_config::RandomnessConfig | jq

# Check IBE config
curl -s http://localhost:8080/v1/accounts/0x1/resource/0x1::ibe_config::IbeConfig | jq
```

### Inspect Framework

```bash
# Extract framework from image
docker run --rm ghcr.io/bomba-atomica/atomica-aptos/validator:latest cat /opt/aptos/framework/head.mrb > framework.mrb

# Compare frameworks
hexdump -C framework.mrb | head -200
```

## Cleanup

```bash
# Stop and remove all containers
docker compose down -v --remove-orphans

# Remove generated artifacts
rm -rf genesis-workspace genesis-artifacts validators logs

# Remove docker volumes
docker volume rm $(docker volume ls -q | grep atomica-debug)
```

## Environment Variables

- `IMAGE_NAME`: Docker image to test (default: ghcr.io/bomba-atomica/atomica-aptos/validator:latest)
- `PRIOR_IMAGE_SHA`: SHA or tag of prior working image (for test-prior.sh)
- `LOG_DIR`: Directory for log output (default: ./logs/current)
- `ATOMICA_DEBUG_TESTNET`: Set to `1` for verbose genesis generation logs

## Troubleshooting

### "Permission denied" errors

```bash
chmod +x *.sh
```

### "Docker daemon not running"

```bash
sudo systemctl start docker
# or
docker info
```

### "Port already in use"

```bash
# Check what's using ports 8080-8083
lsof -i :8080-8083

# Stop conflicting services
docker compose down -v
```

### "Module not found" errors

```bash
# Make sure you're in the docker-testnet directory
cd atomica-test/docker-testnet

# Install dependencies
npm install
```
