
<a id="0x1_ibe_signature"></a>

# Module `0x1::ibe_signature`



-  [Constants](#@Constants_0)
    -  [Reference](#@Reference_1)
    -  [The Map-To-Point Function ($H_1$)](#@The_Map-To-Point_Function_($H_1$)_2)
    -  [Protocol Role](#@Protocol_Role_3)
-  [Function `identity_to_point`](#0x1_ibe_signature_identity_to_point)
    -  [Mathematical Definition](#@Mathematical_Definition_4)
-  [Function `verify_private_key`](#0x1_ibe_signature_verify_private_key)
    -  [Verification Logic](#@Verification_Logic_5)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">0x1::bls12381_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
<b>use</b> <a href="threshold_dsa.md#0x1_threshold_dsa">0x1::threshold_dsa</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_ibe_signature_DST"></a>

This module implements the Identity-Based Encryption (IBE) primitive semantics,
specifically mapping Identities to Group Elements.


<a id="@Reference_1"></a>

### Reference


*   **[BF01]**: Boneh, D., & Franklin, M. (2001). "Identity-based encryption from the Weil pairing."
Section 4.1 "BasicIdent: A basic IBE system" -> **Extract** algorithm.


<a id="@The_Map-To-Point_Function_($H_1$)_2"></a>

### The Map-To-Point Function ($H_1$)


[BF01] requires a hash function $H_1: \{0,1\}^* \to G_1^*$.
We implement this using the IETF standard <code>hash_to_curve</code> (XMD:SHA-256_SSWU_RO).


<a id="@Protocol_Role_3"></a>

### Protocol Role


This module connects the abstract concept of an "Identity" (e.g. a time interval)
to the cryptographic verification logic in <code><a href="threshold_dsa.md#0x1_threshold_dsa">threshold_dsa</a></code>.


<pre><code><b>const</b> <a href="ibe_signature.md#0x1_ibe_signature_DST">DST</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 80, 84, 79, 83, 95, 66, 76, 83, 95, 87, 86, 85, 70, 95, 68, 83, 84];
</code></pre>



<a id="0x1_ibe_signature_H_M_MSG"></a>



<pre><code><b>const</b> <a href="ibe_signature.md#0x1_ibe_signature_H_M_MSG">H_M_MSG</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [72, 40, 109, 41];
</code></pre>



<a id="0x1_ibe_signature_identity_to_point"></a>

## Function `identity_to_point`

Map an Identity string to a $G_1$ group element.


<a id="@Mathematical_Definition_4"></a>

### Mathematical Definition


$$ Q_{ID} = H_1(ID) \in G_1^* $$

This corresponds to the first step of the **Extract** algorithm in [BF01].
In our Timelock system, <code>identity_bytes</code> is the Keccak256 hash of the identity string.

Note: This function prepends "H(m)" to the identity before hashing to curve,
matching the Rust implementation's hash_to_curve(identity, DST, b"H(m)") signature.


<pre><code><b>public</b> <b>fun</b> <a href="ibe_signature.md#0x1_ibe_signature_identity_to_point">identity_to_point</a>(identity_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_signature.md#0x1_ibe_signature_identity_to_point">identity_to_point</a>(identity_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Element&lt;G1&gt; {
    // Prepend "H(m)" message <b>to</b> match Rust hash_to_curve(identity, <a href="ibe_signature.md#0x1_ibe_signature_DST">DST</a>, b"H(m)")
    // This is specific <b>to</b> the blstrs crate's hash_to_curve implementation
    <b>let</b> msg_with_h = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> msg_with_h, 72); // 'H'
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> msg_with_h, 40); // '('
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> msg_with_h, 109); // 'm'
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> msg_with_h, 41); // ')'
    <b>let</b> i = 0;
    <b>while</b> (i &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&identity_bytes)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> msg_with_h, *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&identity_bytes, i));
        i = i + 1;
    };
    hash_to&lt;G1, HashG1XmdSha256SswuRo&gt;(&<a href="ibe_signature.md#0x1_ibe_signature_DST">DST</a>, &msg_with_h)
}
</code></pre>



</details>

<a id="0x1_ibe_signature_verify_private_key"></a>

## Function `verify_private_key`

Verify that a given Private Key ($d_{ID}$) corresponds to the Identity ($ID$)
under the Master Public Key ($P_{pub}$) for the given system ID.


<a id="@Verification_Logic_5"></a>

### Verification Logic


This function verifies that the provided <code>private_key_bytes</code> constitute a valid IBE Private Key for the identity.

$$ \text{Verify}(P_{pub}, ID, d_{ID}) \iff e(d_{ID}, g_2) = e(H_1(ID), P_{pub}) $$

In the context of the Timelock Service:
*   **Identity**: The time interval number (serialized).
*   **Private Key**: The "decryption key" revealed by the validator set.
*   **Verification**: Ensures that the revealed key is cryptographically valid and bound to the interval.

This wraps <code><a href="threshold_dsa.md#0x1_threshold_dsa_verify_signature_point">threshold_dsa::verify_signature_point</a></code>.


<pre><code><b>public</b> <b>fun</b> <a href="ibe_signature.md#0x1_ibe_signature_verify_private_key">verify_private_key</a>(mpk_id: u64, identity: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, private_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_signature.md#0x1_ibe_signature_verify_private_key">verify_private_key</a>(mpk_id: u64, identity: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, private_key_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool {
    <b>let</b> point = <a href="ibe_signature.md#0x1_ibe_signature_identity_to_point">identity_to_point</a>(identity);
    <a href="threshold_dsa.md#0x1_threshold_dsa_verify_signature_point">threshold_dsa::verify_signature_point</a>(mpk_id, point, private_key_bytes)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
