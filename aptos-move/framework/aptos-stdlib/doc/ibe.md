
<a id="0x1_ibe"></a>

# Module `0x1::ibe`

IBE (Identity-Based Encryption) native function wrappers.

This module provides Move wrappers for IBE native functions:
- <code>reconstruct_ibe_dk_internal</code>: Reconstructs DK from G1 DK shares

The native functions are implemented in Rust in the algebra natives
and registered via the <code>natives::cryptography::algebra::ibe</code> module.


<a id="@Data_Flow_(From_Audit)_0"></a>

## Data Flow (From Audit)


```text
Validator holds: s_i (32-byte scalars, private)
Validator computes: dk_share_i = sum_j(s_i[j]) × H(identity) → 48-byte G1 point
Validator submits: dk_share_i (48 bytes) → stored in submitted_shares
Reconstruction: DK = Σ(Lagrange_i × dk_share_i) → 48-byte G1 point
```


<a id="@Data_Format_1"></a>

## Data Format


The native function accepts and returns byte vectors:

| Parameter | Move Type | Description |
|-----------|-----------|-------------|
| <code>validator_indices</code> | <code><a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code> | Validator indices (0-based) |
| <code>dk_shares</code> | <code><a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code> | 48-byte compressed G1 points |
| <code>weights</code> | <code><a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code> | Full weights for ALL validators |
| <code>total_weight</code> | <code>u64</code> | Sum of all validator weights |
| <code>identity</code> | <code><a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code> | 32-byte IBE identity |
| **Returns** | <code><a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code> | 48-byte compressed G1 (DK) |


<a id="@Security_Note_2"></a>

## Security Note


**CRITICAL**: The system accepts DK shares without cryptographic verification.
A malicious validator can submit arbitrary G1 points. Consider adding:
- Storage of public key shares from PVSS transcript
- Verification: e(dk_share, g₂) == e(H(identity), aggregate(pk_shares))


-  [Data Flow (From Audit)](#@Data_Flow_(From_Audit)_0)
-  [Data Format](#@Data_Format_1)
-  [Security Note](#@Security_Note_2)
-  [Function `reconstruct_ibe_dk`](#0x1_ibe_reconstruct_ibe_dk)
    -  [Arguments](#@Arguments_3)
    -  [Returns](#@Returns_4)
    -  [Aborts](#@Aborts_5)
-  [Function `reconstruct_ibe_dk_internal`](#0x1_ibe_reconstruct_ibe_dk_internal)


<pre><code></code></pre>



<a id="0x1_ibe_reconstruct_ibe_dk"></a>

## Function `reconstruct_ibe_dk`

Reconstructs an IBE decryption key from G1 DK shares.

This function performs weighted Lagrange interpolation directly on G1 points.
Each validator's DK share is a vector of contributions, one per virtual player:
<code>dk_share_i_j = s_i[j] × H(identity)</code>


<a id="@Arguments_3"></a>

### Arguments

* <code>validator_indices</code> - Vector of validator indices (0-indexed, from DKG)
* <code>dk_shares</code> - Nested vector of 48-byte compressed G1 points.
<code>dk_shares[i]</code> contains the shares for validator <code>validator_indices[i]</code>.
* <code>weights</code> - Full vector of validator weights (for ALL validators, not just participating)
* <code>threshold</code> - Minimum weight required for reconstruction (as defined during DKG)
* <code>total_weight</code> - Sum of all validator weights
* <code>identity</code> - 32-byte identity hash (from compute_identity)


<a id="@Returns_4"></a>

### Returns

THE reconstructed decryption key as 48-byte compressed G1


<a id="@Aborts_5"></a>

### Aborts

- If <code>validator_indices</code> and <code>dk_shares</code> have different lengths
- If any dk_share is not exactly 48 bytes


<pre><code><b>public</b> <b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk">reconstruct_ibe_dk</a>&lt;G1&gt;(validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, dk_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, threshold: u64, total_weight: u64, identity: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk">reconstruct_ibe_dk</a>&lt;G1&gt;(
    validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    dk_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    threshold: u64,
    total_weight: u64,
    identity: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal">reconstruct_ibe_dk_internal</a>&lt;G1&gt;(
        validator_indices,
        dk_shares,
        weights,
        threshold,
        total_weight,
        identity
    )
}
</code></pre>



</details>

<a id="0x1_ibe_reconstruct_ibe_dk_internal"></a>

## Function `reconstruct_ibe_dk_internal`

Internal native function. Accepts 48-byte G1 DK shares, returns 48-byte compressed G1.


<pre><code><b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal">reconstruct_ibe_dk_internal</a>&lt;G1&gt;(validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, dk_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;, weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, threshold: u64, total_weight: u64, identity: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal">reconstruct_ibe_dk_internal</a>&lt;G1&gt;(
    validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    dk_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;,
    weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    threshold: u64,
    total_weight: u64,
    identity: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
