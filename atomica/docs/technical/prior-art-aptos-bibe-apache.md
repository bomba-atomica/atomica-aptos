# Prior Art: Aptos Batch IBE (FPTX) & DKG Analysis

**Sources:**

- [aptos-labs/aptos-core](https://github.com/aptos-labs/aptos-core) (main branch)
- [aptos-labs/aptos-core](https://github.com/aptos-labs/aptos-core/tree/0e47c90c1c2d64aa00996575c6d53d2513088886) (ZapatOS Apache 2.0 branch)
  **Crates:** `crates/aptos-batch-encryption`, `aptos-dkg`, `aptos-crypto`

The upstream Aptos Core repository contains an implementation of an encrypted mempool solution based on **Batch Identity-Based Encryption (BIBE)**. This document analyzes the cryptographic architecture and implementation, while security code review findings are documented separately in `code-review/2026-01-06-timelock-cryptography-review.md`.

## 1. Cryptographic Architecture

The implementation relies on a committee-based threshold decryption model rather than a time-lock model.

### Core Primitives

| Primitive            | Description                                                                                                  |
| -------------------- | ------------------------------------------------------------------------------------------------------------ |
| **Scheme**           | Batch Identity-Based Encryption (BIBE), a variant of Boneh-Franklin IBE                                      |
| **Curve (main)**     | BLS12-381 (via `ark-bls12-381`)                                                                              |
| **Curve (timelock)** | BN254 (via `ark-bn254`) - 33% smaller G2 elements, ~2x faster pairings                                       |
| **Trust Model**      | Weighted Publicly Verifiable Secret Sharing (PVSS)                                                           |
| **Identity**         | The IBE "Identity" is the **Batch Digest** (a cryptographic commitment to the ordered block of transactions) |

### Curve Selection Rationale

| Aspect          | BLS12-381 (Main) | BN254 (Timelock)    |
| --------------- | ---------------- | ------------------- |
| G2 element size | 96 bytes         | 64 bytes            |
| Pairing speed   | Baseline         | ~2x faster          |
| Security level  | 128-bit          | ~100-bit            |
| Primary use     | Production IBE   | Timelock encryption |

The BN254 variant in the timelock branch prioritizes **performance and smaller ciphertexts** over the marginal security gain, which is acceptable for timelock encryption where keys are ephemeral per epoch.

### Protocol Flow

#### Phase 1: Trusted Setup (DKG)

Validators execute a Distributed Key Generation protocol to establish the system parameters.

- **Mechanism:** Weighted PVSS (Shamir Secret Sharing over `Fr`)
- **Output:**
  - **Master Secret Key (MSK):** A random scalar $s$ distributed among validators. No single entity holds $s$.
  - **Master Public Key (MPK):** A group element $P_{pub} \in G_2$. This is public.
- **Weighting:** Support for `FPTXWeighted` allows validators to hold shares proportional to their consensus stake using virtualized players.

#### Phase 2: Client Encryption

Clients submit transactions using a Hybrid Key Encapsulation Mechanism (KEM).

1. **Symmetric Key Generation:** An ephemeral symmetric key $K$ (AES-128-GCM) is generated.
2. **Payload Encryption:** The transaction body is encrypted using $K$.
3. **Key Encapsulation (IBE):**
   - The client encrypts $K$ using the MPK.
   - The encryption targets a specific IBE Identity derived from the transaction context.
   - Ciphertext structure involves randomized group elements in $G_2$ and a pairing-based One-Time Pad (OTP) source.

#### Phase 3: Batch Digest & Consensus

Transactions are ordered by the consensus protocol.

- **KZG Commitments:** The system computes a KZG polynomial commitment over the set of transaction IDs in the block.
- **Batch Identity:** This commitment (the "Digest") serves as the single IBE Identity for the entire batch.
- **Evaluation Proofs:** Individual transaction inclusion proofs are generated to link specific ciphertexts to the Batch Digest.

#### Phase 4: Threshold Decryption

Once the batch is finalized and the Digest $D$ is known, the committee cooperates to decrypt.

1. **Share Derivation:** Each validator $i$ uses their secret share $s_i$ to compute a partial decryption key (a BLS signature share) on the Digest $D$:
   $$ \sigma_i = s_i \cdot (D + H(MPK)) \in G_1 $$

2. **Aggregation:** A leader collects a threshold of shares (weighted by stake).

3. **Reconstruction:** Using **Fast Lagrange Interpolation**, the shares are aggregated to reconstruct the master decryption key $DK$ for that specific batch:
   $$ DK = s \cdot (D + H(MPK)) $$

4. **Batch Decryption:** The single key $DK$ is used to decrypt all transactions in the batch in parallel. The pairings check $e(DK, C_{tx})$ recovers the symmetric key for each transaction.

## 2. Implementation Details

### Libraries Used

The implementation heavily utilizes the **Rust Crypto** and **Arkworks** ecosystems:

| Library         | Purpose                                                      |
| --------------- | ------------------------------------------------------------ |
| `ark-bls12-381` | Elliptic curve arithmetic (BLS main branch)                  |
| `ark-bn254`     | BN254 pairing (timelock branch)                              |
| `ark-ec`        | Elliptic curve operations and pairings                       |
| `ark-ff`        | Finite field arithmetic                                      |
| `ark-poly`      | Polynomial arithmetic for KZG commitments and Shamir sharing |
| `ark-serialize` | Canonical serialization for cryptographic objects            |
| `aptos-dkg`     | Provides the underlying PVSS traits                          |
| `aptos-crypto`  | BLS keys, Shamir secret sharing, threshold configs           |
| `aes-gcm`       | AES-128-GCM symmetric encryption                             |
| `rayon`         | Parallel processing of batch decryption                      |
| `sha2`          | Hash-to-field operations                                     |
| `ed25519-dalek` | Identity derivation (verification keys)                      |

### Key Structures

```rust
// Master secret key share - distributed to validators
pub struct BIBEMasterSecretKeyShare {
    pub(crate) mpk_g2: G2Affine,      // Public key component
    pub(crate) player: Player,        // Validator identifier
    pub(crate) shamir_share_eval: Fr, // Scalar share (private)
}

// Decryption key share derived from master secret key share
pub struct BIBEDecryptionKeyShareValue {
    pub(crate) signature_share_eval: G1Affine,
}

pub type BIBEDecryptionKeyShare = (Player, BIBEDecryptionKeyShareValue);

// Reconstructed full decryption key
pub struct BIBEDecryptionKey {
    pub(crate) signature_g1: G1Affine,
}
```

### Ciphertext Structure

```rust
pub struct BIBECiphertext<I: Id> {
    pub(crate) id: I,                    // Transaction ID
    pub(crate) round: u64,               // Encryption round
    pub(crate) ct_g2: [G2Affine; 3],     // G2 elements for pairing
    pub(crate) ct_g1: G1Affine,          // G1 element for symmetric key
    pub(crate) symmetric_ct: Vec<u8>,    // AES-128-GCM encrypted payload
}

// Encryption formula:
ct_g2[0] = g2 * r[0] + sig_mpk_g2 * r[1]
ct_g2[1] = (g2 * id.x() - tau_g2) * r[0]
ct_g2[2] = -g2 * r[1]
```

### Key Derivation Logic

The logic in `key_derivation.rs` demonstrates that the Decryption Key is mathematically equivalent to a **BLS Signature** on the Batch Digest.

- **Verification:** Verification keys ($VK_i = s_i \cdot G_2$) allow anyone to verify that a validator's partial share is correct before aggregation.
- **Reconstruction:** The `reconstruct` function implements an optimized algorithm for computing Lagrange coefficients to combine shares over a large domain ($O(N \log^2 N)$ complexity).

## 3. DKG Integration

### DKG and BIBE Relationship

DKG and BIBE are complementary systems for different cryptographic purposes:

| Aspect        | BIBE                      | DKG                                       |
| ------------- | ------------------------- | ----------------------------------------- |
| **Purpose**   | Timelock encryption       | Distributed key generation                |
| **Algorithm** | Shamir Secret Sharing     | PVSS (Publicly Verifiable Secret Sharing) |
| **Curve**     | BN254 or BLS12-381        | BLS12-381                                 |
| **Output**    | Threshold decryption keys | Threshold keys for validators             |

### Key Conversion Flow

```
Validator's BLS12-381 Key (secure storage)
            │
            ▼
    ┌───────────────────────┐
    │  maybe_dk_from_bls_sk │  ◄── Convert BLS SK to DKG decrypt key
    │  (reverse bytes)      │      by reversing the key bytes
    └───────────────────────┘
            │
            ▼
    ┌───────────────────────┐
    │  DKG PVSS Protocol    │  ◄── BLS12-381 based distributed key gen
    │  (WeightedTranscript) │
    └───────────────────────┘
            │
            ▼
    ┌───────────────────────┐
    │  decrypt_own_share()  │  ◄── Each validator decrypts their share
    │  Returns: Scalar      │      using their DKG decrypt key
    └───────────────────────┘
            │
            ▼
    ┌───────────────────────┐
    │  BIBE setup()         │  ◄── Convert BLS scalar to BN254 Fr
    │  .into_fr()           │      (timelock branch only)
    └───────────────────────┘
            │
            ▼
    ┌───────────────────────┐
    │  BIBEMasterSecretKeyShare
    │  (stored in memory only)
    └───────────────────────┘
```

### Production Setup Function

```rust
fn setup(
    digest_key: &Self::DigestKey,
    pvss_public_params: &<Self::SubTranscript as Subtranscript>::PublicParameters,
    subtranscript_happypath: &Self::SubTranscript,
    subtranscript_slowpath: &Self::SubTranscript,
    tc_happypath: &Self::ThresholdConfig,
    tc_slowpath: &Self::ThresholdConfig,
    current_player: Player,
    sk_share_decryption_key: &<Self::SubTranscript as Subtranscript>::DecryptPrivKey,
) -> Result<(...)>;
```

## 4. Key Storage and Recovery

### Key Persistence Matrix

| Component       | Storage                   | Curve     | Persistence              |
| --------------- | ------------------------- | --------- | ------------------------ |
| BLS Private Key | `PersistentSafetyStorage` | BLS12-381 | ✅ Persisted (encrypted) |
| DKG Transcript  | On-chain (DKGState)       | BLS12-381 | ✅ Persisted             |
| BN254 Msk Share | **In-memory only**        | BN254     | ❌ Lost on restart       |

### Ephemeral Key Design

The BN254 master secret key shares exist only in memory and are regenerated each epoch from the DKG state. This is a deliberate security design choice:

1. **Ephemeral per-epoch keys**: BN254 keys are only valid for one epoch
2. **Fresh randomness**: Each epoch's DKG uses new randomness (`InputSecret::generate`)
3. **Reduced attack surface**: No long-term BN254 key material to protect
4. **BLS security for DKG**: The 128-bit security of BLS protects the DKG process

### Recovery Flow After Restart

```
Validator Process Starts
          │
          ▼
┌─────────────────────────────────────┐
│  1. Load BLS private key from       │  ◄── From secure storage/HSM
│     PersistentSafetyStorage         │
└─────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────┐
│  2. Fetch DKGState from on-chain    │  ◄── Stored on-chain as Move resource
│     (in_progress session)           │
└─────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────┐
│  3. If in_progress session exists   │  ◄── Resume DKG protocol if incomplete
│     and matches current epoch       │
└─────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────┐
│  4. Decrypt share from transcript   │  ◄── Uses BLS private key
└─────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────┐
│  5. Convert BLS scalar to BN254     │  ◄── .into_fr()
│     Regenerate BIBE keys in memory  │
└─────────────────────────────────────┘
```

### DKG State Persistence

The `DKGSessionState` contains:

- **Serialized transcript**: Stored on-chain in `0x1::dkg::DKGState` Move resource
- **Metadata**: Epoch, dealer info, start time
- Retrieved after restart and decrypted using BLS private key

## 5. Cryptographic Properties

### Boneh-Franklin Correctness

| Operation        | Formula                                                     | Status     |
| ---------------- | ----------------------------------------------------------- | ---------- |
| **Encryption**   | $U = r \cdot g_2$, $V = M \oplus H_2(e(Q_{ID}, P_{pub})^r)$ | ✅ Correct |
| **Decryption**   | $M = V \oplus H_2(e(d_{ID}, U))$                            | ✅ Correct |
| **Verification** | $e(d_{ID}, g_2) = e(Q_{ID}, P_{pub})$                       | ✅ Correct |

### Security Properties

| Property               | Status            | Notes                                    |
| ---------------------- | ----------------- | ---------------------------------------- |
| **IND-CPA**            | ✅ Secure         | Random $r$ ensures different ciphertexts |
| **Threshold security** | ✅ Secure         | T-of-N prevents single point of failure  |
| **Forward security**   | ✅ Ephemeral keys | Fresh keys each epoch                    |

## 6. Summary of Capabilities

- **Stake-Weighted Security:** The decryption threshold is defined by validator stake, not just node count.
- **Parallelism:** Decryption is highly parallelizable once the batch key is recovered.
- **Atomic Batching:** The use of a KZG Digest ensures that the committee decrypts either the _entire_ batch or _nothing_. They cannot selectively decrypt individual transactions within a batch without deriving the batch key.
- **Performance:** BN254 variant provides ~2x faster pairings and 33% smaller ciphertexts than BLS12-381.
- **Ephemeral Keys:** BN254 keys exist only in memory, regenerated each epoch from DKG state.

## 7. Related Documents

- **Security Code Review**: `code-review/2026-01-06-timelock-cryptography-review.md`
- **Cross-Language Tests**: `timelock-tests/test/cross-lang-ibe-verification.test.ts`
- **Golden Vectors**: `golden-vectors/verify.ts`
- **Drand Integration**: `docs/technical/prior-art-drand.md`
