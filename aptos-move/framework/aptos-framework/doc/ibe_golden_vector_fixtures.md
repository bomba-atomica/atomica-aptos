
<a id="0x1_ibe_golden_vector_fixtures"></a>

# Module `0x1::ibe_golden_vector_fixtures`

IBE Golden Vector Fixtures (Test-Only)

This module provides access to golden test vectors for IBE testing.
All values are encoded as hex strings matching the JSON golden vectors.

The golden vectors are generated from real PVSS transcripts and include:
- Identity computation test vectors
- IBE roundtrip test vectors with full encryption/decryption verification


-  [Function `identity_0_1000000000000`](#0x1_ibe_golden_vector_fixtures_identity_0_1000000000000)
-  [Function `h_identity_0_1000000000000`](#0x1_ibe_golden_vector_fixtures_h_identity_0_1000000000000)
-  [Function `identity_1_1000000000000`](#0x1_ibe_golden_vector_fixtures_identity_1_1000000000000)
-  [Function `h_identity_1_1000000000000`](#0x1_ibe_golden_vector_fixtures_h_identity_1_1000000000000)
-  [Function `identity_0_2000000000000`](#0x1_ibe_golden_vector_fixtures_identity_0_2000000000000)
-  [Function `h_identity_0_2000000000000`](#0x1_ibe_golden_vector_fixtures_h_identity_0_2000000000000)
-  [Function `roundtrip_1_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_identity)
-  [Function `roundtrip_1_threshold`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_threshold)
-  [Function `roundtrip_1_total_weight`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_total_weight)
-  [Function `roundtrip_1_validator_indices`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_indices)
-  [Function `roundtrip_1_validator_weights`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_weights)
-  [Function `roundtrip_1_dk_shares`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_dk_shares)
-  [Function `roundtrip_1_reconstructed_dk`](#0x1_ibe_golden_vector_fixtures_roundtrip_1_reconstructed_dk)
-  [Function `roundtrip_2_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_identity)
-  [Function `roundtrip_2_threshold`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_threshold)
-  [Function `roundtrip_2_total_weight`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_total_weight)
-  [Function `roundtrip_2_validator_indices`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_indices)
-  [Function `roundtrip_2_validator_weights`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_weights)
-  [Function `roundtrip_2_dk_shares`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_dk_shares)
-  [Function `roundtrip_2_reconstructed_dk`](#0x1_ibe_golden_vector_fixtures_roundtrip_2_reconstructed_dk)
-  [Function `roundtrip_3_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_identity)
-  [Function `roundtrip_3_threshold`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_threshold)
-  [Function `roundtrip_3_total_weight`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_total_weight)
-  [Function `roundtrip_3_validator_indices`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_indices)
-  [Function `roundtrip_3_validator_weights`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_weights)
-  [Function `roundtrip_3_dk_shares`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_dk_shares)
-  [Function `roundtrip_3_reconstructed_dk`](#0x1_ibe_golden_vector_fixtures_roundtrip_3_reconstructed_dk)
-  [Function `roundtrip_4_identity`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_identity)
-  [Function `roundtrip_4_threshold`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_threshold)
-  [Function `roundtrip_4_total_weight`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_total_weight)
-  [Function `roundtrip_4_validator_indices`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_indices)
-  [Function `roundtrip_4_validator_weights`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_weights)
-  [Function `roundtrip_4_dk_shares`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_dk_shares)
-  [Function `roundtrip_4_reconstructed_dk`](#0x1_ibe_golden_vector_fixtures_roundtrip_4_reconstructed_dk)
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



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_0_1000000000000">identity_0_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_0_1000000000000">identity_0_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"cd2f0ecdda375c87027bcbd9a8b429057062065239267bfd2496e844b81f7355" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_h_identity_0_1000000000000"></a>

## Function `h_identity_0_1000000000000`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_0_1000000000000">h_identity_0_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_0_1000000000000">h_identity_0_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"95502e8ee330870f5002c4cfc834467a507fcadaa49f9da04c539f322858284e627937fb234993f78dd5869b57419aa9" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_identity_1_1000000000000"></a>

## Function `identity_1_1000000000000`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_1_1000000000000">identity_1_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_1_1000000000000">identity_1_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"1235cfe3eb61fd7c5aa850477532822bbef4d9ca30decc2ca4405ebb30d5cab1" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_h_identity_1_1000000000000"></a>

## Function `h_identity_1_1000000000000`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_1_1000000000000">h_identity_1_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_1_1000000000000">h_identity_1_1000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"aa5dffd79cb804f313d5c2e882183fd8477d3f9a6424cb56fa373e2c7ce0d0904383af53a5680538423d02ac61eaa210" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_identity_0_2000000000000"></a>

## Function `identity_0_2000000000000`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_0_2000000000000">identity_0_2000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_identity_0_2000000000000">identity_0_2000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"ee623d0d70b0a085f0e91a197853c627a9cd7b24cdac79d2782e912e35ddefe6" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_h_identity_0_2000000000000"></a>

## Function `h_identity_0_2000000000000`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_0_2000000000000">h_identity_0_2000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_h_identity_0_2000000000000">h_identity_0_2000000000000</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"b6ef562742ce6af3dcd6335b07bbbd38a36c51d8f13a470a6da39f916ffa41ad3651f454820244a8a38308ef99ec3521" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_identity"></a>

## Function `roundtrip_1_identity`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_identity">roundtrip_1_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_identity">roundtrip_1_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"e3fb053eedce65f62fcbbfd56fe9bbec0abe3bfcc3c5ec54a913bd50c51a5dfe" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_threshold"></a>

## Function `roundtrip_1_threshold`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_threshold">roundtrip_1_threshold</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_threshold">roundtrip_1_threshold</a>(): u64 { 3 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_total_weight"></a>

## Function `roundtrip_1_total_weight`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_total_weight">roundtrip_1_total_weight</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_total_weight">roundtrip_1_total_weight</a>(): u64 { 5 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_indices"></a>

## Function `roundtrip_1_validator_indices`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_indices">roundtrip_1_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_indices">roundtrip_1_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; { <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0, 1, 2, 3, 4] }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_weights"></a>

## Function `roundtrip_1_validator_weights`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_weights">roundtrip_1_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_validator_weights">roundtrip_1_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; { <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1, 1, 1, 1, 1] }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_dk_shares"></a>

## Function `roundtrip_1_dk_shares`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_dk_shares">roundtrip_1_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_dk_shares">roundtrip_1_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt; {
    <b>let</b> v = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[x"9722f3fe074ff0467af66bbb6564aaeec41ac369dbc55a520e1197e2cabd01489fac2b5260c049e9ed26fdd872391d2d"]);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[x"b0cc6092fc45df1b1318952c08dd2a877b5b19deb725b63cfc41c48e84da6e8f77600efe4d38aac33273a87775112a31"]);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[x"ac20a63c6b62ed15a3424d85af25eb006795eba2172996529afbb70d29d1a57066a37ee1c98dea843fa2c998cb6b3919"]);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[x"b42700b02c7997b35f244481afa657a601e915046aed0dee5dcde214e91efdecbf3fcc9d21c11907aac23a84c43927d2"]);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[x"891c64345686bd154a277a7540ed7c32ad7a0abe30a40586515bf07738e6f63f054b6a55d6d5a2288da37f15335618bf"]);
    v
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_1_reconstructed_dk"></a>

## Function `roundtrip_1_reconstructed_dk`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_reconstructed_dk">roundtrip_1_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_reconstructed_dk">roundtrip_1_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"a46066de7604d2d51825cd139d720523d5584654cda567f80c3779304d2dec8842b41a7feec0db2a202409f45492593c" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_identity"></a>

## Function `roundtrip_2_identity`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_identity">roundtrip_2_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_identity">roundtrip_2_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"fe35f39be89aa3d57c43d37f0a4ecb02908c7a82c877eaf5c6517224aa09c46c" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_threshold"></a>

## Function `roundtrip_2_threshold`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_threshold">roundtrip_2_threshold</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_threshold">roundtrip_2_threshold</a>(): u64 { 2 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_total_weight"></a>

## Function `roundtrip_2_total_weight`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_total_weight">roundtrip_2_total_weight</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_total_weight">roundtrip_2_total_weight</a>(): u64 { 4 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_indices"></a>

## Function `roundtrip_2_validator_indices`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_indices">roundtrip_2_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_indices">roundtrip_2_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; { <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0, 1, 2, 3] }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_weights"></a>

## Function `roundtrip_2_validator_weights`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_weights">roundtrip_2_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_validator_weights">roundtrip_2_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; { <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[1, 1, 1, 1] }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_dk_shares"></a>

## Function `roundtrip_2_dk_shares`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_dk_shares">roundtrip_2_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_dk_shares">roundtrip_2_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt; {
    <b>let</b> v = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[x"b60d3ddf9b0a596918276b9016723540aab31e024153a3f5b214150022e34444c2a8c6a08f2bc68f289f715ffdb93192"]);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[x"864b4a9211b0eb77dab0df356f124a86f946ed9004de11e0c0ae1ac529301eef9adf443373f9ab0a33d393db38528337"]);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[x"8f6e359c3d6d94ba5420695da8b441f1268d0bbe616bad8820125730a60eb589ddbf94e33c425e0a7871ec3d1a2ef520"]);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[x"a6edf2ef32d2e25a10b1374dd1c758545036f743f556252ae3451a489d454414820e4d9e287965a2c4a318e3d409c6c0"]);
    v
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_2_reconstructed_dk"></a>

## Function `roundtrip_2_reconstructed_dk`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_reconstructed_dk">roundtrip_2_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_reconstructed_dk">roundtrip_2_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"9266dbe7f2c3a699fa1767d30c56507f792ee751fe457c6958f04dd8e0b6333d71fa609664b2cfada489547761effeb1" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_identity"></a>

## Function `roundtrip_3_identity`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_identity">roundtrip_3_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_identity">roundtrip_3_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"de8e8b23a9e541393a1835326e2e07e51571d61dd42a0c9dd1c2dab16186986d" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_threshold"></a>

## Function `roundtrip_3_threshold`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_threshold">roundtrip_3_threshold</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_threshold">roundtrip_3_threshold</a>(): u64 { 3 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_total_weight"></a>

## Function `roundtrip_3_total_weight`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_total_weight">roundtrip_3_total_weight</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_total_weight">roundtrip_3_total_weight</a>(): u64 { 5 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_indices"></a>

## Function `roundtrip_3_validator_indices`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_indices">roundtrip_3_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_indices">roundtrip_3_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; { <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0, 1, 2] }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_weights"></a>

## Function `roundtrip_3_validator_weights`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_weights">roundtrip_3_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_validator_weights">roundtrip_3_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; { <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[2, 1, 2] }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_dk_shares"></a>

## Function `roundtrip_3_dk_shares`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_dk_shares">roundtrip_3_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_dk_shares">roundtrip_3_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt; {
    <b>let</b> v = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;();
    // Validator 0 (weight 2)
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        x"8723f166aabb441baeb0df1eddfd332c3f5ec3a0b41ae92c54aee283021deb98ac79248dbabd5244bde583b553a4222b",
        x"aff1dc2c26fcb7f48a61f27e34933e8818c3e8f3905ee9cda4956b89d343bc751ed39be795a7762a5aefe742ef192836"
    ]);
    // Validator 1 (weight 1)
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        x"8a96fb8faed4eedb44496140cf2a47c09a56eca0da3a61e8306d11554ca90ba06493ec1d4523cf39326451dbb67d4549"
    ]);
    // Validator 2 (weight 2)
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        x"aff15e922d0418a29c420f384db7524fdf52bfdb16b34710c07b38a40167711c660af07d6261a031a264b78acc63214f",
        x"a24237a89cf507e05553e93950cac155d24496c8d7b5568d4443223826916141c1517ff43873b0fec0a03613f90bec0a"
    ]);
    v
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_3_reconstructed_dk"></a>

## Function `roundtrip_3_reconstructed_dk`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_reconstructed_dk">roundtrip_3_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_reconstructed_dk">roundtrip_3_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"afbaa0adb5ab4ef5d1c72e1aab6497f1fabd61d8f24a7c642593978e4ff18a080c10228fa8a61a8ec9f5ad0e15de1a76" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_identity"></a>

## Function `roundtrip_4_identity`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_identity">roundtrip_4_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_identity">roundtrip_4_identity</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"5758871061dc3844df56aa064d31f0bc171b005ea70e9b1f866d17308e7db2a1" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_threshold"></a>

## Function `roundtrip_4_threshold`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_threshold">roundtrip_4_threshold</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_threshold">roundtrip_4_threshold</a>(): u64 { 3 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_total_weight"></a>

## Function `roundtrip_4_total_weight`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_total_weight">roundtrip_4_total_weight</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_total_weight">roundtrip_4_total_weight</a>(): u64 { 8 }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_indices"></a>

## Function `roundtrip_4_validator_indices`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_indices">roundtrip_4_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_indices">roundtrip_4_validator_indices</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; { <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[0, 1, 2, 3] }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_weights"></a>

## Function `roundtrip_4_validator_weights`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_weights">roundtrip_4_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_validator_weights">roundtrip_4_validator_weights</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt; { <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[2, 3, 2, 1] }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_dk_shares"></a>

## Function `roundtrip_4_dk_shares`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_dk_shares">roundtrip_4_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_dk_shares">roundtrip_4_dk_shares</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt; {
    <b>let</b> v = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;();
    // Validator 0 (weight 2)
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        x"964205562095f0011f07f43394803970b42f61c373e2c6cb1907706691456bc9171f1ea4bc99f0f970e7e9780a11e13a",
        x"89fac2b5260c049e9ed26fdd872391d2d3a3c3065a7707e77b61f9da22987c2b5358043689f07d2cabd01489fac2b526"
    ]);
    // Validator 1 (weight 3)
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        x"8724f112e873a4b0877a760f381c618195f242551ecb785f09696ca0dfb1a6a34bd4efba443dea05867cd88a3b45238f",
        x"aff1dc2c26fcb7f48a61f27e34933e8818c3e8f3905ee9cda4956b89d343bc751ed39be795a7762a5aefe742ef192836",
        x"b3b01331a629cfda8d731263e34bac8c4d9c794754499ae4be6fcb145fa793c21acdf0f1784684d5de286adc1f1f9f0a"
    ]);
    // Validator 2 (weight 2)
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        x"8f6e359c3d6d94ba5420695da8b441f1268d0bbe616bad8820125730a60eb589ddbf94e33c425e0a7871ec3d1a2ef520",
        x"a24237a89cf507e05553e93950cac155d24496c8d7b5568d4443223826916141c1517ff43873b0fec0a03613f90bec0a"
    ]);
    // Validator 3 (weight 1)
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> v, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>[
        x"b3119a32f5b4d0ac7fa283e13cb7e09e971778d889b02e281e74d5b56b2eb73f6a0ca0d30549717bad57f3e8102b1079"
    ]);
    v
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_roundtrip_4_reconstructed_dk"></a>

## Function `roundtrip_4_reconstructed_dk`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_reconstructed_dk">roundtrip_4_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_reconstructed_dk">roundtrip_4_reconstructed_dk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; { x"af3b251c96136842613b45d09a222986330395b5699449526fca7405e8d6bb2743306b193cf10bfdbcfe8cdc468eaa8d" }
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_get_roundtrip_validator_indices"></a>

## Function `get_roundtrip_validator_indices`



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



<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_dk_shares">get_roundtrip_dk_shares</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_get_roundtrip_dk_shares">get_roundtrip_dk_shares</a>(index: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt; {
    <b>if</b> (index == 1) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_1_dk_shares">roundtrip_1_dk_shares</a>()
    <b>else</b> <b>if</b> (index == 2) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_2_dk_shares">roundtrip_2_dk_shares</a>()
    <b>else</b> <b>if</b> (index == 3) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_3_dk_shares">roundtrip_3_dk_shares</a>()
    <b>else</b> <b>if</b> (index == 4) <a href="ibe_golden_vector_fixtures.md#0x1_ibe_golden_vector_fixtures_roundtrip_4_dk_shares">roundtrip_4_dk_shares</a>()
    <b>else</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>()
}
</code></pre>



</details>

<a id="0x1_ibe_golden_vector_fixtures_get_roundtrip_identity"></a>

## Function `get_roundtrip_identity`



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
