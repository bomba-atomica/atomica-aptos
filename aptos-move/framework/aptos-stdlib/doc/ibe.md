
<a id="0x1_ibe"></a>

# Module `0x1::ibe`

IBE (Identity-Based Encryption) native function wrappers.

This module provides Move wrappers for IBE native functions:
- <code>reconstruct_ibe_dk_internal</code>: Reconstructs DK from threshold shares

The native functions are implemented in Rust in the algebra natives
and registered via the <code>natives::cryptography::algebra::ibe</code> module.


-  [Function `reconstruct_ibe_dk`](#0x1_ibe_reconstruct_ibe_dk)
    -  [Arguments](#@Arguments_0)
    -  [Returns](#@Returns_1)
    -  [Aborts](#@Aborts_2)
    -  [Example](#@Example_3)
-  [Function `reconstruct_ibe_dk_internal`](#0x1_ibe_reconstruct_ibe_dk_internal)


<pre><code><b>use</b> <a href="crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
</code></pre>



<a id="0x1_ibe_reconstruct_ibe_dk"></a>

## Function `reconstruct_ibe_dk`

Reconstruct an IBE decryption key from threshold shares using Lagrange interpolation.

This function implements weighted reconstruction matching the PVSS framework:
For each validator's scalar shares s_i, compute DK contribution: s_i * H(identity)
Then interpolate using weighted Lagrange coefficients.


<a id="@Arguments_0"></a>

### Arguments

* <code>validator_indices</code> - Vector of validator indices (0-indexed, from DKG)
* <code>scalar_shares</code> - Nested vector of scalar shares (one inner vector per validator)
* <code>weights</code> - Full vector of validator weights (for ALL validators, not just participating)
* <code>threshold</code> - Minimum number of shares required for reconstruction
* <code>total_weight</code> - Sum of all validator weights
* <code>identity</code> - 32-byte identity hash (from compute_identity)


<a id="@Returns_1"></a>

### Returns

The reconstructed decryption key as a crypto_algebra::Element<G1>


<a id="@Aborts_2"></a>

### Aborts

- If <code>validator_indices</code> and <code>scalar_shares</code> have different lengths
- If scalar_shares inner vectors don't match weights
- If fewer than <code>threshold</code> weight units are provided


<a id="@Example_3"></a>

### Example

```
// 3 validators with weights [1, 1, 1], validators 0, 1, 2 participating
// Each validator has 1 share (weight=1)
let dk = reconstruct_ibe_dk<G1>(
vector[0, 1, 2],                    // validator indices
vector[vector[s0], vector[s1], vector[s2]],  // scalar shares
vector[1, 1, 1],                    // full weights
3,                                  // threshold
3,                                  // total_weight
identity                            // 32-byte identity hash
);
```


<pre><code><b>public</b> <b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk">reconstruct_ibe_dk</a>&lt;G1&gt;(validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, threshold: u64, total_weight: u64, identity: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;G1&gt;
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
): <a href="crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;G1&gt; {
    <a href="crypto_algebra.md#0x1_crypto_algebra_new_element">crypto_algebra::new_element</a>&lt;G1&gt;(
        <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal">reconstruct_ibe_dk_internal</a>&lt;G1&gt;(
            validator_indices,
            scalar_shares,
            weights,
            threshold,
            total_weight,
            identity
        )
    )
}
</code></pre>



</details>

<a id="0x1_ibe_reconstruct_ibe_dk_internal"></a>

## Function `reconstruct_ibe_dk_internal`

Internal native function wrapper.
Accepts scalar shares as byte vectors (little-endian 32-byte scalars).


<pre><code><b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal">reconstruct_ibe_dk_internal</a>&lt;G1&gt;(validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, scalar_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, threshold: u64, total_weight: u64, identity: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): u64
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
): u64;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
