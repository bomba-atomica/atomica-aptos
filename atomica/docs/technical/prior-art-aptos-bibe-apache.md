# Prior Art: Aptos Batch IBE (FPTX) & DKG Analysis

**Sources:**

- [aptos-labs/aptos-core](https://github.com/aptos-labs/aptos-core) (main branch)
- [aptos-labs/aptos-core](https://github.com/aptos-labs/aptos-core/tree/0e47c90c1c2d64aa00996575c6d53d2513088886) (ZapatOS Apache 2.0 branch)
  **Crates:** `crates/aptos-batch-encryption`, `aptos-dkg`, `aptos-crypto`

The upstream Aptos Core repository contains an implementation of an encrypted mempool solution based on **Batch Identity-Based Encryption (BIBE)**. This document analyzes the cryptographic architecture and implementation.

## 1. Cryptographic Architecture

The implementation relies on a committee-based threshold decryption model rather than a time-lock model.

### Core Primitives

| Primitive        | Description                                                                                                  |
| ---------------- | ------------------------------------------------------------------------------------------------------------ |
| **Scheme**       | Batch Identity-Based Encryption (BIBE), a variant of Boneh-Franklin IBE                                      |
| **Curve (FPTX)** | BN254 (via `ark_bn254::Bn254`) - NOT BLS12-381                                                               |
| **Curve (DKG)**  | BLS12-381 (via `aptos-dkg` PVSS)                                                                             |
| **Trust Model**  | Weighted Publicly Verifiable Secret Sharing (PVSS)                                                           |
| **Identity**     | The IBE "Identity" is the **Batch Digest** (a cryptographic commitment to the ordered block of transactions) |

### Important: BN254 for FPTX, BLS12-381 for DKG

**The main branch uses BN254 for the FPTX BIBE scheme itself**, not BLS12-381. This is confirmed in `group.rs`:

```rust
// group.rs explicitly uses BN254
pub use ark_bn254::{
    g1::Config as G1Config, Bn254 as PairingSetting, Config, Fq, Fr, G1Affine, G1Projective,
    G2Affine, G2Projective,
};
```

BLS12-381 is only used by the DKG layer (`aptos-dkg`) which produces the threshold shares. The DKG output is then converted for use with BN254 in the BIBE scheme.

### Curve Selection Rationale

| Aspect          | BN254 (FPTX BIBE)          | BLS12-381 (DKG)         |
| --------------- | -------------------------- | ----------------------- |
| G2 element size | 64 bytes                   | 96 bytes                |
| Pairing speed   | ~2x faster                 | Baseline                |
| Security level  | ~100-bit                   | 128-bit                 |
| Primary use     | BIBE encryption/decryption | PVSS/DKG key generation |

### Variants

| Scheme           | Description                         | Threshold Config             |
| ---------------- | ----------------------------------- | ---------------------------- |
| **FPTX**         | Unweighted threshold encryption     | `ShamirThresholdConfig<Fr>`  |
| **FPTXWeighted** | Stake-weighted threshold encryption | `WeightedConfigArkworks<Fr>` |

The `FPTXWeighted` variant supports validators holding shares proportional to their consensus stake using virtualized players.

### Protocol Flow

#### Phase 1: Trusted Setup (DKG)

Validators execute a Distributed Key Generation protocol to establish the system parameters.

- **Mechanism:** Weighted PVSS (Shamir Secret Sharing over `Fr`)
- **Output:**
  - **Master Secret Key (MSK):** A random scalar $s$ distributed among validators. No single entity holds $s$.
  - **Master Public Key (MPK):** A group element $P_{pub} \in G_2$. This is public.
- **Weighting:** `FPTXWeighted` allows validators to hold shares proportional to their consensus stake using virtualized players.

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

2. **Aggregation:** A leader collects a threshold of shares (weighted by stake for `FPTXWeighted`).

3. **Reconstruction:** Using **Fast Lagrange Interpolation**, the shares are aggregated to reconstruct the master decryption key $DK$ for that specific batch:
   $$ DK = s \cdot (D + H(MPK)) $$

4. **Batch Decryption:** The single key $DK$ is used to decrypt all transactions in the batch in parallel. The pairings check $e(DK, C_{tx})$ recovers the symmetric key for each transaction.

## 2. Implementation Details

### Libraries Used

The implementation heavily utilizes the **Rust Crypto** and **Arkworks** ecosystems:

| Library         | Purpose                                                      |
| --------------- | ------------------------------------------------------------ |
| `ark-bn254`     | BN254 pairing for FPTX BIBE operations                       |
| `ark-bls12-381` | BLS12-381 for DKG PVSS operations                            |
| `ark-ec`        | Elliptic curve operations and pairings                       |
| `ark-ff`        | Finite field arithmetic                                      |
| `ark-poly`      | Polynomial arithmetic for KZG commitments and Shamir sharing |
| `ark-serialize` | Canonical serialization for cryptographic objects            |
| `aptos-dkg`     | Provides the underlying PVSS traits (`SubTranscript`, etc.)  |
| `aptos-crypto`  | Shamir secret sharing, threshold configs, weighted configs   |
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

### Encryption Key Structure

```rust
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EncryptionKey {
    sig_mpk_g2: G2Affine,  // Master public key component
    tau_g2: G2Affine,      // KZG tau for digest generation
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

### Threshold Trait Implementation

```rust
impl BatchThresholdEncryption for FPTX {
    type Ciphertext = Ciphertext<FreeRootId>;
    type DecryptionKey = BIBEDecryptionKey;
    type DecryptionKeyShare = BIBEDecryptionKeyShare;
    type Digest = Digest;
    type DigestKey = DigestKey;
    type EncryptionKey = EncryptionKey;
    type EvalProof = EvalProof;
    type EvalProofs = EvalProofs<FreeRootIdSet<ComputedCoeffs>>;
    type EvalProofsPromise = EvalProofsPromise<FreeRootIdSet<ComputedCoeffs>>;
    type Id = FreeRootId;
    type MasterSecretKeyShare = BIBEMasterSecretKeyShare;
    type PreparedCiphertext = PreparedCiphertext;
    type Round = u64;
    type SubTranscript = aptos_dkg::pvss::chunky::UnweightedSubtranscript<Pairing>;
    type ThresholdConfig = aptos_crypto::arkworks::shamir::ShamirThresholdConfig<Fr>;
    type VerificationKey = BIBEVerificationKey;

    fn setup(...) -> Result<(...)> { ... }
    fn setup_for_testing(...) -> Result<(...)> { ... }
    fn encrypt(...) -> Result<Self::Ciphertext> { ... }
    fn digest(...) -> Result<(Self::Digest, Self::EvalProofsPromise)> { ... }
    fn derive_decryption_key_share(...) -> Result<Self::DecryptionKeyShare> { ... }
    fn reconstruct_decryption_key(...) -> Result<Self::DecryptionKey> { ... }
    fn decrypt(...) -> Result<Vec<P>> { ... }
    // ... additional trait methods
}
```

## 3. DKG Integration

### DKG and BIBE Relationship

DKG and BIBE are complementary systems for different cryptographic purposes:

| Aspect        | BIBE (FPTX)               | DKG                                       |
| ------------- | ------------------------- | ----------------------------------------- |
| **Purpose**   | Threshold encryption      | Distributed key generation                |
| **Algorithm** | Shamir Secret Sharing     | PVSS (Publicly Verifiable Secret Sharing) |
| **Curve**     | BN254                     | BLS12-381                                 |
| **Output**    | Threshold decryption keys | Threshold keys for validators             |

### Key Conversion Flow

```
Validator's BLS12-381 Key (secure storage)
            │
            ▼
    ┌───────────────────────┐
    │  maybe_dk_from_bls_sk │  ◄── Convert BLS SK to DKG decrypt key
    │  (reverse bytes)      │
    └───────────────────────┘
            │
            ▼
    ┌───────────────────────┐
    │  DKG PVSS Protocol    │  ◄── BLS12-381 based distributed key gen
    │  (WeightedTranscript) │      produces transcripts
    └───────────────────────┘
            │
            ▼
    ┌───────────────────────┐
    │  decrypt_own_share()  │  ◄── Each validator decrypts their share
    │  Returns: BLS Scalar  │      using their DKG decrypt key
    └───────────────────────┘
            │
            ▼
    ┌───────────────────────┐
    │  FPTX setup()         │  ◄── Convert BLS scalar to BN254 Fr
    │  .into_fr()           │      via `shamir_share_eval: subtranscript
    └───────────────────────┘            .decrypt_own_share(...).0.into_fr()`
            │
            ▼
    ┌───────────────────────┐
    │  BIBEMasterSecretKeyShare
    │  (BN254 Fr, stored in memory)
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
    msk_share_decryption_key: &<Self::SubTranscript as Subtranscript>::DecryptPrivKey,
) -> Result<(
    Self::EncryptionKey,
    Vec<Self::VerificationKey>,
    Self::MasterSecretKeyShare,
    Vec<Self::VerificationKey>,
    Self::MasterSecretKeyShare,
)> {
    // Verifies happy/slow path public keys match
    // Extracts MPK from transcript
    // Derives verification keys for each player
    // Decrypts own secret share from transcript
    // Verifies VK/MSK consistency
}
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

- **Stake-Weighted Security:** `FPTXWeighted` variant supports validator stake-proportional shares.
- **Parallelism:** Decryption is highly parallelizable once the batch key is recovered.
- **Atomic Batching:** The use of a KZG Digest ensures that the committee decrypts either the _entire_ batch or _nothing_. They cannot selectively decrypt individual transactions within a batch without deriving the batch key.
- **Performance:** BN254 provides ~2x faster pairings and 33% smaller ciphertexts than BLS12-381.
- **Ephemeral Keys:** BN254 keys exist only in memory, regenerated each epoch from DKG state.
- **Dual-Curve Design:** BN254 for BIBE operations, BLS12-381 for DKG key generation.

## 7. Related Documents

- **Security Code Review**: `code-review/2026-01-06-timelock-cryptography-review.md`
- **Cross-Language Tests**: `timelock-tests/test/cross-lang-ibe-verification.test.ts`
- **Golden Vectors**: `golden-vectors/verify.ts`
- **Drand Integration**: `docs/technical/prior-art-drand.md`
