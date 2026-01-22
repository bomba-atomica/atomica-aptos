
<a id="0x1_ibe_golden_vector_fixtures"></a>

# Module `0x1::ibe_golden_vector_fixtures`

IBE Golden Vector Fixtures (Test-Only)

This module provides access to golden test vectors for IBE testing.
All values are encoded as hex strings matching the JSON golden vectors.

The golden vectors are generated from real PVSS transcripts and include:
- Identity computation test vectors
- IBE roundtrip test vectors with full encryption/decryption verification


<a id="@Usage_0"></a>

## Usage


```move
use aptos_framework::ibe_golden_vector_fixtures as fixtures;

// Get identity hash for a specific timelock
let identity = fixtures::get_identity_hash(0, 1000000000000);
assert!(identity == x"cd2f0ecdda375c87027bcbd9a8b429057062065239267bfd2496e844b81f7355", 0);
```


-  [Usage](#@Usage_0)
-  [Function `identity_0_1000000000000`](#0x1_ibe_golden_vector_fixtures_identity_0_1000000000000)
-  [Function `h_identity_0_1000000000000`](#0x1_ibe_golden_vector_fixtures_h_identity_0_1000000000000)
-  [Function `identity_1_1000000000000`](#0x1_ibe_golden_vector_fixtures_identity_1_1000000000000)
-  [Function `h_identity_1_1000000000000`](#0x1_ibe_golden_vector_fixtures_h_identity_1_1000000000000)
-  [Function `identity_0_2000000000000`](#0x1_ibe_golden_vector_fixtures_identity_0_2000000000000)
-  [Function `h_identity_0_2000000000000`](#0x1_ibe_golden_vector_fixtures_h_identity_0_2000000000000)
-  [Function `roundtrip_1_rng_seed`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_rng_seed)
-  [Function `roundtrip_1_msk`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_msk)
-  [Function `roundtrip_1_mpk`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_mpk)
-  [Function `roundtrip_1_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_identity)
-  [Function `roundtrip_1_h_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_h_identity)
-  [Function `roundtrip_1_threshold`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_threshold)
-  [Function `roundtrip_1_total_weight`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_total_weight)
-  [Function `roundtrip_1_validator_indices`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_indices)
-  [Function `roundtrip_1_validator_weights`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_weights)
-  [Function `roundtrip_1_dk_shares`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_dk_shares)
-  [Function `roundtrip_1_reconstructed_dk`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_reconstructed_dk)
-  [Function `roundtrip_1_plaintext`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_plaintext)
-  [Function `roundtrip_1_ciphertext_u`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_ciphertext_u)
-  [Function `roundtrip_1_ciphertext_v`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_ciphertext_v)
-  [Function `roundtrip_2_rng_seed`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_rng_seed)
-  [Function `roundtrip_2_msk`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_msk)
-  [Function `roundtrip_2_mpk`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_mpk)
-  [Function `roundtrip_2_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_identity)
-  [Function `roundtrip_2_h_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_h_identity)
-  [Function `roundtrip_2_threshold`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_threshold)
-  [Function `roundtrip_2_total_weight`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_total_weight)
-  [Function `roundtrip_2_validator_indices`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_indices)
-  [Function `roundtrip_2_validator_weights`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_weights)
-  [Function `roundtrip_2_dk_shares`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_dk_shares)
-  [Function `roundtrip_2_reconstructed_dk`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_reconstructed_dk)
-  [Function `roundtrip_2_plaintext`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_plaintext)
-  [Function `roundtrip_2_ciphertext_u`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_ciphertext_u)
-  [Function `roundtrip_2_ciphertext_v`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_ciphertext_v)
-  [Function `roundtrip_3_rng_seed`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_rng_seed)
-  [Function `roundtrip_3_msk`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_msk)
-  [Function `roundtrip_3_mpk`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_mpk)
-  [Function `roundtrip_3_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_identity)
-  [Function `roundtrip_3_h_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_h_identity)
-  [Function `roundtrip_3_threshold`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_threshold)
-  [Function `roundtrip_3_total_weight`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_total_weight)
-  [Function `roundtrip_3_validator_indices`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_indices)
-  [Function `roundtrip_3_validator_weights`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_weights)
-  [Function `roundtrip_3_dk_shares`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_dk_shares)
-  [Function `roundtrip_3_reconstructed_dk`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_reconstructed_dk)
-  [Function `roundtrip_3_plaintext`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_plaintext)
-  [Function `roundtrip_3_ciphertext_u`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_ciphertext_u)
-  [Function `roundtrip_3_ciphertext_v`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_ciphertext_v)
-  [Function `roundtrip_4_rng_seed`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_rng_seed)
-  [Function `roundtrip_4_msk`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_msk)
-  [Function `roundtrip_4_mpk`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_mpk)
-  [Function `roundtrip_4_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_identity)
-  [Function `roundtrip_4_h_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_h_identity)
-  [Function `roundtrip_4_threshold`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_threshold)
-  [Function `roundtrip_4_total_weight`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_total_weight)
-  [Function `roundtrip_4_validator_indices`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_indices)
-  [Function `roundtrip_4_validator_weights`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_weights)
-  [Function `roundtrip_4_dk_shares`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_dk_shares)
-  [Function `roundtrip_4_reconstructed_dk`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_reconstructed_dk)
-  [Function `roundtrip_4_plaintext`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_plaintext)
-  [Function `roundtrip_4_ciphertext_u`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_ciphertext_u)
-  [Function `roundtrip_4_ciphertext_v`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_ciphertext_v)
-  [Function `get_identity_hash`](#0x1_ibe_golden_vector_fixtures_get_identity_hash)
-  [Function `get_roundtrip_validator_indices`](#0x1_ibe_golden_vector_fixtures_get_roundtrip_validator_indices)
-  [Function `get_roundtrip_validator_weights`](#0x1_ibe_golden_vector_fixtures_get_roundtrip_validator_weights)
-  [Function `get_roundtrip_dk_shares`](#0x1_ibe_golden_vector_fixtures_get_roundtrip_dk_shares)
-  [Function `get_roundtrip_identity`](#0x1_ibe_golden_vector_fixtures_get_roundtrip_identity)
-  [Function `get_roundtrip_total_weight`](#0x1_ibe_golden_vector_fixtures_get_roundtrip_total_weight)
-  [Function `get_roundtrip_threshold`](#0x1_ibe_golden_vector_fixtures_get_roundtrip_threshold)
-  [Function `get_roundtrip_reconstructed_dk`](#0x1_ibe_golden_vector_fixtures_get_roundtrip_reconstructed_dk)


<pre><code></code></pre>



<a id="0x1_ibe_golden_vector_fixtures_identity_0_1000000000000"></a>

## Function `identity_0_1000000000000`

Returns the identity hash for timelock_id=0, deadline_us=1000000000000
Expected: cd2f0ecdda375c87027bcbd9a8b429057062065239267bfd2496e844b81f7355


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_0_1000000000000">identity_0_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_0_1000000000000">identity_0_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"cd2f0ecdda375c87027bcbd9a8b429057062065239267bfd2496e844b81f7355"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_h_identity_0_1000000000000"></a>

## Function `h_identity_0_1000000000000`

Returns H(identity) in G1 for timelock_id=0, deadline_us=1000000000000
Expected: 95502e8ee330870f5002c4cfc834467a507fcadaa49f9da04c539f322858284e627937fb234993f78dd5869b57419aa9


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_0_1000000000000">h_identity_0_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_0_1000000000000">h_identity_0_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"95502e8ee330870f5002c4cfc834467a507fcadaa49f9da04c539f322858284e627937fb234993f78dd5869b57419aa9"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_identity_1_1000000000000"></a>

## Function `identity_1_1000000000000`

Returns the identity hash for timelock_id=1, deadline_us=1000000000000
Expected: 1235cfe3eb61fd7c5aa850477532822bbef4d9ca30decc2ca4405ebb30d5cab1


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_1_1000000000000">identity_1_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_1_1000000000000">identity_1_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"1235cfe3eb61fd7c5aa850477532822bbef4d9ca30decc2ca4405ebb30d5cab1"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_h_identity_1_1000000000000"></a>

## Function `h_identity_1_1000000000000`

Returns H(identity) in G1 for timelock_id=1, deadline_us=1000000000000
Expected: aa5dffd79cb804f313d5c2e882183fd8477d3f9a6424cb56fa373e2c7ce0d0904383af53a5680538423d02ac61eaa210


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_1_1000000000000">h_identity_1_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_1_1000000000000">h_identity_1_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"aa5dffd79cb804f313d5c2e882183fd8477d3f9a6424cb56fa373e2c7ce0d0904383af53a5680538423d02ac61eaa210"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_identity_0_2000000000000"></a>

## Function `identity_0_2000000000000`

Returns the identity hash for timelock_id=0, deadline_us=2000000000000
Expected: ee623d0d70b0a085f0e91a197853c627a9cd7b24cdac79d2782e912e35ddefe6


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_0_2000000000000">identity_0_2000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_0_2000000000000">identity_0_2000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"ee623d0d70b0a085f0e91a197853c627a9cd7b24cdac79d2782e912e35ddefe6"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_h_identity_0_2000000000000"></a>

## Function `h_identity_0_2000000000000`

Returns H(identity) in G1 for timelock_id=0, deadline_us=2000000000000
Expected: b6ef562742ce6af3dcd6335b07bbbd38a36c51d8f13a470a6da39f916ffa41ad3651f454820244a8a38308ef99ec3521


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_0_2000000000000">h_identity_0_2000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_0_2000000000000">h_identity_0_2000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"b6ef562742ce6af3dcd6335b07bbbd38a36c51d8f13a470a6da39f916ffa41ad3651f454820244a8a38308ef99ec3521"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_rng_seed"></a>

## Function `roundtrip_1_rng_seed`

Vector 1: RNG seed


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_rng_seed">roundtrip_1_rng_seed</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_rng_seed">roundtrip_1_rng_seed</a>(): u64 { 12345 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_msk"></a>

## Function `roundtrip_1_msk`

Vector 1: Master secret key (32 bytes, little-endian hex)


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_msk">roundtrip_1_msk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_msk">roundtrip_1_msk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"3fea0be5546f8322489b371987d8097874e8caa1f5e54314f40ad3d56e4c2f6c"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_mpk"></a>

## Function `roundtrip_1_mpk`

Vector 1: Master public key (96 bytes, G2 compressed hex)


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_mpk">roundtrip_1_mpk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_mpk">roundtrip_1_mpk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"82fb16f4572daf7bd95e9c208eb29f4e43fd78df7f4a7129ef438b03b4da1ade521cd25b1eaeec7398bc8e74dd371b5213e87029507649495dc9cf02ace2ca040e79af59d9c380a1bfafa619e511452516e1719ae917265bc87a7b1236a5b0be"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_identity"></a>

## Function `roundtrip_1_identity`

Vector 1: Identity hash (32 bytes)


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_identity">roundtrip_1_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_identity">roundtrip_1_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"e3fb053eedce65f62fcbbfd56fe9bbec0abe3bfcc3c5ec54a913bd50c51a5dfe"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_h_identity"></a>

## Function `roundtrip_1_h_identity`

Vector 1: H(identity) in G1 (48 bytes)


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_h_identity">roundtrip_1_h_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_h_identity">roundtrip_1_h_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"8ee74013044887e31110186f2f90d556e8780998295c69821011707562e44bf9fa28d8c331fa41aaacec7f5f8368b41e"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_threshold"></a>

## Function `roundtrip_1_threshold`

Vector 1: Threshold


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_threshold">roundtrip_1_threshold</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_threshold">roundtrip_1_threshold</a>(): u64 { 3 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_total_weight"></a>

## Function `roundtrip_1_total_weight`

Vector 1: Total weight


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_total_weight">roundtrip_1_total_weight</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_total_weight">roundtrip_1_total_weight</a>(): u64 { 5 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_indices"></a>

## Function `roundtrip_1_validator_indices`

Vector 1: Validator indices [0, 1, 2, 3, 4]


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_indices">roundtrip_1_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_indices">roundtrip_1_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0, 1, 2, 3, 4]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_weights"></a>

## Function `roundtrip_1_validator_weights`

Vector 1: Validator weights [1, 1, 1, 1, 1]


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_weights">roundtrip_1_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_weights">roundtrip_1_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1, 1, 1, 1, 1]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_dk_shares"></a>

## Function `roundtrip_1_dk_shares`

Vector 1: DK shares in G1 (48 bytes each, 5 shares)


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_dk_shares">roundtrip_1_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_dk_shares">roundtrip_1_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        x"9722f3fe074ff0467af66bbb6564aaeec41ac369dbc55a520e1197e2cabd01489fac2b5260c049e9ed26fdd872391d2d",
        x"b0cc6092fc45df1b1318952c08dd2a877b5b19deb725b63cfc41c48e84da6e8f77600efe4d38aac33273a87775112a31",
        x"ac20a63c6b62ed15a3424d85af25eb006795eba2172996529afbb70d29d1a57066a37ee1c98dea843fa2c998cb6b3919",
        x"b42700b02c7997b35f244481afa657a601e915046aed0dee5dcde214e91efdecbf3fcc9d21c11907aac23a84c43927d2",
        x"891c64345686bd154a277a7540ed7c32ad7a0abe30a40586515bf07738e6f63f054b6a55d6d5a2288da37f15335618bf"
    ]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_reconstructed_dk"></a>

## Function `roundtrip_1_reconstructed_dk`

Vector 1: Reconstructed DK in G1 (48 bytes)


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_reconstructed_dk">roundtrip_1_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_reconstructed_dk">roundtrip_1_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"a46066de7604d2d51825cd139d720523d5584654cda567f80c3779304d2dec8842b41a7feec0db2a202409f45492593c"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_plaintext"></a>

## Function `roundtrip_1_plaintext`

Vector 1: Plaintext


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_plaintext">roundtrip_1_plaintext</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_plaintext">roundtrip_1_plaintext</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"48656c6c6f2049424520676f6c64656e20766563746f7220746573742077697468205056535321"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_ciphertext_u"></a>

## Function `roundtrip_1_ciphertext_u`

Vector 1: Ciphertext U component (96 bytes, G2)


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_ciphertext_u">roundtrip_1_ciphertext_u</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_ciphertext_u">roundtrip_1_ciphertext_u</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"b33b839deddaab78e2819a94b74aade57aab61ec3ce41eaa45775eedcc0ea6d51994701e4021e2517f8c3d522ee0c8fb020471a9582fc4e6f64428d833c4ec0eceabfddb11c3c3c11c1829191bc18ccc71ecf9628301bc38ecd0d1f71baa12b1"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_ciphertext_v"></a>

## Function `roundtrip_1_ciphertext_v`

Vector 1: Ciphertext V component


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_ciphertext_v">roundtrip_1_ciphertext_v</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_ciphertext_v">roundtrip_1_ciphertext_v</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"509a8a216f2d56cd243bb83b91b107bb9f495ec1ecb7c4a2d48b9a6b214f42c4136d341ffb8079"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_rng_seed"></a>

## Function `roundtrip_2_rng_seed`

Vector 2: RNG seed


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_rng_seed">roundtrip_2_rng_seed</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_rng_seed">roundtrip_2_rng_seed</a>(): u64 { 67890 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_msk"></a>

## Function `roundtrip_2_msk`

Vector 2: Master secret key


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_msk">roundtrip_2_msk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_msk">roundtrip_2_msk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"f5473a7e34fdbb238794a9a78a62cbe20f5f445fc2d5a5e9d27f0ad59b102f71"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_mpk"></a>

## Function `roundtrip_2_mpk`

Vector 2: Master public key


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_mpk">roundtrip_2_mpk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_mpk">roundtrip_2_mpk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"874774c6f8fc9d169328bd85db98d06147d6cf84706f341f43377fa19934b0b423ce2ce1df95bf9b607a8ef54c16468007b34e34540de15866b0a89503c777c898b863ed1772bd1e0c47122d1c248eb19f670b5263bd99e096841804e4cbd983"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_identity"></a>

## Function `roundtrip_2_identity`

Vector 2: Identity hash


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_identity">roundtrip_2_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_identity">roundtrip_2_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"fe35f39be89aa3d57c43d37f0a4ecb02908c7a82c877eaf5c6517224aa09c46c"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_h_identity"></a>

## Function `roundtrip_2_h_identity`

Vector 2: H(identity) in G1


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_h_identity">roundtrip_2_h_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_h_identity">roundtrip_2_h_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"8a8a5697ed766fd6d60dceef333ccd432ee3a22e286f964a3c0ad7407eccb965454c0aedb04db714ebc0aed08916d021"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_threshold"></a>

## Function `roundtrip_2_threshold`

Vector 2: Threshold


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_threshold">roundtrip_2_threshold</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_threshold">roundtrip_2_threshold</a>(): u64 { 2 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_total_weight"></a>

## Function `roundtrip_2_total_weight`

Vector 2: Total weight


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_total_weight">roundtrip_2_total_weight</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_total_weight">roundtrip_2_total_weight</a>(): u64 { 4 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_indices"></a>

## Function `roundtrip_2_validator_indices`

Vector 2: Validator indices [0, 1, 2, 3]


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_indices">roundtrip_2_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_indices">roundtrip_2_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0, 1, 2, 3]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_weights"></a>

## Function `roundtrip_2_validator_weights`

Vector 2: Validator weights [1, 1, 1, 1]


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_weights">roundtrip_2_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_weights">roundtrip_2_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1, 1, 1, 1]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_dk_shares"></a>

## Function `roundtrip_2_dk_shares`

Vector 2: DK shares in G1 (4 shares)


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_dk_shares">roundtrip_2_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_dk_shares">roundtrip_2_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        x"b60d3ddf9b0a596918276b9016723540aab31e024153a3f5b214150022e34444c2a8c6a08f2bc68f289f715ffdb93192",
        x"864b4a9211b0eb77dab0df356f124a86f946ed9004de11e0c0ae1ac529301eef9adf443373f9ab0a33d393db38528337",
        x"8f6e359c3d6d94ba5420695da8b441f1268d0bbe616bad8820125730a60eb589ddbf94e33c425e0a7871ec3d1a2ef520",
        x"a6edf2ef32d2e25a10b1374dd1c758545036f743f556252ae3451a489d454414820e4d9e287965a2c4a318e3d409c6c0"
    ]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_reconstructed_dk"></a>

## Function `roundtrip_2_reconstructed_dk`

Vector 2: Reconstructed DK


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_reconstructed_dk">roundtrip_2_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_reconstructed_dk">roundtrip_2_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"9266dbe7f2c3a699fa1767d30c56507f792ee751fe457c6958f04dd8e0b6333d71fa609664b2cfada489547761effeb1"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_plaintext"></a>

## Function `roundtrip_2_plaintext`

Vector 2: Plaintext


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_plaintext">roundtrip_2_plaintext</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_plaintext">roundtrip_2_plaintext</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"5365636f6e6420746573742063617365207769746820342076616c696461746f7273"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_ciphertext_u"></a>

## Function `roundtrip_2_ciphertext_u`

Vector 2: Ciphertext U


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_ciphertext_u">roundtrip_2_ciphertext_u</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_ciphertext_u">roundtrip_2_ciphertext_u</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"8b6111ff088b03482f90b5bd55c3224c286964c1612749af366f9826ffacca8ffcef749a796097febd6d09c647a3ef6111ad5b6688b9c696f66c0489980f2cbd4a139518d606ddac4135db2a141bf54cb75ee8c83f6a897de0b4925933eac29e"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_ciphertext_v"></a>

## Function `roundtrip_2_ciphertext_v`

Vector 2: Ciphertext V


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_ciphertext_v">roundtrip_2_ciphertext_v</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_ciphertext_v">roundtrip_2_ciphertext_v</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"9b0eaa53e097b8ffd0978f238be9b256cea118c5e7221d74a00b3197fafcbdfef2c5"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_rng_seed"></a>

## Function `roundtrip_3_rng_seed`

Vector 3: RNG seed


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_rng_seed">roundtrip_3_rng_seed</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_rng_seed">roundtrip_3_rng_seed</a>(): u64 { 99999 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_msk"></a>

## Function `roundtrip_3_msk`

Vector 3: Master secret key


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_msk">roundtrip_3_msk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_msk">roundtrip_3_msk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"54efc669c86a245a0b3e15b52e957a432474d664ca824513cc3e489e1cb14007"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_mpk"></a>

## Function `roundtrip_3_mpk`

Vector 3: Master public key


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_mpk">roundtrip_3_mpk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_mpk">roundtrip_3_mpk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"986e5dc6d6b494fa5a5e8e1e2f1c5ddb22a021fd24c2117e3be5f2f9a844c04aa02c4270c09c0a7044c0f7e9780316a4102870266aab226c49949f8c069ac4858c9d1f5edb8ca90815e219072fe884b0a03fea5494a3e505498d3b1128996cb4"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_identity"></a>

## Function `roundtrip_3_identity`

Vector 3: Identity hash


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_identity">roundtrip_3_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_identity">roundtrip_3_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"de8e8b23a9e541393a1835326e2e07e51571d61dd42a0c9dd1c2dab16186986d"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_h_identity"></a>

## Function `roundtrip_3_h_identity`

Vector 3: H(identity) in G1


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_h_identity">roundtrip_3_h_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_h_identity">roundtrip_3_h_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"81d2a7a38c3852f64b1faf838f55a55fd03df97d8012b82f04e9925098516ac6bf6d8eb3fae98147ef41f522422b2a0d"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_threshold"></a>

## Function `roundtrip_3_threshold`

Vector 3: Threshold


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_threshold">roundtrip_3_threshold</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_threshold">roundtrip_3_threshold</a>(): u64 { 3 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_total_weight"></a>

## Function `roundtrip_3_total_weight`

Vector 3: Total weight


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_total_weight">roundtrip_3_total_weight</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_total_weight">roundtrip_3_total_weight</a>(): u64 { 5 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_indices"></a>

## Function `roundtrip_3_validator_indices`

Vector 3: Validator indices [0, 1, 2]


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_indices">roundtrip_3_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_indices">roundtrip_3_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0, 1, 2]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_weights"></a>

## Function `roundtrip_3_validator_weights`

Vector 3: Validator weights [2, 1, 2]


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_weights">roundtrip_3_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_weights">roundtrip_3_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[2, 1, 2]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_dk_shares"></a>

## Function `roundtrip_3_dk_shares`

Vector 3: DK shares in G1 (3 shares)


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_dk_shares">roundtrip_3_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_dk_shares">roundtrip_3_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        x"862b3a0396c945a81c83866c60cbe87968455e2b74f3213bd52164e0cded92ccd8f147111a23f2723960281d646a6bfd",
        x"8a96fb8faed4eedb44496140cf2a47c09a56eca0da3a61e8306d11554ca90ba06493ec1d4523cf39326451dbb67d4549",
        x"a87fb1ad947d3e962b21a184a763e91ada32fbf596bf6dafb5aabec78dc52766ed03d9a80ddd8122b79a4acf4f450918"
    ]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_reconstructed_dk"></a>

## Function `roundtrip_3_reconstructed_dk`

Vector 3: Reconstructed DK


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_reconstructed_dk">roundtrip_3_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_reconstructed_dk">roundtrip_3_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"afbaa0adb5ab4ef5d1c72e1aab6497f1fabd61d8f24a7c642593978e4ff18a080c10228fa8a61a8ec9f5ad0e15de1a76"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_plaintext"></a>

## Function `roundtrip_3_plaintext`

Vector 3: Plaintext


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_plaintext">roundtrip_3_plaintext</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_plaintext">roundtrip_3_plaintext</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"556e657175616c20776569676874732074657374205b322c312c325d"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_ciphertext_u"></a>

## Function `roundtrip_3_ciphertext_u`

Vector 3: Ciphertext U


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_ciphertext_u">roundtrip_3_ciphertext_u</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_ciphertext_u">roundtrip_3_ciphertext_u</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"94078b8f4d68ebfaa8f568badfd271f99013fecdc37c48a320b2e768c38ee61d510971efffd3d07b223bf00fd809ee6906a8a0029f44ba15597aa8c695088fb0c040a03f1ffae76513f0c200c386653e80dcab242fedf535c53867f2f1cef812"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_ciphertext_v"></a>

## Function `roundtrip_3_ciphertext_v`

Vector 3: Ciphertext V


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_ciphertext_v">roundtrip_3_ciphertext_v</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_ciphertext_v">roundtrip_3_ciphertext_v</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"c577135d44e15ef14761ae4d7857faece4a4c5287f57ae6f6969351b"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_rng_seed"></a>

## Function `roundtrip_4_rng_seed`

Vector 4: RNG seed


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_rng_seed">roundtrip_4_rng_seed</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_rng_seed">roundtrip_4_rng_seed</a>(): u64 { 11111 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_msk"></a>

## Function `roundtrip_4_msk`

Vector 4: Master secret key


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_msk">roundtrip_4_msk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_msk">roundtrip_4_msk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"7f97d8fbbd275b2b59753888b49c4bb2cf8910cf6621959ff5c146d224a50214"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_mpk"></a>

## Function `roundtrip_4_mpk`

Vector 4: Master public key


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_mpk">roundtrip_4_mpk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_mpk">roundtrip_4_mpk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"b71acf0e8a5c46a29ed8803219fc90d9ea8aa6b92c511fadadfab7e86a8a53839fbd4fcf47be51fb04c05b54f3e721d20da1f2f9e2d5c0b3f641a3cf2873344fd05d2d6cb6ec48d7e6a379d974b0cdf23aa9175b336f36916ec7f9dc0823fde7"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_identity"></a>

## Function `roundtrip_4_identity`

Vector 4: Identity hash


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_identity">roundtrip_4_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_identity">roundtrip_4_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"5758871061dc3844df56aa064d31f0bc171b005ea70e9b1f866d17308e7db2a1"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_h_identity"></a>

## Function `roundtrip_4_h_identity`

Vector 4: H(identity) in G1


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_h_identity">roundtrip_4_h_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_h_identity">roundtrip_4_h_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"a62edbe7b9728a6569fc67d2d6558c2295eef25a83588ab008f58fb641dc59b613cbb967f602ee722641cf10123121d3"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_threshold"></a>

## Function `roundtrip_4_threshold`

Vector 4: Threshold


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_threshold">roundtrip_4_threshold</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_threshold">roundtrip_4_threshold</a>(): u64 { 3 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_total_weight"></a>

## Function `roundtrip_4_total_weight`

Vector 4: Total weight


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_total_weight">roundtrip_4_total_weight</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_total_weight">roundtrip_4_total_weight</a>(): u64 { 8 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_indices"></a>

## Function `roundtrip_4_validator_indices`

Vector 4: Validator indices [0, 1, 2, 3]


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_indices">roundtrip_4_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_indices">roundtrip_4_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0, 1, 2, 3]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_weights"></a>

## Function `roundtrip_4_validator_weights`

Vector 4: Validator weights [2, 3, 2, 1]


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_weights">roundtrip_4_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_weights">roundtrip_4_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[2, 3, 2, 1]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_dk_shares"></a>

## Function `roundtrip_4_dk_shares`

Vector 4: DK shares in G1 (4 shares)


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_dk_shares">roundtrip_4_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_dk_shares">roundtrip_4_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        x"a02a2adabcf4c3ec4a4c40dfb1a6a34bd4efba443dea05867cd88a3b45238fcc48721a83e95f5821826f8b5207846a09",
        x"b3b01331a629cfda8d731263e34bac8c4d9c794754499ae4be6fcb145fa793c21acdf0f1784684d5de286adc1f1f9f0a",
        x"8ca474497c7d328b4b8899041d25c69946607c9cdadf6ff7ccd05e3bf2acd29d27a156ec753f5d263fb98ab6917a061d",
        x"b3119a32f5b4d0ac7fa283e13cb7e09e971778d889b02e281e74d5b56b2eb73f6a0ca0d30549717bad57f3e8102b1079"
    ]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_reconstructed_dk"></a>

## Function `roundtrip_4_reconstructed_dk`

Vector 4: Reconstructed DK


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_reconstructed_dk">roundtrip_4_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_reconstructed_dk">roundtrip_4_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"af3b251c96136842613b45d09a222986330395b5699449526fca7405e8d6bb2743306b193cf10bfdbcfe8cdc468eaa8d"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_plaintext"></a>

## Function `roundtrip_4_plaintext`

Vector 4: Plaintext


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_plaintext">roundtrip_4_plaintext</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_plaintext">roundtrip_4_plaintext</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"556e657175616c2077656967687473205b322c332c322c315d20342076616c696461746f7273"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_ciphertext_u"></a>

## Function `roundtrip_4_ciphertext_u`

Vector 4: Ciphertext U


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_ciphertext_u">roundtrip_4_ciphertext_u</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_ciphertext_u">roundtrip_4_ciphertext_u</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"b13212a23d6f148499fa94c3c0994ba0fe5886a5d82247de8c3dccea06b6bff8326e59ab3009aa1fe9cec17faa6896af0989d078c87f42399efbbd48ce44308558b835e10bb17337dea70fe8387a3b3275e38282bca87bbb95c0ca934a47f584"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_ciphertext_v"></a>

## Function `roundtrip_4_ciphertext_v`

Vector 4: Ciphertext V


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_ciphertext_v">roundtrip_4_ciphertext_v</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_ciphertext_v">roundtrip_4_ciphertext_v</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    x"11050ed8b7027dedc1c064b7d39a059e7ddfd029d0410fa4c7be068b78f162cada64b21315a4"
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_get_identity_hash"></a>

## Function `get_identity_hash`

Get the identity hash fixture by timelock_id and deadline_us
Returns empty vector if not found


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_identity_hash">get_identity_hash</a>(timelock_id: u64, deadline_us: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_identity_hash">get_identity_hash</a>(timelock_id: u64, deadline_us: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>if</b> (timelock_id == 0 && deadline_us == 1000000000000) {
        <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_0_1000000000000">identity_0_1000000000000</a>()
    } <b>else</b> <b>if</b> (timelock_id == 1 && deadline_us == 1000000000000) {
        <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_1_1000000000000">identity_1_1000000000000</a>()
    } <b>else</b> <b>if</b> (timelock_id == 0 && deadline_us == 2000000000000) {
        <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_0_2000000000000">identity_0_2000000000000</a>()
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
    }
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_get_roundtrip_validator_indices"></a>

## Function `get_roundtrip_validator_indices`

Get roundtrip vector by index (1-4)
Returns validator indices for the given roundtrip


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_validator_indices">get_roundtrip_validator_indices</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_validator_indices">get_roundtrip_validator_indices</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>if</b> (index == 1) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_indices">roundtrip_1_validator_indices</a>()
    <b>else</b> <b>if</b> (index == 2) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_indices">roundtrip_2_validator_indices</a>()
    <b>else</b> <b>if</b> (index == 3) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_indices">roundtrip_3_validator_indices</a>()
    <b>else</b> <b>if</b> (index == 4) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_indices">roundtrip_4_validator_indices</a>()
    <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_get_roundtrip_validator_weights"></a>

## Function `get_roundtrip_validator_weights`

Get roundtrip vector by index (1-4)
Returns validator weights for the given roundtrip


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_validator_weights">get_roundtrip_validator_weights</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_validator_weights">get_roundtrip_validator_weights</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; {
    <b>if</b> (index == 1) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_weights">roundtrip_1_validator_weights</a>()
    <b>else</b> <b>if</b> (index == 2) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_weights">roundtrip_2_validator_weights</a>()
    <b>else</b> <b>if</b> (index == 3) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_weights">roundtrip_3_validator_weights</a>()
    <b>else</b> <b>if</b> (index == 4) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_weights">roundtrip_4_validator_weights</a>()
    <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_get_roundtrip_dk_shares"></a>

## Function `get_roundtrip_dk_shares`

Get roundtrip vector by index (1-4)
Returns DK shares for the given roundtrip


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_dk_shares">get_roundtrip_dk_shares</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_dk_shares">get_roundtrip_dk_shares</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    <b>if</b> (index == 1) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_dk_shares">roundtrip_1_dk_shares</a>()
    <b>else</b> <b>if</b> (index == 2) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_dk_shares">roundtrip_2_dk_shares</a>()
    <b>else</b> <b>if</b> (index == 3) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_dk_shares">roundtrip_3_dk_shares</a>()
    <b>else</b> <b>if</b> (index == 4) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_dk_shares">roundtrip_4_dk_shares</a>()
    <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_get_roundtrip_identity"></a>

## Function `get_roundtrip_identity`

Get roundtrip vector by index (1-4)
Returns identity for the given roundtrip


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_identity">get_roundtrip_identity</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_identity">get_roundtrip_identity</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>if</b> (index == 1) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_identity">roundtrip_1_identity</a>()
    <b>else</b> <b>if</b> (index == 2) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_identity">roundtrip_2_identity</a>()
    <b>else</b> <b>if</b> (index == 3) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_identity">roundtrip_3_identity</a>()
    <b>else</b> <b>if</b> (index == 4) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_identity">roundtrip_4_identity</a>()
    <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_get_roundtrip_total_weight"></a>

## Function `get_roundtrip_total_weight`

Get roundtrip vector by index (1-4)
Returns total weight for the given roundtrip


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_total_weight">get_roundtrip_total_weight</a>(index: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_total_weight">get_roundtrip_total_weight</a>(index: u64): u64 {
    <b>if</b> (index == 1) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_total_weight">roundtrip_1_total_weight</a>()
    <b>else</b> <b>if</b> (index == 2) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_total_weight">roundtrip_2_total_weight</a>()
    <b>else</b> <b>if</b> (index == 3) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_total_weight">roundtrip_3_total_weight</a>()
    <b>else</b> <b>if</b> (index == 4) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_total_weight">roundtrip_4_total_weight</a>()
    <b>else</b> 0
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_get_roundtrip_threshold"></a>

## Function `get_roundtrip_threshold`

Get roundtrip vector by index (1-4)
Returns threshold for the given roundtrip


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_threshold">get_roundtrip_threshold</a>(index: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_threshold">get_roundtrip_threshold</a>(index: u64): u64 {
    <b>if</b> (index == 1) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_threshold">roundtrip_1_threshold</a>()
    <b>else</b> <b>if</b> (index == 2) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_threshold">roundtrip_2_threshold</a>()
    <b>else</b> <b>if</b> (index == 3) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_threshold">roundtrip_3_threshold</a>()
    <b>else</b> <b>if</b> (index == 4) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_threshold">roundtrip_4_threshold</a>()
    <b>else</b> 0
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_get_roundtrip_reconstructed_dk"></a>

## Function `get_roundtrip_reconstructed_dk`

Get roundtrip vector by index (1-4)
Returns reconstructed DK for the given roundtrip


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_reconstructed_dk">get_roundtrip_reconstructed_dk</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_reconstructed_dk">get_roundtrip_reconstructed_dk</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>if</b> (index == 1) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_reconstructed_dk">roundtrip_1_reconstructed_dk</a>()
    <b>else</b> <b>if</b> (index == 2) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_reconstructed_dk">roundtrip_2_reconstructed_dk</a>()
    <b>else</b> <b>if</b> (index == 3) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_reconstructed_dk">roundtrip_3_reconstructed_dk</a>()
    <b>else</b> <b>if</b> (index == 4) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_reconstructed_dk">roundtrip_4_reconstructed_dk</a>()
    <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[]
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
