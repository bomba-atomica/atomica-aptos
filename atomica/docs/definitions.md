# Atomica Definitions

This document defines the core terminology used in the Atomica timelock system, grounded in canonical cryptographic literature.

---

## Table of Contents

1. [Cryptographic Primitives](#cryptographic-primitives)
2. [Key Management](#key-management)
3. [Protocol Phases](#protocol-phases)
4. [Threshold Concepts](#threshold-concepts)
5. [System Components](#system-components)
6. [Data Structures](#data-structures)

---

## Cryptographic Primitives

### BLS12-381

A pairing-friendly elliptic curve used for all cryptographic operations in Atomica. Provides:

- **G1**: 48-byte elliptic curve group (generator G1)
- **G2**: 96-byte elliptic curve group (generator G2)
- **Gt**: Target group of order r (576 bytes when serialized)

The pairing function `e: G1 × G2 → Gt` enables the cryptographic operations below.

### Boneh-Franklin Identity-Based Encryption (IBE)

A public-key encryption scheme where the public key is an identity string rather than a random value. Defined by the paper ["Identity-Based Encryption from the Weil Pairing" (Boneh & Franklin, 2001)](https://crypto.stanford.edu/~dabo/papers/bfibe.pdf).

**Key Properties:**

- No need for certificates or public key infrastructure
- Public key derived from identity (e.g., email, or in our case, timelock parameters)
- Master secret key held by a trusted authority (in our case, distributed via DKG)

**Four Algorithms:**

1. `Setup(λ) → (msk, mpk)` - Generates master secret and public parameters
2. `Extract(msk, id) → sk_id` - Derives private key for an identity
3. `Encrypt(mpk, id, m) → CT` - Encrypts message for an identity
4. `Decrypt(sk_id, CT) → m` - Decrypts using identity-based private key

### Publicly Verifiable Secret Sharing (PVSS)

A secret sharing scheme where:

- Each share is encrypted for a specific recipient
- Anyone can verify shares are correctly formed without knowing the secret
- Used to distribute encrypted shares to validators

Reference: ["Distributed Key Generation for Distributed Cryptography" (Gennaro et al.)](https://link.springer.com/article/10.1007/s00145-019-09332-0)

---

## Key Management

### Master Public Key (MPK)

The public key of the IBE system, generated via DKG and published on-chain. All encryption operations use this key.

- **Type**: G2 point (96 bytes)
- **Published by**: Validators collectively via DKG
- **Used for**: Encrypting messages for a given identity

### Master Secret Key (MSK)

The secret key from which all decryption keys are derived. Never reconstructed or stored on any single machine.

- **Distribution**: Split into shares via DKG
- **Threshold**: Requires 2/3+ of validator shares to reconstruct

### Decryption Key (DK)

The private key corresponding to a specific identity (timelock_id + deadline). Derived as:

```
DK = s × Q_id
where:
  s = master secret (from DKG)
  Q_id = HashToCurve_G1(identity)
```

- **Type**: G1 point (48 bytes)
- **Revealed**: After deadline timestamp, via threshold reconstruction

### Decryption Key Share (DK_share)

A portion of the decryption key held by each validator. Each validator receives one share during DKG.

- **Type**: G1 point
- **Stored**: Locally by each validator in persistent storage
- **Revealed**: Submitted on-chain after deadline

### Identity

The public identifier for IBE encryption, computed as:

```
identity = Keccak256("timelock_id:" || timelock_id || ":deadline_timestamp_microseconds:" || deadline)
```

- **Format**: 32-byte hash
- **One-to-one**: Each (timelock_id, deadline) pair has a unique identity
- **Application-agnostic**: Contains no auction or application semantics

---

## Protocol Phases

### Setup

The initial configuration phase:

1. DKG ceremony executed by validators
2. MSK split into shares using PVSS
3. Each validator receives encrypted share
4. MPK computed and published on-chain

### Encryption

Any party can encrypt without validator involvement:

1. Compute identity from timelock_id and deadline
2. Fetch MPK from blockchain
3. Encrypt message using IBE.Encrypt
4. Store ciphertext (U, V) off-chain or on-chain

### Decryption (Reveal)

After deadline passes:

1. Validators submit their DK_shares on-chain
2. Contract aggregates shares until threshold met
3. DK reconstructed when threshold exceeded
4. Parties with ciphertexts decrypt using DK

---

## Threshold Concepts

### Threshold (t)

The minimum number of decryption key shares required to reconstruct the full decryption key.

- **Default**: 2/3 + 1 of validator voting power
- **Rationale**: Ensures liveness while maintaining security against minority collusion

### Validator Weight (w)

The voting power assigned to each validator, determining their influence in threshold cryptography.

- **Weight assignment**: Based on stake (from consensus)
- **Share count**: Validators with higher weight receive more shares
- **Total weight**: Sum of all validator weights

### Share

A portion of the master secret key distributed to one validator.

- **One-to-one**: Each validator receives exactly one share
- **Weight-based**: Share is weighted by the validator's stake
- **Encrypted**: Transit-encrypted to prevent interception

### Threshold Cryptography

Cryptographic techniques requiring cooperation among multiple parties to perform operations.

**Properties:**

- **Liveness**: Any t+1 honest validators can reconstruct
- **Security**: Fewer than t shares reveal no information about secret
- **No trusted dealer**: Secret generated collectively, not by any single party

### BLS Threshold Cryptography

Using BLS signatures in a threshold setting. Reference: ["Threshold BLS Signatures" (Boldyreva, 2003)](https://link.springer.com/article/10.1007/s00145-003-0125-6)

**Key properties for Atomica:**

- Same BLS key material used for consensus can be adapted for IBE
- Shares can be combined to produce valid decryption keys
- Verifiable: Anyone can verify shares are valid without knowing the secret

---

## System Components

### Distributed Key Generation (DKG)

The protocol by which validators collectively generate the master secret key.

**Phases:**

1. **Deal**: Each validator generates random secret, creates shares for others
2. **Verify**: Validators verify received shares
3. **Broadcast**: Validators broadcast commitments and encrypted shares
4. **Aggregate**: Combine valid shares into final transcript
5. **Publish**: Submit aggregated transcript on-chain

**Output:**

- MPK: Public key for encryption
- DK_share: Each validator's private share

### Epoch

The Aptos consensus time unit. Each epoch has:

- A validator set
- A block proposer
- A DKG session (for randomness or timelock)

### Deadline

The timestamp (in microseconds) when decryption becomes possible. Unique to each timelock.

- **Format**: Unix epoch microseconds (u64)
- **Alignment**: Must align to checkpoint period
- **Irrevocable**: Cannot be changed after encryption

### Timelock ID

A unique identifier for a timelock registration.

- **One-to-one with deadline**: Each timelock_id has exactly one deadline
- **Assigned on registration**: When `register_timelock()` is called

---

## Data Structures

### DKG Transcript

The complete record of a DKG ceremony, published on-chain.

**Contents:**

- Metadata (epoch, author)
- BCS-serialized transcript bytes (WeightedTranscript)
- PVSS commitments and encrypted shares

**Types:**

- `DKGTranscript`: On-chain transaction format
- `WeightedTranscript`: PVSS transcript with weights
- `Transcripts`: Main transcript + optional fast-path transcript

### Rounding Profile

Configuration for weighted threshold cryptography.

**Fields:**

- `validator_weights`: Computed weight for each validator
- `secrecy_threshold_in_stake_ratio`: Minimum stake to keep secret
- `reconstruct_threshold_in_stake_ratio`: Minimum stake to reconstruct
- `reconstruct_threshold_in_weights`: Number of weights needed to reconstruct

### Session

A DKG execution context, identified by epoch.

**States:**

- `NotStarted`: Session not yet begun
- `InProgress`: DKG dealing and aggregation ongoing
- `Finished`: Transcript submitted, awaiting chain acceptance

---

## Algorithm Reference

### IBE.Encrypt

```
Input:   MPK (G2), identity (32 bytes), message M
Output:  Ciphertext (U: G2, V: bytes)

1. r ← random scalar in [1, r-1]
2. U ← r × G2_generator
3. Q_id ← HashToCurve_G1(identity)
4. g_id ← e(Q_id, MPK)^r
5. K ← Keccak256(g_id_as_bytes)[0:32]
6. V ← M XOR K
7. return (U, V)
```

### IBE.Decrypt

```
Input:   DK (G1), identity (32 bytes), Ciphertext (U, V)
Output:  message M

1. g_id ← e(DK, U)
2. K ← Keccak256(g_id_as_bytes)[0:32]
3. M ← V XOR K
4. return M
```

### DK Derivation

```
Input:   MSK s (scalar), identity (32 bytes)
Output:  DK (G1 point)

1. Q_id ← HashToCurve_G1(identity)
2. DK ← s × Q_id
3. return DK
```

---

## Relationship Diagram

```
                                    ┌─────────────────────────┐
                                    │      Validator Set      │
                                    │   (n validators)        │
                                    │                         │
            ┌───────────────────────┤ BLS public keys (PK_i) │
            │                       │ (for encrypting shares)│
            │                       └───────────┬─────────────┘
            │                                       │
            │                       ┌───────────────┼───────────────┐
            │                       │               │               │
            │           ┌───────────▼───┐   ┌───────▼─────┐   ┌───────▼─────┐
            │           │ Validator 1   │   │ Validator 2 │   │ Validator n │
            │           │ BLS SK (SK_1) │   │ BLS SK      │   │ BLS SK      │
            │           └───────┬───────┘   └──────┬──────┘   └──────┬──────┘
            │                   │                 │                  │
            │                   └─────────────────┼──────────────────┘
            │                                     │
            │                                     ▼
            │                       ┌───────────────────────────┐
            │                       │      DKG Ceremony         │
            │                       │   (PVSS + Encryption)     │
            │                       │                           │
            │                       │   Input:                  │
            │                       │   - Validator BLS PKs     │
            │                       │   - Validator BLS SKs     │
            │                       │   - Threshold config (t)  │
            │                       │                           │
            │                       │   Output:                 │
            │                       │   - MPK (published)       │
            │                       │   - DK_share_i (private)  │
            │                       └─────────────┬─────────────┘
            │                                     │
            │        ┌────────────────────────────┼────────────────────────────┐
            │        │                            │                            │
┌───────────▼────┐ ┌─▼──────────────┐ ┌───────────▼──────────┐ ┌───────────────▼──────────────┐
│ MPK (G2)       │ │ DK_share_1 (G1)│ │ DK_share_2 (G1)      │ │ DK_share_n (G1)              │
│ Published      │ │ Stored locally │ │ Stored locally       │ │ Stored locally               │
│ On-chain       │ │ by Validator 1 │ │ by Validator 2       │ │ by Validator n               │
└────────┬───────┘ └───────┬────────┘ └──────────┬──────────┘ └───────────────┬──────────────┘
         │                 │                      │                            │
         │                 └──────────────────────┼────────────────────────────┘
         │                                            │
         │                              ┌─────────────┴─────────────┐
         │                              │                           │
         │                 ┌────────────▼────────────┐  ┌─────────▼────────────┐
         │                 │  Threshold Reconstruct  │  │  DK_share submission │
         │                 │  (t+1 shares required)  │  │  (after deadline)    │
         │                 │                         │  │                      │
         │                 │  Combines shares to     │  │  Validators submit   │
         │                 │  produce DK             │  │  shares on-chain     │
         │                 └────────────┬────────────┘  └──────────┬───────────┘
         │                              │                          │
         │                              │                          │
         │                              ▼                          │
         │                    ┌────────────────────┐              │
         │                    │ DK (G1)            │              │
         │                    │ Decryption Key     │              │
         │                    │ Revealed on-chain  │              │
         │                    └────────────────────┘              │
         │                              │                          │
         │                              │                          │
┌────────▼────────────────┐ ┌───────────▼──────────────────┐ ┌─────▼─────────────────────────┐
│ IBE.Encrypt             │ │ IBE.Decrypt                  │ │ On-chain Share Aggregation   │
│ (Anyone can encrypt)    │ │ (After deadline, DK known)   │ │ ValidatorTransaction::       │
│                         │ │                              │ │ TimelockShare                │
│ Input:                  │ │ Input:                       │ │                              │
│ - MPK                   │ │ - DK (from reconstruction)   │ │ Validators submit DK_shares  │
│ - Identity              │ │ - Ciphertext (U, V)          │ │ Contract aggregates until    │
│ - Message               │ │ - Identity                   │ │ threshold met                │
│                         │ │                              │ │                              │
│ Output:                 │ │ Output:                      │ └──────────────────────────────┘
│ Ciphertext (U, V)       │ │ Message M                    │              │
└─────────────────────────┘ └──────────────────────────────┘              │
                                                                         │
                                                            ┌─────────────▼─────────────┐
                                                            │ Threshold Reconstruct     │
                                                            │ (t+1 shares)              │
                                                            │ DK revealed on-chain      │
                                                            └───────────────────────────┘
```

**Key Observations:**

1. **MSK is not stored anywhere** - it's conceptually distributed as shares from the start
2. **DK_shares are input AND output** of DKG - each validator receives their share
3. **BLS keys serve dual purposes** - consensus AND encrypting DKG shares
4. **DK is reconstructed only after deadline** - fromDK_shares, never from MSK

---

## References

1. Boneh, D., & Franklin, M. (2001). Identity-Based Encryption from the Weil Pairing. CRYPTO 2001.
2. Gennaro, R., Jarecki, S., Krawczyk, H., & Rabin, T. (1999). Secure Distributed Key Generation for Discrete-Log Based Cryptosystems. Journal of Cryptology.
3. Boldyreva, A. (2003). Threshold Signatures, Multisignatures and Blind Signatures Based on the Gap-Diffie-Hellman-Group Signing Scheme. PKC 2003.
4. Boneh, D., Gentry, C., & Waters, B. (2005). Collusion Resistant Broadcast Encryption with Ciphertext Size O(1). CRYPTO 2005.
5. Apache Aptos Framework Documentation - DKG and Validator Transaction modules.

---

## Change Log

| Version | Date       | Description     |
| ------- | ---------- | --------------- |
| 1.0     | 2026-01-07 | Initial version |
