# CRITICAL FINDING: MPK Extraction is Already Implemented!

## Discovery

The Rust→Move bridge for MPK extraction **IS ALREADY IMPLEMENTED** in the codebase!

### Location

**File:** `aptos-move/aptos-vm/src/validator_txns/dkg.rs`

### Code Analysis

```rust
// Line 52-56: MPK extraction function
fn extract_mpk_from_transcript(transcript_bytes: &[u8]) -> Result<Vec<u8>, ExecutionFailure> {
    let transcript: Transcripts = bcs::from_bytes(transcript_bytes)
        .map_err(|_| Expected(ExpectedFailure::TranscriptDeserializationFailed))?;
    Ok(transcript.main.get_dealt_public_key().to_bytes().to_vec())
}

// Line 117: MPK is extracted
let mpk = extract_mpk_from_transcript(dkg_node.transcript_bytes.as_slice())?;

// Lines 122-126: MPK is passed to Move function
let args = vec![
    MoveValue::Signer(AccountAddress::ONE),
    dkg_node.transcript_bytes.as_move_value(),
    mpk.as_move_value(),  // ← MPK IS PASSED!
];

session.execute_function_bypass_visibility(
    &RECONFIGURATION_WITH_DKG_MODULE,
    FINISH_WITH_DKG_RESULT,  // Calls reconfiguration_with_dkg::finish_with_dkg_result()
    ...
);
```

## What This Means

1. **The MPK IS being extracted** from the DKG transcript
2. **The MPK IS being passed** to `reconfiguration_with_dkg::finish_with_dkg_result()`
3. **The Move code SHOULD be setting** the IBE config with this MPK

## Why IBE Config is Still Missing

If the code is extracting and passing MPK correctly, but `IBEPublicParams` resource is still missing on-chain, then the issue must be in ONE of these places:

### Hypothesis 1: Empty MPK

The MPK being extracted is empty or invalid, causing the Move code to skip setting it:

```move
// reconfiguration_with_dkg.move:69
if (!vector::is_empty(&mpk)) {
    let new_epoch = reconfiguration::current_epoch() + 1;
    ibe_config::set_mpk(mpk, new_epoch);  // Only called if mpk is NOT empty
};
```

**Check:** Is `transcript.main.get_dealt_public_key()` returning an empty vector?

### Hypothesis 2: Scalar Transcript Not in Serialized Transcript

The `Transcripts` struct has a `scalar: Option<ScalarTrx>` field. When serialized with BCS, if `scalar` is `None`, the MPK extraction would still work from `main`, but scalar shares wouldn't be available for IBE.

**Check:** Is `scalar: Some(...)` in the serialized transcript, or is it `None`?

### Hypothesis 3: genesis.move Not Calling initialize_timelock_registry

The `ibe_config::initialize()` might be called, but `ibe_config::initialize_timelock_registry()` might not be, causing the resource to not exist.

**Check:** Does `genesis.move` call both functions?

## Next Debugging Steps

### Step 1: Add Debug Logging in dkg.rs

```rust
// In extract_mpk_from_transcript()
fn extract_mpk_from_transcript(transcript_bytes: &[u8]) -> Result<Vec<u8>, ExecutionFailure> {
    let transcript: Transcripts = bcs::from_bytes(transcript_bytes)
        .map_err(|_| Expected(ExpectedFailure::TranscriptDeserializationFailed))?;

    let mpk = transcript.main.get_dealt_public_key().to_bytes().to_vec();

    // ADD THIS:
    eprintln!("[DEBUG] Extracted MPK from transcript:");
    eprintln!("  MPK length: {} bytes", mpk.len());
    eprintln!("  MPK hex: {}", hex::encode(&mpk));
    eprintln!("  Scalar transcript present: {}", transcript.scalar.is_some());

    Ok(mpk)
}
```

### Step 2: Check Smoke Test

Run the IBE MPK smoke test to confirm it passes:

```bash
cd testsuite/smoke-test
cargo test --lib ibe_mpk_on_chain -- --nocapture
```

If the smoke test PASSES, then compare:
- How smoke test initializes genesis
- What docker testnet genesis does differently

### Step 3: Query On-Chain After DKG

In the docker testnet, after DKG completes (epoch 2), query:

```bash
# Check if IBEPublicParams exists
curl http://localhost:8080/v1/accounts/0x1/resource/0x1::ibe_config::IBEPublicParams

# Check DKG state
curl http://localhost:8080/v1/accounts/0x1/resource/0x1::dkg::DKGState | jq '.data.last_completed.transcript' | head -c 200

# Check if TimelockRegistry exists
curl http://localhost:8080/v1/accounts/0x1/resource/0x1::ibe_config::TimelockRegistry
```

### Step 4: Check genesis.move Initialization

Verify that genesis properly initializes IBE:

```bash
grep -n "ibe_config::initialize" aptos-move/framework/aptos-framework/sources/genesis.move
```

Expected to see BOTH:
1. `ibe_config::initialize(&aptos_framework_account);` (line 133)
2. `ibe_config::initialize_timelock_registry(aptos_framework);` (line 314)

## Possible Root Causes

### Most Likely: MPK is Empty

The `transcript.main.get_dealt_public_key()` might be returning an empty vector because:
1. The transcript aggregation failed
2. Only partial transcripts were received
3. The dealt public key is not being set correctly

**Evidence needed:** Log the MPK length before passing to Move

### Less Likely: Scalar Transcript Missing

The scalar transcript might not be in the aggregated transcript because:
1. DKG manager doesn't include it in aggregation
2. Serialization drops the `scalar` field
3. Validators don't deal scalar transcripts

**Evidence needed:** Deserialize the on-chain transcript and check `scalar.is_some()`

### Unlikely: Move Code Bug

The Move code might have a bug in `ibe_config::set_mpk()` that prevents the resource from being created.

**Evidence needed:** Add Move debug statements or check transaction outputs

## Recommended Immediate Action

1. **Add logging** to `extract_mpk_from_transcript()` to see MPK length
2. **Run smoke test** to confirm it passes with current code
3. **Compare smoke test vs docker testnet** genesis configuration
4. **Check validator logs** after adding MPK extraction logging

## Summary

The good news: **The code to extract and pass MPK already exists!**

The bad news: **Something is preventing it from working correctly in docker testnet.**

Most likely cause: **MPK is empty** when extracted from transcript, causing Move code to skip `ibe_config::set_mpk()`.

**Next step:** Add debug logging and re-run docker testnet to see what MPK value is being extracted.
