# Task: Fix Weighted G1 Reconstruction for Aptos IBE

### 1. Architecture Overview

The system is structured across three layers to balance security and transparency:

- **Core Cryptography (`aptos-dkg` crate)**: Contains the heavy-lifting logic for key generation, PVSS transcripts, and DK reconstruction. The `reconstruct_ibe_dk_from_g1_shares` function in Rust is where the actual math resides.
- **Native Bridge (`aptos-move`)**: Acts as a passthrough. It provides the Rust native function implementation that exposes `aptos-dkg` APIs to the Move VM, handling type conversion and gas charging.
- **Move Contracts (`aptos-framework`)**: Manages public state. It stores the Master Public Key (MPK), validator public key shares, and DKG transcripts. These are safe to provide publicly and are used to verify/reconstruct DKs.
- **Golden Vectors (Fixtures)**: We maintain a set of "ground truth" fixtures in both `ibe_golden_vector_fixtures.move` and Rust tests. These ensure that any changes to the math or API are verified against known-good keys and transcripts across all layers.

### 2. Context & Problem Statement

We are implementing Identity-Based Encryption (IBE) Decryption Key (DK) reconstruction. Following an audit, we moved from reconstructing the Master Secret Key (MSK) as a scalar (insecure) to reconstructing the DK directly as a G1 point: $DK = \sum (\text{Coeff}_i \times DK\_share_i)$.

The equal-weight cases work perfectly. However, **unequal-weight reconstruction is failing** in Move unit tests (Fixtures 215 and 2321).

### 2. The Technical Mismatch

The Aptos DKG framework handles weighted PVSS via a **"Virtual Player"** model (see `GenericWeighting` in `crates/aptos-dkg/src/pvss/weighted/generic_weighting.rs`). A validator with weight $w_i$ is treated as $w_i$ virtual players, each with its own scalar share $s_{i,j}$ and its own Lagrange coefficient $\lambda_{i,j}(0)$.

In the IBE implementation, a validator $i$ computes a **single aggregated DK share**:
$$DK\_share_i = \left(\sum_{j=1}^{w_i} s_{i,j}\right) \times H(\text{identity})$$

My current implementation in `reconstruct_ibe_dk_from_g1_shares` attempts to use a simplified weighting:
$$\lambda_{weighted} = \lambda_i \times \frac{weight_i}{\sum weight_{participating}}$$
This is mathematically incorrect for the Virtual Player model. Because the Lagrange coefficients $\lambda_{i,j}(0)$ are different for each virtual player (as they are evaluated at different roots of unity), we cannot simply multiply the aggregated G1 point by a single scalar.

### 3. Affected Files

- **Logic:** `crates/aptos-dkg/src/ibe/mod.rs` -> `reconstruct_ibe_dk_from_g1_shares`
- **Tests (Debugging):** `crates/aptos-dkg/src/ibe/tests.rs` -> `test_compare_scalar_and_g1_reconstruction` (Currently failing to compile due to import/type mismatches).
- **Native Wrapper:** `aptos-move/framework/src/natives/cryptography/algebra/ibe.rs`
- **Move Fixtures:** `aptos-move/framework/aptos-framework/tests/ibe_native_test.move`

### 4. Required Solution

1.  **Reconcile the Math:** You need to derive the correct way to reconstruct on G1 when the input is an aggregated share per validator, but the underlying secret sharing uses virtual players.
    - _Hint:_ If the validator only provides $\sum s_{i,j}$, the reconstruction might only be possible if the DKG was initialized in a way where virtual player shares are compatible, or if the reconstruction sum uses the correct summation of virtual Lagrange coefficients: $C_i = \dots$
2.  **Fix the Comparison Test:** Finish the implementation of `test_compare_scalar_and_g1_reconstruction` in `ibe/tests.rs`. This test compares the framework's (working) scalar reconstruction against our (broken) G1 reconstruction. This is the "ground truth" for debugging.
3.  **Validate via Move:** Ensure all 14 tests in `ibe_native_test.move` pass, especially the unequal weight cases.

### 5. Constraint

Do **not** revert to scalar reconstruction. The reconstruction **must** happen on G1 points to prevent the Master Secret Key from ever being exposed in memory.
