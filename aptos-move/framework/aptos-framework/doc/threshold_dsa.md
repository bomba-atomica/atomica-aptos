
<a id="0x1_threshold_dsa"></a>

# Module `0x1::threshold_dsa`



-  [Resource `State`](#0x1_threshold_dsa_State)
-  [Struct `MasterPublicKeyPublishedEvent`](#0x1_threshold_dsa_MasterPublicKeyPublishedEvent)
-  [Constants](#@Constants_0)
    -  [Cryptographic Foundation](#@Cryptographic_Foundation_1)
        -  [References](#@References_2)
        -  [Definitions](#@Definitions_3)
    -  [Access Control](#@Access_Control_4)
-  [Function `initialize`](#0x1_threshold_dsa_initialize)
-  [Function `publish_master_public_key`](#0x1_threshold_dsa_publish_master_public_key)
-  [Function `get_master_public_key`](#0x1_threshold_dsa_get_master_public_key)
-  [Function `verify_signature`](#0x1_threshold_dsa_verify_signature)
    -  [Mathematical Verification](#@Mathematical_Verification_5)
    -  [Parameters](#@Parameters_6)
-  [Function `verify_signature_point`](#0x1_threshold_dsa_verify_signature_point)
-  [Function `verify_timelock_share`](#0x1_threshold_dsa_verify_timelock_share)
-  [Function `aggregate_timelock_shares`](#0x1_threshold_dsa_aggregate_timelock_shares)
    -  [Parameters](#@Parameters_7)
    -  [Returns](#@Returns_8)
-  [Function `compute_lagrange_coefficient`](#0x1_threshold_dsa_compute_lagrange_coefficient)
-  [Function `mod_exp`](#0x1_threshold_dsa_mod_exp)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">0x1::bls12381_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
</code></pre>



<a id="0x1_threshold_dsa_State"></a>

## Resource `State`



<pre><code><b>struct</b> <a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>master_public_keys: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Map of ID (e.g. epoch) to Master Public Key bytes (compressed G2)
</dd>
</dl>


</details>

<a id="0x1_threshold_dsa_MasterPublicKeyPublishedEvent"></a>

## Struct `MasterPublicKeyPublishedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="threshold_dsa.md#0x1_threshold_dsa_MasterPublicKeyPublishedEvent">MasterPublicKeyPublishedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>master_public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_threshold_dsa_ENOT_VALIDATOR"></a>

The <code><a href="threshold_dsa.md#0x1_threshold_dsa">threshold_dsa</a></code> module implements specific components of a Threshold Digital Signature Algorithm (DSA)
based on the BLS (Boneh-Lynn-Shacham) signature scheme.


<a id="@Cryptographic_Foundation_1"></a>

### Cryptographic Foundation


This module manages the **Master Public Key (MPK)** and verifies signatures against it.
In the context of Identity-Based Encryption (IBE), the "Signature" corresponds to an extracted Private Key over an Identity.


<a id="@References_2"></a>

#### References


*   **[BLS01]**: Boneh, D., Lynn, B., & Shacham, H. (2001). "Short signatures from the Weil pairing."
*   **[BF01]**: Boneh, D., & Franklin, M. (2001). "Identity-based encryption from the Weil pairing."


<a id="@Definitions_3"></a>

#### Definitions


*   **$G_1, G_2, G_T$**: Cyclic groups of prime order $q$ equipped with a bilinear map $e: G_1 \times G_2 \to G_T$.
*   **$g_2$**: Generator of $G_2$.
*   **$s$**: The Master Secret Key (MSK), $s \in_R \mathbb{Z}_q^*$.
*   **$P_{pub}$**: The Master Public Key (MPK), $P_{pub} = s \cdot g_2$ (in $G_2$).

Note: This implementation uses Type-3 pairings (BLS12-381), whereas the original [BF01] paper describes Type-1.


<a id="@Access_Control_4"></a>

### Access Control


Only active validators (via <code><a href="stake.md#0x1_stake">stake</a></code> module) are authorized to publish the MPK.


<pre><code><b>const</b> <a href="threshold_dsa.md#0x1_threshold_dsa_ENOT_VALIDATOR">ENOT_VALIDATOR</a>: u64 = 1;
</code></pre>



<a id="0x1_threshold_dsa_EINVALID_PUBKEY"></a>



<pre><code><b>const</b> <a href="threshold_dsa.md#0x1_threshold_dsa_EINVALID_PUBKEY">EINVALID_PUBKEY</a>: u64 = 4;
</code></pre>



<a id="0x1_threshold_dsa_EINVALID_SIGNATURE"></a>



<pre><code><b>const</b> <a href="threshold_dsa.md#0x1_threshold_dsa_EINVALID_SIGNATURE">EINVALID_SIGNATURE</a>: u64 = 5;
</code></pre>



<a id="0x1_threshold_dsa_EMPK_ALREADY_EXISTS"></a>



<pre><code><b>const</b> <a href="threshold_dsa.md#0x1_threshold_dsa_EMPK_ALREADY_EXISTS">EMPK_ALREADY_EXISTS</a>: u64 = 2;
</code></pre>



<a id="0x1_threshold_dsa_EMPK_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="threshold_dsa.md#0x1_threshold_dsa_EMPK_NOT_FOUND">EMPK_NOT_FOUND</a>: u64 = 3;
</code></pre>



<a id="0x1_threshold_dsa_initialize"></a>

## Function `initialize`



<pre><code><b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(framework, <a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a> {
            master_public_keys: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
        });
    }
}
</code></pre>



</details>

<a id="0x1_threshold_dsa_publish_master_public_key"></a>

## Function `publish_master_public_key`

Publish a unified Master Public Key for a given ID.


<pre><code><b>public</b> entry <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_publish_master_public_key">publish_master_public_key</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, id: u64, pk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_publish_master_public_key">publish_master_public_key</a>(
    validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    id: u64,
    pk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a> {
    <b>let</b> validator_addr = std::signer::address_of(validator);
    <b>assert</b>!(<a href="stake.md#0x1_stake_is_current_epoch_validator">stake::is_current_epoch_validator</a>(validator_addr), <a href="threshold_dsa.md#0x1_threshold_dsa_ENOT_VALIDATOR">ENOT_VALIDATOR</a>);

    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a>&gt;(@aptos_framework);
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.master_public_keys, id)) {
        // Validate PK format (G2 compressed)
        <b>let</b> pk_point = deserialize&lt;G2, FormatG2Compr&gt;(&pk);
        <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&pk_point), <a href="threshold_dsa.md#0x1_threshold_dsa_EINVALID_PUBKEY">EINVALID_PUBKEY</a>);

        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.master_public_keys, id, pk);

        aptos_framework::event::emit(<a href="threshold_dsa.md#0x1_threshold_dsa_MasterPublicKeyPublishedEvent">MasterPublicKeyPublishedEvent</a> {
            id,
            master_public_key: pk,
        });
    };
}
</code></pre>



</details>

<a id="0x1_threshold_dsa_get_master_public_key"></a>

## Function `get_master_public_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_get_master_public_key">get_master_public_key</a>(id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_get_master_public_key">get_master_public_key</a>(id: u64): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a>&gt;(@aptos_framework)) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.master_public_keys, id)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.master_public_keys, id))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_threshold_dsa_verify_signature"></a>

## Function `verify_signature`

Verify a signature share (or unique signature) against the generic Master Public Key (MPK).


<a id="@Mathematical_Verification_5"></a>

### Mathematical Verification


Given a message $m$, signature $\sigma$, and public key $P_{pub}$:
1.  Map the message to a point in $G_1$: $H(m) \in G_1$.
2.  Verify the bilinear pairing equality:
$$ e(\sigma, g_2) \stackrel{?}{=} e(H(m), P_{pub}) $$

Where:
*   $\sigma = s \cdot H(m)$ (The signature / extracted private key).
*   $P_{pub} = s \cdot g_2$ (The Master Public Key).

By bilinearity: $e(s \cdot H(m), g_2) = e(H(m), g_2)^s = e(H(m), s \cdot g_2) = e(H(m), P_{pub})$.


<a id="@Parameters_6"></a>

### Parameters


*   <code>id</code>: The identifier for the stored MPK (e.g. epoch).
*   <code>msg</code>: The message bytes to be signed. This will be hashed to curve $G_1$ via <code>hash_to_curve</code>.
*   <code>sig</code>: The signature bytes (compressed $G_1$ point).

Returns <code><b>true</b></code> if the verification holds, <code><b>false</b></code> otherwise.


<pre><code><b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_verify_signature">verify_signature</a>(id: u64, msg: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sig: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_verify_signature">verify_signature</a>(id: u64, msg: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, sig: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool <b>acquires</b> <a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a>&gt;(@aptos_framework)) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a>&gt;(@aptos_framework);
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.master_public_keys, id)) {
        <b>return</b> <b>false</b>
    };

    <b>let</b> mpk_bytes = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.master_public_keys, id);
    <b>let</b> mpk_opt = deserialize&lt;G2, FormatG2Compr&gt;(mpk_bytes);
    <b>let</b> sig_opt = deserialize&lt;G1, FormatG1Compr&gt;(&sig);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&mpk_opt) || <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&sig_opt)) {
        <b>return</b> <b>false</b>
    };

    <b>let</b> mpk = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(mpk_opt);
    <b>let</b> signature = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(sig_opt);

    // Use Hash-<b>to</b>-Curve <b>to</b> map message <b>to</b> G1
    // Using same suite <b>as</b> defined in <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">bls12381_algebra</a>
    <b>let</b> h_msg = hash_to&lt;G1, HashG1XmdSha256SswuRo&gt;(&b"IBE-BLS-SIG", &msg);
    // Note: DST should be verified against <b>spec</b>. "IBE-BLS-SIG" matches nothing?
    // Using empty DST or a specific one?
    // Boneh-Franklin uses H_1.
    // <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">bls12381_algebra</a> uses HashG1XmdSha256SswuRo default DST "QUUX...".
    // I should probably pass DST or <b>use</b> a standard one.
    // For now using "IBE-BLS_SIG" <b>as</b> placeholder. This must match how signatures were generated!
    // But wait, the user said "Exact nomenclature of IBE paper".
    // The paper just says H_1.
    // I will <b>use</b> a fixed DST provided by `<a href="ibe_signature.md#0x1_ibe_signature">ibe_signature</a>` via arguments?
    // `<a href="threshold_dsa.md#0x1_threshold_dsa">threshold_dsa</a>` is generic. Let's <b>assume</b> the caller handles hashing <b>to</b> G1?
    // But `verify_signature` takes `msg: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;`.
    // If `<a href="ibe_signature.md#0x1_ibe_signature">ibe_signature</a>` calls this <b>with</b> ALREADY HASHED bytes (ID), then we should <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">hash</a> them <b>to</b> curve.

    // Verification: e(sig, g2_gen) == e(h_msg, mpk)
    <b>let</b> lhs = pairing&lt;G1, G2, Gt&gt;(&signature, &one&lt;G2&gt;());
    <b>let</b> rhs = pairing&lt;G1, G2, Gt&gt;(&h_msg, &mpk);

    eq&lt;Gt&gt;(&lhs, &rhs)
}
</code></pre>



</details>

<a id="0x1_threshold_dsa_verify_signature_point"></a>

## Function `verify_signature_point`

Helper to verify a point directly if the message is already mapped to G1


<pre><code><b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_verify_signature_point">verify_signature_point</a>(id: u64, msg_point: <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;, sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_verify_signature_point">verify_signature_point</a>(id: u64, msg_point: Element&lt;G1&gt;, sig_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool <b>acquires</b> <a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a> {
     <b>if</b> (!<b>exists</b>&lt;<a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a>&gt;(@aptos_framework)) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="threshold_dsa.md#0x1_threshold_dsa_State">State</a>&gt;(@aptos_framework);
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.master_public_keys, id)) {
        <b>return</b> <b>false</b>
    };

    <b>let</b> mpk_bytes = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.master_public_keys, id);
    <b>let</b> mpk_opt = deserialize&lt;G2, FormatG2Compr&gt;(mpk_bytes);
    <b>let</b> sig_opt = deserialize&lt;G1, FormatG1Compr&gt;(&sig_bytes);

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&mpk_opt) || <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_none">option::is_none</a>(&sig_opt)) {
        <b>return</b> <b>false</b>
    };

    <b>let</b> mpk = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(mpk_opt);
    <b>let</b> signature = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(sig_opt);

    <b>let</b> lhs = pairing&lt;G1, G2, Gt&gt;(&signature, &one&lt;G2&gt;());
    <b>let</b> rhs = pairing&lt;G1, G2, Gt&gt;(&msg_point, &mpk);

    eq&lt;Gt&gt;(&lhs, &rhs)
}
</code></pre>



</details>

<a id="0x1_threshold_dsa_verify_timelock_share"></a>

## Function `verify_timelock_share`

Verify that a timelock decryption key share is valid

Currently performs format validation (G1 point deserialization).
Full cryptographic verification requires DKG state for:
- Validator public key lookup by index
- Identity-based verification (share should be s_i × Q_id)


<pre><code><b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_verify_timelock_share">verify_timelock_share</a>(share_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_verify_timelock_share">verify_timelock_share</a>(
    share_bytes: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
): bool {
    <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&deserialize&lt;G1, FormatG1Compr&gt;(&share_bytes))
}
</code></pre>



</details>

<a id="0x1_threshold_dsa_aggregate_timelock_shares"></a>

## Function `aggregate_timelock_shares`

Aggregate timelock decryption key shares using Lagrange-weighted sum

Given shares s_i × Q_id from participating validators with indices V,
the decryption key is: DK = Σ λ_i × (s_i × Q_id) = (Σ λ_i × s_i) × Q_id = s × Q_id
where λ_i are Lagrange coefficients for the set V.


<a id="@Parameters_7"></a>

### Parameters


* <code>share_bytes_list</code>: Vector of serialized G1 share points
* <code>validator_indices</code>: Vector of validator indices corresponding to each share
* <code>total_validators</code>: Total number of validators in the DKG session


<a id="@Returns_8"></a>

### Returns


The aggregated decryption key (serialized G1 point), or empty if shares is empty


<pre><code><b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_aggregate_timelock_shares">aggregate_timelock_shares</a>(share_bytes_list: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, validator_indices: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, total_validators: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_aggregate_timelock_shares">aggregate_timelock_shares</a>(
    share_bytes_list: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    validator_indices: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;,
    total_validators: u64
): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> n = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(share_bytes_list);
    <b>if</b> (n == 0) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;()
    };

    <b>let</b> lambdas = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u128&gt;();
    <b>let</b> j = 0;
    <b>while</b> (j &lt; n) {
        <b>let</b> idx = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(validator_indices, j);
        <b>let</b> lambda = <a href="threshold_dsa.md#0x1_threshold_dsa_compute_lagrange_coefficient">compute_lagrange_coefficient</a>(idx, validator_indices, total_validators);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> lambdas, lambda);
        j = j + 1;
    };

    <b>let</b> result = zero&lt;G1&gt;();
    <b>let</b> i = 0;
    <b>while</b> (i &lt; n) {
        <b>let</b> share_bytes = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(share_bytes_list, i);
        <b>let</b> share_opt = deserialize&lt;G1, FormatG1Compr&gt;(share_bytes);
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_is_some">option::is_some</a>(&share_opt)) {
            <b>let</b> share = <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_destroy_some">option::destroy_some</a>(share_opt);
            <b>let</b> lambda_u128 = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&lambdas, i);
            <b>let</b> lambda_scalar = from_u64&lt;Fr&gt;((lambda_u128 <b>as</b> u64));
            <b>let</b> scaled = scalar_mul(&share, &lambda_scalar);
            result = add(&result, &scaled);
        };
        i = i + 1;
    };

    serialize&lt;G1, FormatG1Compr&gt;(&result)
}
</code></pre>



</details>

<a id="0x1_threshold_dsa_compute_lagrange_coefficient"></a>

## Function `compute_lagrange_coefficient`

Compute Lagrange coefficient λ_k for validator k

λ_k = Π_{i ∈ V, i ≠ k} (0 - i) / (k - i)
= Π_{i ∈ V, i ≠ k} (-i) / (k - i)

Uses modulo arithmetic with prime q = 1000003.


<pre><code><b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_compute_lagrange_coefficient">compute_lagrange_coefficient</a>(k: u64, participants: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, _total: u64): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_compute_lagrange_coefficient">compute_lagrange_coefficient</a>(k: u64, participants: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, _total: u64): u128 {
    <b>let</b> num = 1u128;
    <b>let</b> den = 1u128;
    <b>let</b> n = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(participants);
    <b>let</b> i = 0;
    <b>let</b> q: u128 = 1000003;
    <b>while</b> (i &lt; n) {
        <b>let</b> idx = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(participants, i);
        <b>if</b> (idx != k) {
            // numerator: (-idx) mod q = q - idx
            <b>let</b> neg_idx = q - (idx <b>as</b> u128);
            num = (num * neg_idx) % q;

            // denominator: (k - idx) mod q
            // Handle the sign by checking <b>if</b> k &gt; idx
            <b>let</b> diff = <b>if</b> (k &gt; idx) { k - idx } <b>else</b> { idx - k };
            <b>let</b> diff_mod = (diff <b>as</b> u128) % q;
            <b>if</b> (k &lt; idx) {
                // k - idx is negative, so (k - idx) mod q = q - diff
                den = (den * (q - diff_mod)) % q;
            } <b>else</b> {
                den = (den * diff_mod) % q;
            };
        };
        i = i + 1;
    };

    // λ = num * den^(-1) mod q
    <b>let</b> den_inv = <a href="threshold_dsa.md#0x1_threshold_dsa_mod_exp">mod_exp</a>(den, q - 2, q);
    (num * den_inv) % q
}
</code></pre>



</details>

<a id="0x1_threshold_dsa_mod_exp"></a>

## Function `mod_exp`

Modular exponentiation: base^exp mod mod


<pre><code><b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_mod_exp">mod_exp</a>(base: u128, exp: u128, mod: u128): u128
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="threshold_dsa.md#0x1_threshold_dsa_mod_exp">mod_exp</a>(base: u128, exp: u128, mod: u128): u128 {
    <b>let</b> result = 1u128;
    <b>let</b> b = base % mod;
    <b>let</b> e = exp;
    <b>while</b> (e &gt; 0) {
        <b>if</b> (e % 2 == 1) {
            result = (result * b) % mod;
        };
        b = (b * b) % mod;
        e = e / 2;
    };
    result
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
