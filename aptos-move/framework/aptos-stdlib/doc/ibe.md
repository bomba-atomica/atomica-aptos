
<a id="0x1_ibe"></a>

# Module `0x1::ibe`

IBE (Identity-Based Encryption) native function wrappers.

This module provides Move wrappers for IBE native functions:
- <code>reconstruct_ibe_dk_internal</code>: Reconstructs DK from threshold shares

The native functions are implemented in Rust in the algebra natives
and registered via the <code>natives::cryptography::algebra::ibe</code> module.


-  [Function `reconstruct_ibe_dk_internal`](#0x1_ibe_reconstruct_ibe_dk_internal)
    -  [Arguments](#@Arguments_0)
    -  [Returns](#@Returns_1)
    -  [Aborts](#@Aborts_2)
    -  [Example](#@Example_3)
-  [Function `reconstruct_ibe_dk_internal_internal`](#0x1_ibe_reconstruct_ibe_dk_internal_internal)


<pre><code><b>use</b> <a href="crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
</code></pre>



<a id="0x1_ibe_reconstruct_ibe_dk_internal"></a>

## Function `reconstruct_ibe_dk_internal`

Reconstruct an IBE decryption key from threshold shares using Lagrange interpolation.

This function implements: DK = Σ λ_i * dk_share_i
where λ_i are weighted Lagrange coefficients based on validator indices and weights.


<a id="@Arguments_0"></a>

### Arguments

* <code>validator_indices</code> - Vector of validator indices (1-indexed, typically from DKG)
* <code>dk_shares</code> - Vector of G1 decryption key shares (crypto_algebra::Element<G1>)
* <code>weights</code> - Vector of validator weights corresponding to each share
* <code>threshold</code> - Minimum number of shares required for reconstruction
* <code>total_weight</code> - Sum of all validator weights


<a id="@Returns_1"></a>

### Returns

The reconstructed decryption key as a crypto_algebra::Element<G1>


<a id="@Aborts_2"></a>

### Aborts

- If <code>validator_indices</code>, <code>dk_shares</code>, and <code>weights</code> have different lengths
- If fewer than <code>threshold</code> shares are provided


<a id="@Example_3"></a>

### Example

```
// 3 validators with equal weights, threshold 2
let dk = reconstruct_ibe_dk_internal<G1>(
vector[1, 2, 3],           // validator indices
vector[share1, share2, share3],  // DK shares
vector[1, 1, 1],           // weights
2,                         // threshold
3                          // total_weight
);
```


<pre><code><b>public</b> <b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal">reconstruct_ibe_dk_internal</a>&lt;G1&gt;(validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, dk_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;G1&gt;&gt;, weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, threshold: u64, total_weight: u64): <a href="crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;G1&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal">reconstruct_ibe_dk_internal</a>&lt;G1&gt;(
    validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    dk_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;G1&gt;&gt;,
    weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    threshold: u64,
    total_weight: u64,
): <a href="crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;G1&gt; {
    <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal_internal">reconstruct_ibe_dk_internal_internal</a>(
        validator_indices,
        dk_shares,
        weights,
        threshold,
        total_weight
    )
}
</code></pre>



</details>

<a id="0x1_ibe_reconstruct_ibe_dk_internal_internal"></a>

## Function `reconstruct_ibe_dk_internal_internal`

Internal native function wrapper.
This is called via the algebra natives infrastructure.


<pre><code>#[deprecated]
<b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal_internal">reconstruct_ibe_dk_internal_internal</a>&lt;G1&gt;(validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, dk_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;G1&gt;&gt;, weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, threshold: u64, total_weight: u64): <a href="crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;G1&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>native</b> <b>fun</b> <a href="ibe.md#0x1_ibe_reconstruct_ibe_dk_internal_internal">reconstruct_ibe_dk_internal_internal</a>&lt;G1&gt;(
    validator_indices: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    dk_shares: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;G1&gt;&gt;,
    weights: <a href="../../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    threshold: u64,
    total_weight: u64,
): <a href="crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;G1&gt;;
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
