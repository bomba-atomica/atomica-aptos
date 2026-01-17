
<a id="0x1_ibe_config"></a>

# Module `0x1::ibe_config`

IBE (Identity-Based Encryption) configuration module.

This module stores the Master Public Key (MPK) derived from the DKG transcript,
enabling clients to perform timelock encryption using the Boneh-Franklin IBE scheme.

The MPK is a G2 point (96 bytes compressed) that is updated after each successful DKG.
Clients can query the MPK via view functions to encrypt messages that can only be
decrypted after validators reveal the corresponding decryption key.


-  [Resource `IBEPublicParams`](#0x1_ibe_config_IBEPublicParams)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_ibe_config_initialize)
-  [Function `set_mpk`](#0x1_ibe_config_set_mpk)
-  [Function `get_mpk`](#0x1_ibe_config_get_mpk)
-  [Function `get_epoch`](#0x1_ibe_config_get_epoch)
-  [Function `is_ready`](#0x1_ibe_config_is_ready)


<pre><code><b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_ibe_config_IBEPublicParams"></a>

## Resource `IBEPublicParams`

Stores IBE public parameters, updated after each successful DKG.


<pre><code><b>struct</b> <a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mpk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 Master Public Key (G2, 96 bytes compressed)
 This is the dealt public key from the DKG transcript
</dd>
<dt>
<code>epoch: u64</code>
</dt>
<dd>
 Epoch when this MPK was generated
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_ibe_config_E_IBE_NOT_READY"></a>

IBE is not ready (MPK not yet set)


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_IBE_NOT_READY">E_IBE_NOT_READY</a>: u64 = 2;
</code></pre>



<a id="0x1_ibe_config_E_INVALID_MPK_LENGTH"></a>

MPK length must be exactly 96 bytes (G2 compressed)


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_INVALID_MPK_LENGTH">E_INVALID_MPK_LENGTH</a>: u64 = 1;
</code></pre>



<a id="0x1_ibe_config_G2_COMPRESSED_LENGTH"></a>

Expected length of a compressed G2 point


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_G2_COMPRESSED_LENGTH">G2_COMPRESSED_LENGTH</a>: u64 = 96;
</code></pre>



<a id="0x1_ibe_config_initialize"></a>

## Function `initialize`

Called in genesis to initialize IBE config.


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_initialize">initialize</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(aptos_framework, <a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a> {
            mpk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            epoch: 0,
        });
    }
}
</code></pre>



</details>

<a id="0x1_ibe_config_set_mpk"></a>

## Function `set_mpk`

Update MPK after DKG completes.
Called by reconfiguration_with_dkg when a new DKG transcript is finalized.

The MPK must be exactly 96 bytes (compressed G2 point).


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_set_mpk">set_mpk</a>(mpk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, epoch: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_set_mpk">set_mpk</a>(mpk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, epoch: u64) <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a> {
    <b>assert</b>!(
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&mpk) == <a href="ibe_config.md#0x1_ibe_config_G2_COMPRESSED_LENGTH">G2_COMPRESSED_LENGTH</a>,
        <a href="ibe_config.md#0x1_ibe_config_E_INVALID_MPK_LENGTH">E_INVALID_MPK_LENGTH</a>
    );
    <b>let</b> params = <b>borrow_global_mut</b>&lt;<a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a>&gt;(@aptos_framework);
    params.mpk = mpk;
    params.epoch = epoch;
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_mpk"></a>

## Function `get_mpk`

Get the current Master Public Key.
Returns an empty vector if IBE is not yet initialized with a valid MPK.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_mpk">get_mpk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_mpk">get_mpk</a>(): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a>&gt;(@aptos_framework)) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>()
    };
    <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a>&gt;(@aptos_framework).mpk
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_epoch"></a>

## Function `get_epoch`

Get the epoch when the current MPK was set.
Returns 0 if IBE is not yet initialized.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_epoch">get_epoch</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_epoch">get_epoch</a>(): u64 <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a>&gt;(@aptos_framework)) {
        <b>return</b> 0
    };
    <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a>&gt;(@aptos_framework).epoch
}
</code></pre>



</details>

<a id="0x1_ibe_config_is_ready"></a>

## Function `is_ready`

Check if IBE is ready for encryption.
Returns true if a valid MPK (96 bytes) has been set.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_is_ready">is_ready</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_is_ready">is_ready</a>(): bool <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a>&gt;(@aptos_framework)) {
        <b>return</b> <b>false</b>
    };
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&<b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a>&gt;(@aptos_framework).mpk) == <a href="ibe_config.md#0x1_ibe_config_G2_COMPRESSED_LENGTH">G2_COMPRESSED_LENGTH</a>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
