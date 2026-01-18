# Chunked Lifted ElGamal: Scalar Generation for Distributed IBE

This document explains the scalar generation mechanism in Aptos's chunked lifted ElGamal scheme, designed for distributed key generation (DKG) and identity-based encryption (IBE).

## Overview

The chunked lifted ElGamal scheme produces **scalars** that serve as secret shares for threshold IBE/BIBE systems. The key insight is that a single scalar (e.g., a BIBE master secret key) is:

1. **Split into chunks** of `ell` bits each (default: 16 bits)
2. **Encrypted with correlated randomness** to maintain correctness
3. **Decrypted via discrete log** using a BSGS table
4. **Reconstructed** from chunks using base-B arithmetic

## Core Components

### 1. Chunking: Splitting Scalars into Fixed-Size Pieces

**File:** `crates/aptos-dkg/src/pvss/chunky/chunks.rs`

```rust
/// Converts a field element into little-endian chunks of `num_bits` bits.
pub(crate) fn scalar_to_le_chunks<F: PrimeField>(num_bits: u8, scalar: &F) -> Vec<F> {
    assert!(
        num_bits.is_multiple_of(8) && num_bits > 0 && num_bits <= 64,
        "Invalid chunk size"
    );

    let bytes = scalar.into_bigint().to_bytes_le();
    let num_bytes = num_bits / 8;
    let num_chunks = bytes.len().div_ceil(num_bytes as usize);

    let mut chunks = Vec::with_capacity(num_chunks);

    for bytes_chunk in bytes.chunks(num_bytes as usize) {
        let mut padded = [0u8; 8]; // The last chunk might be shorter
        padded[..bytes_chunk.len()].copy_from_slice(bytes_chunk);

        let chunk_val = u64::from_le_bytes(padded);
        let chunk = F::from(chunk_val);
        chunks.push(chunk);
    }

    chunks
}
```

**Example:** For `ell = 16` (radix B = 2^16 = 65536) on a 255-bit scalar field:

- Each chunk is 16 bits (2 bytes)
- `num_chunks = ceil(255/16) = 16` chunks
- Original scalar: `z = Σ_j z_j * B^j` where `z_j ∈ [0, B)`

### 2. Correlated Randomness Generation

**File:** `crates/aptos-dkg/src/pvss/chunky/chunked_elgamal.rs`

```rust
pub(crate) fn correlated_randomness<F, R>(rng: &mut R, radix: u64, num_chunks: u32) -> Vec<F>
where
    F: ark_ff::PrimeField,
    R: rand_core::RngCore + rand_core::CryptoRng,
{
    let mut r_vals = Vec::with_capacity(num_chunks as usize);
    r_vals.push(F::zero()); // placeholder for r_0
    let mut remainder = F::zero();

    // Precompute radix as F once
    let radix_f = F::from(radix);
    let mut cur_base = radix_f;

    // Fill r_1 .. r_{num_chunks-1} randomly
    for _ in 1..num_chunks {
        let r = sample_field_element(rng);
        r_vals.push(r);
        remainder -= r * cur_base;  // Accumulate to compute r_0
        cur_base *= radix_f;
    }

    r_vals[0] = remainder;  // Set r_0 to make Σ r_j * B^j = 0

    r_vals
}
```

**Key Property:** `Σ_{j=0}^{n-1} r_j * B^j = 0`

This is critical for correctness: when a validator decrypts and reconstructs the original scalar, the correlated randomness cancels out.

### 3. Number of Chunks Calculation

```rust
pub(crate) fn num_chunks_per_scalar<F: PrimeField>(ell: u8) -> u32 {
    F::MODULUS_BIT_SIZE.div_ceil(ell as u32)
}
```

For BLS12-381's Fr (~255 bits) with `ell = 16`:

- `num_chunks = ceil(255/16) = 16`

## The Homomorphism: Encryption

**File:** `crates/aptos-dkg/src/pvss/chunky/chunked_elgamal.rs`

```rust
/// Given:
/// - `G_1, H_1` ∈ G₁ (group generators)
/// - `ek_i` ∈ G₁ (encryption keys for player i)
/// - `z_{i,j}` ∈ Scalar<E> (plaintext z_i, chunked into chunks z_{i,j})
/// - `r_j` ∈ Scalar<E> (randomness for each chunk column)
///
/// The homomorphism maps input to:
/// C_{i,j} = G_1 * z_{i,j} + ek_i * r_j
/// R_j    = H_1 * r_j
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct Homomorphism<'a, E: Pairing> {
    pub pp: &'a PublicParameters<E>,
    pub eks: &'a [E::G1Affine],
}

impl<E: Pairing> homomorphism::Trait for Homomorphism<'_, E> {
    type Codomain = CodomainShape<E::G1>;
    type Domain = Witness<E::ScalarField>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        self.apply_msm(self.msm_terms(input))
    }
}

impl<'a, E: Pairing> fixed_base_msms::Trait for Homomorphism<'a, E> {
    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        // C_{i,j} = z_{i,j} * G_1 + r_j * ek[i]
        let Cs = input
            .plaintext_chunks
            .iter()
            .enumerate()
            .map(|(i, z_i)| {
                chunks_msm_terms(self.pp, self.eks[i], z_i, &input.plaintext_randomness)
            })
            .collect();

        // R_j = r_j * H_1
        let Rs = input
            .plaintext_randomness
            .iter()
            .map(|&r_j| MsmInput {
                bases: vec![self.pp.H],
                scalars: vec![r_j.0],
            })
            .collect();

        CodomainShape {
            chunks: Cs,
            randomness: Rs,
        }
    }

    fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput {
        E::G1::msm(input.bases(), input.scalars()).expect("MSM failed")
    }
}

fn chunks_msm_terms<E: Pairing>(
    pp: &PublicParameters<E>,
    ek: <E as Pairing>::G1Affine,
    chunks: &[Scalar<E::ScalarField>],
    correlated_randomness: &[Scalar<E::ScalarField>],
) -> Vec<MsmInput<E::G1Affine, E::ScalarField>> {
    chunks
        .iter()
        .zip(correlated_randomness.iter())
        .map(|(&z_ij, &r_j)| MsmInput {
            bases: vec![pp.G, ek],
            scalars: vec![z_ij.0, r_j.0],
        })
        .collect()
}
```

**Witness Structure:**

```rust
#[derive(SigmaProtocolWitness, ...)]
pub struct Witness<F: PrimeField> {
    pub plaintext_chunks: Vec<Vec<Scalar<F>>>,     // z_{i,j} for each player i, chunk j
    pub plaintext_randomness: Vec<Scalar<F>>,      // r_j for each chunk column
}

#[derive(CanonicalSerialize, ...)]
pub struct CodomainShape<T> {
    pub chunks: Vec<Vec<T>>,   // C_{i,j} ciphertexts
    pub randomness: Vec<T>,    // R_j randomness commitments
}
```

## Decryption: Recovering the Scalar

**File:** `crates/aptos-dkg/src/pvss/chunky/chunked_elgamal.rs` (tests section)

```rust
fn test_decrypt_roundtrip<E: Pairing>() {
    // ... setup ...

    // Apply homomorphism to obtain chunked ciphertexts
    let CodomainShape::<E::G1> {
        chunks: Cs,
        randomness: Rs,
    } = hom.apply(&witness);

    // Build BSGS table for discrete log computation
    let table = dlog::table::build::<E::G1>(pp.G.into(), 1u32 << (radix_exponent / 2));

    // Decrypt each player's ciphertext
    for i in 0..num_players {
        // Compute C_{i,j} - dk_i * R_j for all chunks
        // This cancels the encryption randomness, leaving G * z_{i,j}
        let exponentiated_chunks: Vec<E::G1> = Cs[i]
            .iter()
            .zip(Rs.iter())
            .map(|(C_ij, &R_j)| C_ij.sub(R_j * dks[i]))
            .collect();

        // Recover plaintext chunk values via BSGS
        // Each chunk z_{i,j} is in [0, B) where B = 2^ell
        let chunks: Vec<_> = bsgs::dlog_vec(
            pp.G.into_group(),
            &exponentiated_chunks,
            &table,
            1 << radix_exponent,  // Search space: 2^ell
        )
        .expect("dlog_vec failed")
        .into_iter()
        .map(|x| E::ScalarField::from(x))
        .collect();

        // Reconstruct original scalar: z_i = Σ_j z_{i,j} * B^j
        let recovered = chunks::le_chunks_to_scalar(radix_exponent, &chunks);
    }
}
```

**Decryption Steps:**

1. **Cancel randomness:** `C_{i,j} - dk_i * R_j = G * z_{i,j}` (since `R_j = H * r_j` and `ek_i = H * dk_i`)
2. **Discrete log:** Solve `G * x = result` for `x` in `[0, 2^ell)` using BSGS
3. **Reconstruct:** `z_i = Σ_j chunk_j * (2^ell)^j`

## Scalar Reconstruction

**File:** `crates/aptos-dkg/src/pvss/chunky/chunks.rs`

```rust
/// Reconstructs a field element from `num_bits`-bit chunks (little-endian order).
pub(crate) fn le_chunks_to_scalar<F: PrimeField>(num_bits: u8, chunks: &[F]) -> F {
    assert!(
        num_bits.is_multiple_of(8) && num_bits > 0 && num_bits <= 64,
        "Invalid chunk size"
    );

    let base = F::from(1u128 << num_bits);  // B = 2^ell
    let mut acc = F::zero();
    let mut multiplier = F::one();

    for &chunk in chunks {
        acc += chunk * multiplier;
        multiplier *= base;
    }

    acc
}
```

## Complete Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                         DEALER (Distributor)                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Input: Master Secret Key scalar `msk`                              │
│                                                                      │
│  1. Chunk the scalar:                                                │
│     msk ──► [z_0, z_1, ..., z_{n-1}]  where msk = Σ z_j * B^j      │
│                                                                      │
│  2. Generate correlated randomness:                                  │
│     r_0, r_1, ..., r_{n-1} such that Σ r_j * B^j = 0               │
│                                                                      │
│  3. Encrypt each player's share:                                     │
│     For player i with encryption key ek_i:                          │
│       C_{i,j} = G * z_j + ek_i * r_j                                │
│       R_j = H * r_j                                                 │
│                                                                      │
│  4. Output: ({C_{i,j}}, {R_j}) for all i, j                        │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      VALIDATOR i (Recipient)                         │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  Input: Ciphertext ({C_{i,j}}, {R_j}), decryption key dk_i          │
│                                                                      │
│  1. Cancel randomness from ciphertext:                              │
│     D_{i,j} = C_{i,j} - dk_i * R_j = G * z_j                        │
│                                                                      │
│  2. Recover each chunk via discrete log (BSGS):                     │
│     z_j = dlog_G(D_{i,j})  where z_j ∈ [0, B)                       │
│                                                                      │
│  3. Reconstruct the scalar:                                          │
│     msk_i = Σ_j z_j * B^j                                           │
│                                                                      │
│  Output: msk_i (validator's secret share of the master secret)      │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

## Integration with BIBE

**File:** `crates/aptos-batch-encryption/src/schemes/fptx.rs`

The decrypted scalars from chunked ElGamal directly become BIBE master secret key shares:

```rust
let msk_share = Self::MasterSecretKeyShare {
    mpk_g2,
    player: current_player,
    shamir_share_eval: subtranscript
        .decrypt_own_share(...)  // ← Returns DealtSecretKeyShare (G₁ element)
        .0                       // ← Extract the group element
        .into_fr(),              // ← Convert to scalar field element
        //                       ^^^ This scalar IS the BIBE MSK share!
};
```

## Security Considerations

1. **Chunk Size (`ell`):** Must balance BSGS table size vs. number of chunks
   - Larger `ell` = fewer chunks = smaller ciphertexts
   - Smaller `ell` = easier discrete logs (smaller BSGS table)

2. **BSGS Table:** Pre-computed for `2^{ell/2}` entries per chunk
   - Memory: O(2^{ell/2}) group elements
   - For `ell = 16`: 2^8 = 256 entries per chunk, 16 chunks = ~4K entries

3. **Correlated Randomness:** The constraint `Σ r_j * B^j = 0` ensures:
   - Decryption correctness: randomness cancels out
   - Still provides IND-CPA security (randomness is unpredictable)

## Simplified API for Lean Implementation

For a minimal distributed IBE scalar production system:

```rust
// Core functions needed:
// 1. chunk_scalar(scalar: &F, ell: u8) -> Vec<F>
//    - Split scalar into ell-bit chunks (little-endian)

// 2. generate_correlated_randomness(rng, radix: u64, num_chunks: u32) -> Vec<F>
//    - Generate r_0..r_{n-1} with Σ r_j * B^j = 0

// 3. encrypt_chunks(chunks: &[F], randomness: &[F], ek: &G1Affine, pp: &PublicParameters) -> (Vec<G1>, Vec<G1>)
//    - Compute (C_j, R_j) for each chunk j

// 4. decrypt_chunks(C: &[G1], R: &[G1], dk: &Scalar, pp: &PublicParameters, table: &BSGSTable) -> Vec<F>
//    - Compute C_j - dk * R_j, then BSGS to recover chunks

// 5. reconstruct_scalar(chunks: &[F], ell: u8) -> F
//    - Compute Σ chunk_j * (2^ell)^j
```

## References

- Original PVSS paper: "Simple and Efficient Threshold Distributed Key Generation"
- Chunked ElGamal: Enables range proofs on encrypted scalars
- BSGS (Baby-Step Giant-Step): Algorithm for computing discrete logs in bounded range
