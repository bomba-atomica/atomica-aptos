
<a id="0x1_ibe"></a>

# Module `0x1::ibe`

IBE (Identity-Based Encryption) native function wrappers.

This module provides Move wrappers for IBE native functions:
- <code>reconstruct_ibe_dk_internal</code>: Reconstructs DK from PVSS threshold scalar shares

The native functions are implemented in Rust in the algebra natives
and registered via the <code>natives::cryptography::algebra::ibe</code> module.


<a id="@Data_Format_0"></a>

## Data Format


The native function accepts and returns byte vectors:
- <code>scalar_shares</code>: <code><a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code> where each inner vector is a 32-byte little-endian Scalar
- Return value: <code><a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code> which is a 48-byte compressed G1 (the reconstructed DK)

This matches <code>aptos_dkg::ibe::reconstruct_ibe_dk()</code> requirements.


-  [Data Format](#@Data_Format_0)
-  [Function `reconstruct_ibe_dk`](#0x1_ibe_reconstruct_ibe_dk)
    -  [Arguments](#@Arguments_1)
    -  [Returns](#@Returns_2)
    -  [Aborts](#@Aborts_3)
-  [Function `reconstruct_ibe_dk_internal`](#0x1_ibe_reconstruct_ibe_dk_internal)


<pre><code></code></pre>



<a id="0x1_ibe_reconstruct_ibe_dk"></a>

## Function `reconstruct_ibe_dk`

Reconstruct an IBE decryption key from PVSS threshold scalar shares.


<a id="@Arguments_1"></a>

### Arguments

* <code>validator_indices</code> - Vector of validator indices (0-indexed, from DKG)
* <code>scalar_shares</code> - Nested vector of 32-byte little-endian scalar shares
* <code>weights</code> - Full vector of validator weights (for ALL validators, not just participating)
* <code>threshold</code> - Minimum number of shares required for reconstruction
* <code>total_weight</code> - Sum of all validator weights
* <code>identity</code> - 32-byte identity hash (from compute_identity)


<a id="@Returns_2"></a>

### Returns

The reconstructed decryption key as 48-byte compressed G1


<a id="@Aborts_3"></a>

### Aborts

- If <code>validator_indices</code> and <code>scalar_shares</code> have different lengths
- If fewer than <code>threshold</code> weight units are provided


<pre><code><b>public</b> <b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk">reconstruct_ibe_dk</a>&lt;G1&gt;(validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, threshold: u64, total_weight: u64, identity: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk">reconstruct_ibe_dk</a>&lt;G1&gt;(
    validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    scalar_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    threshold: u64,
    total_weight: u64,
    identity: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal">reconstruct_ibe_dk_internal</a>&lt;G1&gt;(
        validator_indices,
        scalar_shares,
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

Internal native function. Returns 48-byte compressed G1.


<pre><code><b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal">reconstruct_ibe_dk_internal</a>&lt;G1&gt;(validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, threshold: u64, total_weight: u64, identity: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal">reconstruct_ibe_dk_internal</a>&lt;G1&gt;(
    validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    scalar_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    threshold: u64,
    total_weight: u64,
    identity: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
): <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
