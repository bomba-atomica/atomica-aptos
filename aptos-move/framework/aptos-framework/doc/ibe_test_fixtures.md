
<a id="0x1_ibe_test_fixtures"></a>

# Module `0x1::ibe_test_fixtures`

Copyright © Aptos Foundation
SPDX-License-Identifier: Apache-2.0


-  [Constants](#@Constants_0)
-  [Function `identity_to_g1`](#0x1_ibe_test_fixtures_identity_to_g1)
-  [Function `parse_g2_hex`](#0x1_ibe_test_fixtures_parse_g2_hex)
-  [Function `parse_g1_hex`](#0x1_ibe_test_fixtures_parse_g1_hex)


<pre><code><b>use</b> <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">0x1::bls12381_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
</code></pre>



<a id="@Constants_0"></a>

## Constants


<a id="0x1_ibe_test_fixtures_BLS_WVUF_DST"></a>



<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_BLS_WVUF_DST">BLS_WVUF_DST</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [65, 80, 84, 79, 83, 95, 66, 76, 83, 95, 87, 86, 85, 70, 95, 68, 83, 84];
</code></pre>



<a id="0x1_ibe_test_fixtures_FIXTURE_CIPHERTEXT_U_HEX"></a>

Ciphertext U (G2 point) - 96 bytes


<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_FIXTURE_CIPHERTEXT_U_HEX">FIXTURE_CIPHERTEXT_U_HEX</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [160, 129, 167, 13, 77, 96, 231, 106, 234, 78, 77, 76, 206, 55, 49, 183, 65, 208, 186, 101, 39, 131, 7, 30, 195, 173, 138, 157, 205, 72, 129, 117, 20, 232, 38, 145, 91, 147, 239, 207, 104, 240, 200, 203, 190, 10, 137, 32, 14, 110, 9, 195, 132, 232, 99, 239, 94, 229, 76, 143, 143, 153, 197, 186, 105, 72, 8, 99, 16, 157, 34, 157, 243, 224, 12, 209, 190, 61, 5, 228, 35, 139, 45, 240, 219, 193, 138, 255, 75, 147, 154, 184, 141, 197, 97, 12];
</code></pre>



<a id="0x1_ibe_test_fixtures_FIXTURE_CIPHERTEXT_V_HEX"></a>

Ciphertext V (encrypted message) - 21 bytes


<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_FIXTURE_CIPHERTEXT_V_HEX">FIXTURE_CIPHERTEXT_V_HEX</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [184, 71, 198, 73, 209, 75, 188, 219, 124, 81, 134, 175, 22, 27, 251, 144, 17, 107, 10, 77, 13, 227, 49, 70, 112, 68];
</code></pre>



<a id="0x1_ibe_test_fixtures_FIXTURE_DEADLINE_MICROSECONDS"></a>

Test deadline in microseconds (2024-01-01 01:00:00 UTC)


<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_FIXTURE_DEADLINE_MICROSECONDS">FIXTURE_DEADLINE_MICROSECONDS</a>: u64 = 1704070800000000;
</code></pre>



<a id="0x1_ibe_test_fixtures_FIXTURE_DK_G1_HEX"></a>

DK (Decryption Key) - G1 point, 48 bytes compressed


<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_FIXTURE_DK_G1_HEX">FIXTURE_DK_G1_HEX</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [138, 22, 182, 115, 182, 125, 200, 64, 94, 238, 165, 56, 117, 60, 141, 92, 183, 39, 219, 200, 228, 70, 70, 77, 61, 160, 136, 18, 217, 64, 97, 104, 143, 237, 110, 202, 45, 255, 215, 117, 224, 218, 230, 197, 59, 21, 255, 203];
</code></pre>



<a id="0x1_ibe_test_fixtures_FIXTURE_EPOCH"></a>

Test epoch for fixtures


<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_FIXTURE_EPOCH">FIXTURE_EPOCH</a>: u64 = 42;
</code></pre>



<a id="0x1_ibe_test_fixtures_FIXTURE_EXPECTED_DECRYPTED_HEX"></a>

Expected decrypted result (matches FIXTURE_MESSAGE)


<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_FIXTURE_EXPECTED_DECRYPTED_HEX">FIXTURE_EXPECTED_DECRYPTED_HEX</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [71, 111, 108, 100, 101, 110, 32, 86, 101, 99, 116, 111, 114, 32, 77, 101, 115, 115, 97, 103, 101, 32, 50, 48, 50, 52];
</code></pre>



<a id="0x1_ibe_test_fixtures_FIXTURE_IDENTITY_HASH"></a>

Identity hash: Keccak256(FIXTURE_IDENTITY_STRING)
Matches Rust: 6d68192a4097c6215fc53001590729f5f5d3f68e0449dcb94f76de2effc95f71


<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_FIXTURE_IDENTITY_HASH">FIXTURE_IDENTITY_HASH</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [109, 104, 25, 42, 64, 151, 198, 33, 95, 197, 48, 1, 89, 7, 41, 245, 245, 211, 246, 142, 4, 73, 220, 185, 79, 118, 222, 46, 255, 201, 95, 113];
</code></pre>



<a id="0x1_ibe_test_fixtures_FIXTURE_IDENTITY_STRING"></a>

Identity string: "timelock_id:42:deadline_timestamp_microseconds:1704070800000000"


<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_FIXTURE_IDENTITY_STRING">FIXTURE_IDENTITY_STRING</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [116, 105, 109, 101, 108, 111, 99, 107, 95, 105, 100, 58, 52, 50, 58, 100, 101, 97, 100, 108, 105, 110, 101, 95, 116, 105, 109, 101, 115, 116, 97, 109, 112, 95, 109, 105, 99, 114, 111, 115, 101, 99, 111, 110, 100, 115, 58, 49, 55, 48, 52, 48, 55, 48, 56, 48, 48, 48, 48, 48, 48, 48, 48];
</code></pre>



<a id="0x1_ibe_test_fixtures_FIXTURE_MESSAGE"></a>

Test message as bytes


<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_FIXTURE_MESSAGE">FIXTURE_MESSAGE</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [71, 111, 108, 100, 101, 110, 32, 86, 101, 99, 116, 111, 114, 32, 77, 101, 115, 115, 97, 103, 101, 32, 50, 48, 50, 52];
</code></pre>



<a id="0x1_ibe_test_fixtures_FIXTURE_MPK_G2_HEX"></a>

MPK (Master Public Key) - G2 point, 96 bytes compressed


<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_FIXTURE_MPK_G2_HEX">FIXTURE_MPK_G2_HEX</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [134, 68, 121, 205, 200, 187, 231, 163, 170, 49, 148, 165, 10, 253, 213, 228, 212, 94, 200, 95, 241, 253, 181, 133, 17, 70, 141, 97, 245, 23, 5, 247, 24, 251, 240, 9, 78, 92, 14, 116, 249, 82, 57, 242, 237, 210, 68, 41, 12, 140, 17, 193, 41, 86, 45, 23, 112, 165, 244, 219, 233, 90, 198, 11, 176, 108, 62, 60, 100, 93, 91, 30, 246, 92, 209, 162, 236, 6, 131, 61, 151, 69, 6, 72, 119, 135, 191, 221, 83, 180, 56, 4, 1, 16, 197, 205];
</code></pre>



<a id="0x1_ibe_test_fixtures_FIXTURE_MSK_HEX"></a>

MSK (Master Secret Key) as hex - 32 bytes


<pre><code><b>const</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_FIXTURE_MSK_HEX">FIXTURE_MSK_HEX</a>: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; = [153, 73, 251, 135, 250, 195, 1, 250, 133, 45, 117, 127, 237, 127, 163, 207, 9, 57, 40, 133, 0, 137, 118, 101, 131, 215, 238, 234, 148, 55, 109, 1];
</code></pre>



<a id="0x1_ibe_test_fixtures_identity_to_g1"></a>

## Function `identity_to_g1`

Map identity bytes to G1 point using hash_to_curve


<pre><code><b>public</b> <b>fun</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_identity_to_g1">identity_to_g1</a>(identity: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_identity_to_g1">identity_to_g1</a>(identity: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Element&lt;G1&gt; {
    <b>let</b> msg_with_h = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> msg_with_h, 72); // 'H'
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> msg_with_h, 40); // '('
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> msg_with_h, 109); // 'm'
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> msg_with_h, 41); // ')'
    <b>let</b> i = 0;
    <b>while</b> (i &lt; <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(identity)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> msg_with_h, *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(identity, i));
        i = i + 1;
    };
    <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_hash_to">crypto_algebra::hash_to</a>&lt;G1, HashG1XmdSha256SswuRo&gt;(&<a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_BLS_WVUF_DST">BLS_WVUF_DST</a>, &msg_with_h)
}
</code></pre>



</details>

<a id="0x1_ibe_test_fixtures_parse_g2_hex"></a>

## Function `parse_g2_hex`

Parse G2 hex to Element


<pre><code><b>public</b> <b>fun</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_parse_g2_hex">parse_g2_hex</a>(hex: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G2">bls12381_algebra::G2</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_parse_g2_hex">parse_g2_hex</a>(hex: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Element&lt;G2&gt; {
    <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;G2, FormatG2Compr&gt;(hex).extract()
}
</code></pre>



</details>

<a id="0x1_ibe_test_fixtures_parse_g1_hex"></a>

## Function `parse_g1_hex`

Parse G1 hex to Element


<pre><code><b>public</b> <b>fun</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_parse_g1_hex">parse_g1_hex</a>(hex: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_Element">crypto_algebra::Element</a>&lt;<a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra_G1">bls12381_algebra::G1</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_test_fixtures.md#0x1_ibe_test_fixtures_parse_g1_hex">parse_g1_hex</a>(hex: &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;): Element&lt;G1&gt; {
    <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra_deserialize">crypto_algebra::deserialize</a>&lt;G1, FormatG1Compr&gt;(hex).extract()
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
