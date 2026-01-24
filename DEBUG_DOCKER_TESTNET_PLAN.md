# Docker Testnet Debugging Plan

## Problem Statement

The latest published docker image (`ghcr.io/bomba-atomica/atomica-aptos/validator:latest`) fails to make blockchain progress in local testnet, while the prior published image worked correctly. The repository has heavily modified DKG (Distributed Key Generation) implementation, particularly implementing dual-output DKG for both randomness and IBE (Identity-Based Encryption).

## Root Cause Hypotheses

1. **Missing DKG feature flags at genesis** - The dual-output DKG may require specific feature flags to be enabled during genesis generation
2. **DKG implementation issues** - The chunked lifted ElGamal PVSS for scalar shares may have bugs
3. **Binary build issues** - The latest binaries may be missing required features or built incorrectly
4. **Genesis configuration** - Missing or incorrect configuration for DKG/IBE modules

## Investigation Plan

### Phase 1: Reproduce the Issue (Tasks 1-5)

**Goal:** Create a reproducible test environment that demonstrates the failure

#### Task 1.1: Create Docker Compose Configuration
- **File:** `atomica-test/docker-testnet/docker-compose.yaml`
- **Action:** Copy and adapt from `~/atomica/source/docker-testnet/config/docker-compose.yaml`
- **Key changes:**
  - Use environment variables for image selection (`IMAGE_NAME_LATEST` and `IMAGE_NAME_PRIOR`)
  - Set up 4 validators with proper networking (172.19.0.0/16)
  - Configure ports: 8080-8083 (REST API), 9101-9104 (metrics)
  - Mount genesis artifacts and validator configs

#### Task 1.2: Create Genesis Generation Script
- **File:** `atomica-test/docker-testnet/generate-genesis.sh`
- **Action:** Copy from `~/atomica/source/docker-testnet/config/generate-genesis.sh`
- **Key requirements:**
  - Support DKG/IBE feature flags
  - Use framework.mrb from docker image
  - Generate for 4 validators with proper IP addresses

#### Task 1.3: Create TypeScript Test Harness
- **File:** `atomica-test/docker-testnet/test-network.ts`
- **Purpose:** Automated test script to:
  - Start testnet with specified image
  - Wait for validator health
  - Monitor block production for 60 seconds
  - Check if blocks are progressing
  - Collect validator logs on failure
  - Generate comparison report
- **Dependencies:** Use existing `@atomica/aptos-docker-testnet` package

#### Task 1.4: Create Test Runner Scripts
- **File 1:** `atomica-test/docker-testnet/test-latest.sh` - Test latest image
- **File 2:** `atomica-test/docker-testnet/test-prior.sh` - Test prior image
- **File 3:** `atomica-test/docker-testnet/test-comparison.sh` - Run both and compare
- **Each script:**
  - Set appropriate IMAGE_NAME environment variable
  - Run TypeScript test harness
  - Collect logs to separate directories
  - Exit with clear success/failure status

#### Task 1.5: Create Package Configuration
- **File:** `atomica-test/docker-testnet/package.json`
- **Action:** Set up minimal package for TypeScript testing
- **Dependencies:**
  - `@atomica/aptos-docker-testnet`
  - `aptos` SDK
  - `typescript`
  - Test runner (e.g., `tsx` or `ts-node`)

### Phase 2: Identify Image Versions (Tasks 6-7)

**Goal:** Determine exact images to test

#### Task 2.1: Identify Latest Image
- **Action:** Find the most recent published image SHA/tag
- **Command:** `docker pull ghcr.io/bomba-atomica/atomica-aptos/validator:latest && docker images`
- **Record:** Image SHA and build date

#### Task 2.2: Identify Last Working Image
- **Action:** Query GitHub Container Registry or git history to find the prior working image
- **Options:**
  - Check git history for last successful workflow run before latest
  - Look for image tags by commit SHA
  - Ask user for specific working version
- **Record:** Image SHA/tag and commit SHA

### Phase 3: Execute Tests (Tasks 8-9)

**Goal:** Confirm the issue exists and establish baseline

#### Task 3.1: Test Latest Image
- **Run:** `./test-latest.sh`
- **Expected Result:** Network fails to progress (blocks stay at 0)
- **Collect:**
  - Validator logs (all 4 validators)
  - Genesis artifacts
  - Node configurations
  - Final block heights

#### Task 3.2: Test Prior Image
- **Run:** `./test-prior.sh`
- **Expected Result:** Network progresses normally
- **Collect:** Same artifacts as Task 3.1

### Phase 4: Log Analysis (Tasks 10-15)

**Goal:** Identify the root cause from validator logs

#### Task 4.1: Search for DKG-related Errors
- **Search patterns:**
  - `DKG.*error|DKG.*failed`
  - `transcript.*invalid|transcript.*failed`
  - `scalar.*elgamal|chunked.*elgamal`
  - `IBE.*error|IBE.*failed`
  - `PVSS.*error|PVSS.*failed`
- **Files:** Latest image validator logs
- **Expected:** May find DKG initialization or verification failures

#### Task 4.2: Search for Feature Flag Warnings
- **Search patterns:**
  - `feature.*not.*enabled|feature.*disabled`
  - `missing.*feature|requires.*feature`
  - `randomness.*config|randomness.*disabled`
  - `ibe.*config|ibe.*disabled`
- **Files:** Latest image validator logs and genesis output
- **Expected:** May find missing feature flags for DKG/IBE

#### Task 4.3: Search for Consensus Issues
- **Search patterns:**
  - `consensus.*stuck|consensus.*timeout`
  - `block.*production.*failed`
  - `epoch.*0.*stuck|no.*progress`
  - `quorum.*failed|vote.*failed`
- **Files:** Latest image validator logs
- **Expected:** Consensus may be blocked waiting for DKG

#### Task 4.4: Compare Genesis Artifacts
- **Action:** Diff genesis.blob and waypoint.txt between working and failing runs
- **Tools:** `hexdump -C genesis.blob | head -100` to inspect binary format
- **Look for:** Differences in feature flags, module initialization

#### Task 4.5: Compare Node Configurations
- **Action:** Diff node-config.yaml files between working and failing runs
- **Look for:** Missing or different configuration parameters

#### Task 4.6: Analyze Validator Startup Sequence
- **Action:** Extract and compare startup logs (first 200 lines) from both images
- **Look for:**
  - DKG initialization order
  - Module loading sequence
  - Feature flag activation
  - Genesis state loading

### Phase 5: Hypothesis Testing (Tasks 16-20)

**Goal:** Test specific fixes based on findings

#### Task 5.1: Test with DKG Feature Flags
- **Action:** Modify genesis generation to enable DKG-related features
- **Feature flags to try:**
  - `RANDOMNESS_ENABLED`
  - `IBE_ENABLED`
  - `DKG_DUAL_OUTPUT_ENABLED`
  - Check `aptos-move/framework/src/natives/features.rs` for correct flags
- **Implementation:** Update `generate-genesis.sh` or use aptos CLI flags
- **Test:** Re-run with modified genesis

#### Task 5.2: Test with Simplified DKG
- **Action:** If dual-output DKG is the issue, test with DKG disabled or randomness-only
- **Implementation:**
  - Disable scalar transcript generation
  - Or use older genesis without IBE
- **Test:** Re-run with modified genesis

#### Task 5.3: Inspect Framework Binary
- **Action:** Extract and compare framework.mrb from both docker images
- **Commands:**
  ```bash
  docker run --rm ghcr.io/bomba-atomica/atomica-aptos/validator:latest cat /opt/aptos/framework/head.mrb > latest-framework.mrb
  docker run --rm ghcr.io/bomba-atomica/atomica-aptos/validator:PRIOR_TAG cat /opt/aptos/framework/head.mrb > prior-framework.mrb
  hexdump -C latest-framework.mrb | head -200 > latest-framework.hex
  hexdump -C prior-framework.mrb | head -200 > prior-framework.hex
  diff -u prior-framework.hex latest-framework.hex
  ```
- **Look for:** Module differences, especially in DKG/IBE/randomness modules

#### Task 5.4: Test Binary Compatibility
- **Action:** Check if aptos-node binary version matches framework version
- **Commands:**
  ```bash
  docker run --rm ghcr.io/bomba-atomica/atomica-aptos/validator:latest /usr/local/bin/aptos-node --version
  docker run --rm ghcr.io/bomba-atomica/atomica-aptos/validator:latest /usr/local/bin/aptos --version
  ```
- **Verify:** Both binaries exist and report consistent versions

#### Task 5.5: Manual DKG Inspection
- **Action:** If logs show specific DKG errors, inspect the DKG state
- **Options:**
  - Query on-chain DKG state via REST API
  - Check DKG transcript storage
  - Inspect validator DKG configuration
- **Commands:**
  ```bash
  curl http://localhost:8080/v1/accounts/0x1/resource/0x1::dkg::DKGState
  curl http://localhost:8080/v1/accounts/0x1/resource/0x1::randomness_config::RandomnessConfig
  ```

### Phase 6: Fix Implementation (Tasks 21-25)

**Goal:** Implement the fix based on identified root cause

#### Task 6.1: Document Root Cause
- **File:** `atomica-test/docker-testnet/DEBUG_FINDINGS.md`
- **Content:**
  - Exact error messages
  - Root cause analysis
  - Reproduction steps
  - Fix approach

#### Task 6.2: Implement Fix
- **Options based on findings:**
  - **Option A - Missing feature flags:** Update genesis generation script
  - **Option B - DKG bug:** Fix DKG implementation in Rust code
  - **Option C - Binary issue:** Rebuild binaries with correct features
  - **Option D - Config issue:** Update validator node configuration

#### Task 6.3: Test Fix
- **Action:** Re-run test suite with fix applied
- **Success criteria:**
  - Network makes progress (blocks > 0)
  - All 4 validators stay healthy
  - Consensus reaches blocks within 60 seconds
  - No DKG-related errors in logs

#### Task 6.4: Regression Testing
- **Action:** Test fix against both latest and prior images
- **Ensure:** Fix doesn't break previously working configuration

#### Task 6.5: Document Fix
- **Update:** `DEBUG_FINDINGS.md` with:
  - Final fix implementation
  - Configuration changes needed
  - Steps to verify fix
  - Prevention measures for future

## Key Files to Create

```
atomica-aptos/
└── atomica-test/
    └── docker-testnet/
        ├── docker-compose.yaml           # Docker compose for 4-validator testnet
        ├── generate-genesis.sh           # Genesis generation with DKG support
        ├── package.json                  # TypeScript dependencies
        ├── test-network.ts               # Main test harness
        ├── test-latest.sh                # Test latest image
        ├── test-prior.sh                 # Test prior image
        ├── test-comparison.sh            # Compare both runs
        ├── README.md                     # Usage instructions
        ├── DEBUG_FINDINGS.md             # Investigation results (created during execution)
        └── logs/                         # Log collection directory
            ├── latest/
            │   ├── validator-0.log
            │   ├── validator-1.log
            │   ├── validator-2.log
            │   ├── validator-3.log
            │   ├── genesis-output.log
            │   └── test-results.json
            └── prior/
                ├── validator-0.log
                ├── validator-1.log
                ├── validator-2.log
                ├── validator-3.log
                ├── genesis-output.log
                └── test-results.json
```

## Success Criteria

1. **Reproducible test environment** - Can reliably reproduce the issue
2. **Root cause identified** - Clear understanding of why latest image fails
3. **Fix implemented and tested** - Network makes progress with fix
4. **Documentation complete** - Future developers can understand the issue and fix

## Timeline Estimate

- **Phase 1 (Reproduce):** 2-3 hours - Set up test infrastructure
- **Phase 2 (Identify):** 30 minutes - Find image versions
- **Phase 3 (Execute):** 1 hour - Run tests and collect logs
- **Phase 4 (Analysis):** 2-3 hours - Deep dive into logs and artifacts
- **Phase 5 (Testing):** 2-4 hours - Test various hypotheses
- **Phase 6 (Fix):** 1-4 hours - Implement and verify fix (varies by complexity)

**Total:** 8-15 hours depending on complexity of root cause

## Priority Areas for Investigation

1. **DKG Feature Flags** (Highest priority)
   - Dual-output DKG is a major recent change
   - Feature flags commonly cause genesis issues
   - Easy to test with genesis regeneration

2. **Genesis Module Initialization** (High priority)
   - Check if IBE/DKG modules are properly initialized
   - Verify randomness_config and ibe_config on-chain resources

3. **Binary Version Mismatch** (Medium priority)
   - Framework version must match aptos-node version
   - Check build process for latest image

4. **DKG Implementation Bugs** (Lower priority for initial investigation)
   - Would require code changes
   - More time-consuming to fix
   - Should be ruled out after configuration issues

## Next Steps

After accepting this plan:
1. Execute Phase 1 to create test infrastructure
2. Execute Phases 2-3 to reproduce the issue
3. Execute Phase 4 to analyze logs and identify root cause
4. Execute Phases 5-6 based on findings
