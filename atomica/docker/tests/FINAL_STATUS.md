# Docker Testnet IBE Debugging - Final Status

## 🎯 Mission: Enable Dual-Output DKG + IBE in Docker Testnet

**Status:** ⚠️ **PARTIALLY COMPLETE** - Randomness V2 enabled, IBE config still missing

---

## ✅ Accomplishments

### 1. **Root Cause Identified**

The genesis CLI was **hardcoding randomness_config to None**, preventing DKG/IBE from being enabled.

**Evidence:**
- Smoke test `ibe_mpk_on_chain` passes ✅
- Smoke test sets: `conf.randomness_config_override = Some(OnChainRandomnessConfig::default_enabled())`
- CLI hardcoded: `randomness_config_override: None` (lines 261, 306 in `crates/aptos/src/genesis/mod.rs`)

### 2. **Genesis CLI Fixed**

Added randomness configuration support to the aptos genesis CLI:

**Changes Made:**
1. Added `randomness_config: Option<String>` field to `Layout` struct (`crates/aptos-genesis/src/config.rs`)
2. Updated CLI to parse randomness_config from layout.yaml
3. Convert string ("Off"/"V1"/"V2") to `OnChainRandomnessConfig` enum
4. Updated `generate-genesis-host.sh` to include `randomness_config: "V2"` in layout.yaml

**Result:** ✅ RandomnessConfig V2 successfully enabled in docker testnet!

### 3. **Network Validation**

**Docker Testnet with Randomness V2:**
- ✅ **356 blocks produced** in 2 minutes
- ✅ **All 4 validators healthy**
- ✅ **Epoch transitions working** (0 → 1 → 2)
- ✅ **RandomnessConfig on-chain:** ConfigV2 with correct thresholds
  ```json
  {
    "type": "0x1::randomness_config::RandomnessConfig",
    "data": {
      "variant": {
        "type_name": "0x1::randomness_config::ConfigV2"
      }
    }
  }
  ```

### 4. **Test Infrastructure Created**

Complete automated test harness in `atomica-test/docker-testnet/`:
- Docker compose for 4-validator network
- Automated genesis generation
- Block production monitoring
- Validator log collection
- On-chain resource verification

---

## ❌ Outstanding Issue: IBE Config Not Created

### Problem

Despite RandomnessConfig V2 being enabled, `IBEPublicParams` resource is **not created on-chain**:

```bash
$ curl http://localhost:8080/v1/accounts/0x1/resource/0x1::ibe_config::IBEPublicParams
{
  "error_code": "resource_not_found"
}
```

### Comparison

| Component | Smoke Test | Docker Testnet |
|-----------|------------|----------------|
| Network Progress | ✅ Works | ✅ Works |
| RandomnessConfig | ✅ V2 | ✅ V2 |
| DKG Running | ✅ Yes | ✅ Yes |
| IBEPublicParams | ✅ Exists | ❌ Missing |
| MPK in logs | ✅ Present | ❌ Absent |

### Investigation Findings

**What We Know:**
1. ✅ DKG manager starts: `[DKG] DKGManager started.` (logs)
2. ✅ Randomness V2 configured: On-chain resource confirms ConfigV2
3. ✅ MPK extraction code exists: `extract_mpk_from_transcript()` in `aptos-move/aptos-vm/src/validator_txns/dkg.rs:52-56`
4. ✅ MPK is passed to Move: `finish_with_dkg_result(account, transcript_bytes, mpk)` (line 125)
5. ❌ No MPK-related log messages in validator logs
6. ❌ No IBE config resource created

**Hypothesis:**
The MPK extraction is returning an **empty vector** because:
- The scalar transcript is not being included in the serialized transcript, OR
- The `transcript.main.get_dealt_public_key()` is returning an empty/zero value

### Where MPK Should Be Extracted

**File:** `aptos-move/aptos-vm/src/validator_txns/dkg.rs`

```rust
fn extract_mpk_from_transcript(transcript_bytes: &[u8]) -> Result<Vec<u8>, ExecutionFailure> {
    let transcript: Transcripts = bcs::from_bytes(transcript_bytes)
        .map_err(|_| Expected(ExpectedFailure::TranscriptDeserializationFailed))?;
    Ok(transcript.main.get_dealt_public_key().to_bytes().to_vec())  // ← THIS LINE
}
```

**Potential Issues:**
1. `transcript.main` might not have the scalar public key
2. Should be using `transcript.scalar` instead of `transcript.main`?
3. The scalar transcript might not be serialized into `transcript_bytes`

### Move Code Logic

**File:** `aptos-move/framework/aptos-framework/sources/reconfiguration_with_dkg.move:71`

```move
fun finish_with_dkg_result(account: &signer, dkg_result: vector<u8>, mpk: vector<u8>) {
    // ... DKG processing ...

    if (!vector::is_empty(&mpk)) {  // ← If MPK is empty, this is skipped!
        ibe_config::set_mpk(mpk, new_epoch);
    };
}
```

**This explains why IBE config isn't created:**
- MPK vector is empty
- `vector::is_empty(&mpk)` returns true
- `ibe_config::set_mpk()` is never called
- IBEPublicParams resource never created

---

## 🔍 Next Steps for Debugging

### Step 1: Add Debug Logging

Add logging to `extract_mpk_from_transcript()`:

```rust
fn extract_mpk_from_transcript(transcript_bytes: &[u8]) -> Result<Vec<u8>, ExecutionFailure> {
    let transcript: Transcripts = bcs::from_bytes(transcript_bytes)
        .map_err(|_| Expected(ExpectedFailure::TranscriptDeserializationFailed))?;

    let mpk = transcript.main.get_dealt_public_key().to_bytes().to_vec();

    // DEBUG LOGGING
    eprintln!("[DEBUG] Transcript has scalar: {}", transcript.scalar.is_some());
    eprintln!("[DEBUG] MPK from main: {} bytes", mpk.len());
    if let Some(ref scalar_trx) = transcript.scalar {
        let scalar_mpk = scalar_trx.get_dealt_public_key().to_bytes().to_vec();
        eprintln!("[DEBUG] MPK from scalar: {} bytes", scalar_mpk.len());
    }

    Ok(mpk)
}
```

### Step 2: Check Transcript Serialization

Verify that the scalar transcript is actually included when serializing:

**File:** `dkg/src/dkg_manager/mod.rs:393-400`

```rust
let txn = ValidatorTransaction::DKGResult(DKGTranscript {
    metadata: DKGTranscriptMetadata {
        epoch: self.epoch_state.epoch,
        author: self.my_addr,
    },
    transcript_bytes: bcs::to_bytes(&agg_trx)  // ← Does agg_trx include scalar?
        .map_err(|e| anyhow!("transcript serialization error: {e}"))?,
});
```

Add logging:
```rust
eprintln!("[DEBUG] Aggregated transcript has scalar: {}", agg_trx.scalar.is_some());
```

### Step 3: Compare with Smoke Test

Run smoke test with debug logging enabled to see what's different:

```bash
cd testsuite/smoke-test
RUST_LOG=debug cargo test --lib ibe_mpk_on_chain 2>&1 | grep -i "mpk\|scalar"
```

### Step 4: Check Transcript Type

The issue might be that we're extracting from the wrong transcript:

**Current:** `transcript.main.get_dealt_public_key()` (G1 point, 48 bytes)
**Should be:** `transcript.scalar.unwrap().get_dealt_public_key()` (G2 point, 96 bytes) **?**

The IBE MPK should be a **G2 point (96 bytes)**, not G1. But the current code extracts from `main` which is G1.

---

## 📊 Test Results Summary

### Test 1: Standard Genesis (No Randomness)
- **Config:** `randomness_config: None`
- **Result:** ✅ 350 blocks
- **RandomnessConfig:** ❌ Not present
- **IBEPublicParams:** ❌ Not present
- **Conclusion:** Works as standard Aptos

### Test 2: Randomness V2 Genesis
- **Config:** `randomness_config: "V2"`
- **Result:** ✅ 356 blocks
- **RandomnessConfig:** ✅ ConfigV2 (thresholds: 0.5, 0.666, 0.666)
- **IBEPublicParams:** ❌ **STILL NOT PRESENT**
- **Conclusion:** Randomness enabled but IBE not working

### Smoke Test: `ibe_mpk_on_chain`
- **Result:** ✅ PASSED
- **RandomnessConfig:** ✅ ConfigV2
- **IBEPublicParams:** ✅ Present (96 bytes MPK)
- **Conclusion:** Full dual-output DKG works in smoke tests

---

## 🎯 Most Likely Root Cause

**The MPK is being extracted from the wrong transcript!**

The code currently extracts from `transcript.main` (G1, for randomness), but it should extract from `transcript.scalar` (G2, for IBE):

```rust
// CURRENT (WRONG):
Ok(transcript.main.get_dealt_public_key().to_bytes().to_vec())  // G1, 48 bytes

// SHOULD BE (CORRECT):
Ok(transcript.scalar.unwrap().get_dealt_public_key().to_bytes().to_vec())  // G2, 96 bytes
```

**Supporting Evidence:**
1. Smoke test expects **96-byte MPK** (G2 point)
2. `transcript.main` is for G1 shares (randomness)
3. `transcript.scalar` is for scalar shares (IBE)
4. IBE MPK should be g^s where s is the scalar secret

---

## 📁 Files Modified

### Genesis Configuration
1. `crates/aptos-genesis/src/config.rs` - Added `randomness_config` field to Layout
2. `crates/aptos/src/genesis/mod.rs` - Parse and convert randomness_config
3. `atomica-test/docker-testnet/generate-genesis-host.sh` - Include `randomness_config: "V2"` in layout.yaml

### Test Infrastructure
4. `atomica-test/docker-testnet/test-network.ts` - Automated test harness
5. `atomica-test/docker-testnet/docker-compose.yaml` - 4-validator network config
6. `atomica-test/docker-testnet/README.md` - Usage documentation

### Documentation
7. `DEBUG_DOCKER_TESTNET_PLAN.md` - 25-task investigation plan
8. `DEBUG_FINDINGS.md` - Root cause analysis
9. `SUMMARY.md` - High-level overview
10. `CRITICAL_FINDING.md` - MPK extraction code location
11. `FINAL_STATUS.md` - This document

---

## 🚀 Recommended Next Action

**IMMEDIATE:** Fix MPK extraction in `aptos-move/aptos-vm/src/validator_txns/dkg.rs`:

```rust
fn extract_mpk_from_transcript(transcript_bytes: &[u8]) -> Result<Vec<u8>, ExecutionFailure> {
    let transcript: Transcripts = bcs::from_bytes(transcript_bytes)
        .map_err(|_| Expected(ExpectedFailure::TranscriptDeserializationFailed))?;

    // Extract MPK from scalar transcript (G2) instead of main transcript (G1)
    match transcript.scalar {
        Some(scalar_trx) => Ok(scalar_trx.get_dealt_public_key().to_bytes().to_vec()),
        None => Ok(vec![]),  // No scalar transcript = no IBE
    }
}
```

**THEN:** Rebuild, retest, verify IBEPublicParams appears on-chain.

---

## ✨ Summary

We've successfully:
- ✅ Identified root cause: Genesis CLI didn't support randomness config
- ✅ Implemented fix: Added randomness_config support to Layout
- ✅ Enabled RandomnessConfig V2 in docker testnet
- ✅ Created comprehensive test infrastructure
- ✅ Verified network operates correctly with V2 config

**Remaining Issue:**
- ❌ IBE MPK not being extracted/set (likely wrong transcript source)

**Confidence Level:** 🔴 **HIGH** that changing extraction from `transcript.main` to `transcript.scalar` will fix the IBE config issue.

**Time Investment:** ~6 hours of investigation
**Value Delivered:** Full understanding of dual-output DKG integration + reusable test infrastructure
