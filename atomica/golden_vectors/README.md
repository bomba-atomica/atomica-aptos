# Timelock Encryption Golden Test Vectors

This directory contains golden test vectors for Atomica's timelock encryption system. These vectors ensure consistency across all implementations:

- **Rust implementation** (`crates/aptos-dkg`)
- **Move contracts** (`aptos-move/framework/aptos-framework/sources/ibe_config.move`)
- **Move VM native functions** (if/when implemented)

## Files

- `timelock_golden_vectors.json` - Machine-readable test vectors in JSON format
- `timelock_golden_vectors.txt` - Human-readable reference documentation

## Test Vectors

Each test vector contains:

- **timelock_id** - The unique timelock identifier (u64)
- **deadline_us** - The deadline timestamp in microseconds (u64)
- **timelock_id_bcs_hex** - BCS-encoded timelock_id (little-endian, 8 bytes)
- **deadline_us_bcs_hex** - BCS-encoded deadline_us (little-endian, 8 bytes)
- **identity_hash_hex** - SHA3-256 hash of `BCS(timelock_id) || BCS(deadline_us)` (32 bytes)

## Identity Computation

The identity hash is computed as:

```
identity = SHA3-256(BCS(timelock_id) || BCS(deadline_us))
```

This matches the Move contract implementation in `ibe_config.move`:

```move
let identity_input = vector::empty<u8>();
vector::append(&mut identity_input, bcs::to_bytes(&timelock_id));
vector::append(&mut identity_input, bcs::to_bytes(&deadline_us));
let identity = sha3_256(identity_input);
```

## Regenerating Vectors

To regenerate the golden test vectors (e.g., after modifying the identity computation logic):

```bash
cd /path/to/atomica-aptos
cargo test --package aptos-dkg generate_golden_vectors -- --ignored --nocapture
```

The generator is located at `crates/aptos-dkg/src/ibe/golden_vectors.rs`.

**Important:** Only regenerate vectors when intentionally changing the identity computation algorithm. These vectors serve as a stable reference for cross-implementation testing.

## Using These Vectors

### In Move Unit Tests

```move
#[test]
fun test_identity_matches_golden_vector() {
    // Test vector #1: timelock_id=0, deadline_us=1000000000000
    let timelock_id = 0u64;
    let deadline_us = 1000000000000u64;

    let expected_identity = x"dadcc1614575180d09b4d638b16eb2ee581dae80bbd3ac9c95b06605e51718f3";

    let identity_input = vector::empty<u8>();
    vector::append(&mut identity_input, bcs::to_bytes(&timelock_id));
    vector::append(&mut identity_input, bcs::to_bytes(&deadline_us));
    let actual_identity = sha3_256(identity_input);

    assert!(actual_identity == expected_identity, 0);
}
```

### In Rust Tests

```rust
#[test]
fn test_identity_matches_golden_vector() {
    use sha3::{Digest, Sha3_256};

    // Test vector #1
    let timelock_id = 0u64;
    let deadline_us = 1_000_000_000_000u64;

    let expected = hex::decode(
        "dadcc1614575180d09b4d638b16eb2ee581dae80bbd3ac9c95b06605e51718f3"
    ).unwrap();

    let mut hasher = Sha3_256::new();
    hasher.update(&bcs::to_bytes(&timelock_id).unwrap());
    hasher.update(&bcs::to_bytes(&deadline_us).unwrap());
    let actual: Vec<u8> = hasher.finalize().to_vec();

    assert_eq!(actual, expected);
}
```

## Test Coverage

The vectors cover:

1. **Basic case** - Minimal values to verify correct implementation
2. **ID uniqueness** - Different IDs with same deadline produce different identities
3. **Deadline uniqueness** - Same ID with different deadlines produce different identities
4. **Large values** - Realistic large values for stress testing
5. **Edge cases** - Maximum u64 values to test boundary conditions

## Version History

- **v1.0.0** (2026-01-20) - Initial golden vectors for identity computation
