# IBE DK Reconstruction Architecture

## Ground Truth

The source of truth is `crates/aptos-dkg/src/ibe/mod.rs::reconstruct_ibe_dk()`:

```rust
pub fn reconstruct_ibe_dk(
    validator_indices: &[u64],
    scalar_shares: &[Vec<Scalar>],  // GROUND TRUTH
    weights: &[u64],
    total_weight: u64,
    identity: &[u8; 32],
) -> Result<G1Affine, IbeError>
```

## Data Format

### `scalar_shares`: `Vec<Vec<Scalar>>`

- Outer Vec: One entry per participating validator (matches `validator_indices`)
- Inner Vec: ALL scalar shares for that validator (count = validator's weight)
- Each `Scalar`: 32 bytes, little-endian serialization

### Move VM Interface

Move can only pass `vector<vector<u8>>` to native functions:

```move
public fun reconstruct_ibe_dk<G1>(
    validator_indices: vector<u64>,
    scalar_shares: vector<vector<u8>>,  // 32-byte little-endian scalars
    weights: vector<u64>,
    threshold: u64,
    total_weight: u64,
    identity: vector<u8>,
): vector<u8>  // 48-byte compressed G1
```

## Native Function Implementation

1. Extract arguments from Move VM
2. Deserialize each inner `Vec<u8>` to `Scalar` using `Scalar::from_bytes_le()`
3. Wrap each `Scalar` in `Vec<Scalar>` for apt-dkg API
4. Call `reconstruct_ibe_dk()`
5. Serialize result as 48-byte compressed G1 and return

## Protocol Flow

1. DKG outputs scalar shares `s_i` from PVSS
2. Validators submit 32-byte little-endian serialized scalar shares
3. Native function reconstructs master secret via Lagrange interpolation
4. Computes `DK = H(identity)^secret`
5. Returns 48-byte compressed G1

## Related Files

| File                                                              | Purpose                  |
| ----------------------------------------------------------------- | ------------------------ |
| `crates/aptos-dkg/src/ibe/mod.rs`                                 | Canonical implementation |
| `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`    | Native function          |
| `aptos-move/framework/aptos-stdlib/sources/cryptography/ibe.move` | Move interface           |
