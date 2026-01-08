# Upstream Analysis: Aptos Batch IBE (FPTX)

**Source:** [aptos-labs/aptos-core](https://github.com/aptos-labs/aptos-core)
**Crate:** `crates/aptos-batch-encryption`
**Scheme:** FPTX (Fast/Fair Parallel Threshold Encryption)

The upstream Aptos Core repository contains an active implementation of an encrypted mempool solution based on **Batch Identity-Based Encryption (BIBE)**. This system allows transactions to be encrypted before submission and decrypted only after consensus ordering, mitigating MEV (Maximal Extractable Value) and censorship.

## 1. Cryptographic Architecture

The implementation relies on a committee-based threshold decryption model rather than a time-lock model.

### Core Primitives

- **Scheme:** Batch Identity-Based Encryption (BIBE), a variant of Boneh-Franklin IBE.
- **Curve:** BLS12-381 (via the `arkworks` ecosystem).
- **Trust Model:** Weighted Publicly Verifiable Secret Sharing (PVSS).
- **Identity:** The IBE "Identity" is the **Batch Digest** (a cryptographic commitment to the ordered block of transactions).

### Protocol Flow

#### Phase 1: Trusted Setup (DKG)

Validators execute a Distributed Key Generation protocol to establish the system parameters.

- **Mechanism:** Weighted PVSS (Shamir Secret Sharing over `Fr`).
- **Output:**
  - **Master Secret Key (MSK):** A random scalar $s$ distributed among validators. No single entity holds $s$.
  - **Master Public Key (MPK):** A group element $P_{pub} \in G_2$. This is public.
- **Weighting:** Support for `FPTXWeighted` allows validators to hold shares proportional to their consensus stake using virtualized players.

#### Phase 2: Client Encryption

Clients submit transactions using a Hybrid Key Encapsulation Mechanism (KEM).

1.  **Symmetric Key Generation:** An ephemeral symmetric key $K$ (e.g., for AES-GCM or ChaCha20) is generated.
2.  **Payload Encryption:** The transaction body is encrypted using $K$.
3.  **Key Encapsulation (IBE):**
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

1.  **Share Derivation:** Each validator $i$ uses their secret share $s_i$ to compute a partial decryption key (a BLS signature share) on the Digest $D$.
    $$ \sigma_i = s_i \cdot (D + H(MPK)) \in G_1 $$
2.  **Aggregation:** A leader collects a threshold of shares (weighted by stake).
3.  **Reconstruction:** Using **Fast Lagrange Interpolation**, the shares are aggregated to reconstruct the master decryption key $DK$ for that specific batch.
    $$ DK = s \cdot (D + H(MPK)) $$
4.  **Batch Decryption:** The single key $DK$ is used to decrypt all transactions in the batch in parallel. The pairings check $e(DK, C_{tx})$ recovers the symmetric key for each transaction.

## 2. Implementation Details

### Libraries Used

The implementation heavily utilizes the **Rust Crypto** and **Arkworks** ecosystems:

- **`ark-bls12-381`**: Elliptic curve arithmetic.
- **`ark-poly`**: Polynomial arithmetic for KZG commitments and Shamir sharing.
- **`aptos-dkg`**: Provides the underlying PVSS traits.
- **`rayon`**: Used for parallel processing of batch decryption.

### Key Derivation Logic

The logic in `key_derivation.rs` demonstrates that the Decryption Key is mathematically equivalent to a **BLS Signature** on the Batch Digest.

- **Verification:** Verification keys ($VK_i = s_i \cdot G_2$) allow anyone to verify that a validator's partial share is correct before aggregation.
- **Reconstruction:** The `reconstruct` function in `shamir.rs` implements an optimized algorithm for computing Lagrange coefficients to combine shares over a large domain ($O(N \log^2 N)$ complexity).

### Encryption Scheme (FPTX)

The `FPTX` struct implements the `BatchThresholdEncryption` trait.

- **Inputs:** `EncryptionKey`, `Plaintext`, `AssociatedData`.
- **Outputs:** `Ciphertext` containing:
  - `id`: The transaction ID.
  - `ct_g2`: 3 elements in $G_2$ (the IBE header).
  - `padded_key`: The encapsulated symmetric key.
  - `symmetric_ciphertext`: The actual transaction payload.

## 3. Summary of Capabilities

- **Stake-Weighted Security:** The decryption threshold is defined by validator stake, not just node count.
- **Parallelism:** Decryption is highly parallelizable once the batch key is recovered.
- **Atomic Batching:** The use of a KZG Digest ensures that the committee decrypts either the _entire_ batch or _nothing_. They cannot selectively decrypt individual transactions within a batch without deriving the batch key.
