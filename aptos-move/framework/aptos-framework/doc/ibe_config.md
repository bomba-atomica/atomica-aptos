
<a id="0x1_ibe_config"></a>

# Module `0x1::ibe_config`

IBE (Identity-Based Encryption) configuration and Timelock Registry module.

This module implements the on-chain components for Atomica's timelock encryption system.


<a id="@Core_Types_0"></a>

### Core Types


- <code><a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a></code>: Stores the Master Public Key (MPK) from DKG
- <code><a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a></code>: Manages all registered timelocks
- <code><a href="ibe_config.md#0x1_ibe_config_TimelockInfo">TimelockInfo</a></code>: Per-timelock state including shares and DK


<a id="@Workflow_1"></a>

### Workflow


1. Registration - User calls <code><a href="ibe_config.md#0x1_ibe_config_register_timelock">register_timelock</a>(deadline_us)</code> → gets <code>timelock_id</code>
2. Encryption - Client queries MPK and identity, encrypts with IBE
3. DKG - Validators run DKG, produce shares, publish MPK
4. Reveal - After deadline, validators submit scalar shares
5. Reconstruction - Native function reconstructs DK when threshold met
6. Decryption - Anyone queries DK, decrypts ciphertext


<a id="@Data_Format_2"></a>

### Data Format


The native function <code><a href="../../aptos-stdlib/doc/ibe.md#0x1_ibe_reconstruct_ibe_dk">ibe::reconstruct_ibe_dk</a></code> accepts:
- <code>validator_indices</code>: <code><a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code> - Validator indices (0-based)
- <code>scalar_shares</code>: <code><a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code> - 32-byte little-endian scalars
- <code>weights</code>: <code><a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code> - Full weights for ALL validators
- <code>total_weight</code>: <code>u64</code> - Sum of all weights
- <code>identity</code>: <code><a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code> - 32-byte IBE identity

Returns 48-byte compressed G1 (the reconstructed DK).


<a id="@Error_Codes_3"></a>

### Error Codes


| Code | Description |
|------|-------------|
| 1 | MPK must be 96 bytes (G2 compressed) |
| 2 | MPK not yet set by DKG |
| 3 | Deadline has not passed |
| 4 | Timelock ID not registered |
| 5 | DK not yet revealed |
| 6 | Threshold must be positive |
| 7 | Validator already submitted share |


    -  [Core Types](#@Core_Types_0)
    -  [Workflow](#@Workflow_1)
    -  [Data Format](#@Data_Format_2)
    -  [Error Codes](#@Error_Codes_3)
-  [Resource `IBEPublicParams`](#0x1_ibe_config_IBEPublicParams)
-  [Struct `TimelockInfo`](#0x1_ibe_config_TimelockInfo)
-  [Resource `TimelockRegistry`](#0x1_ibe_config_TimelockRegistry)
-  [Struct `TimelockRegistrationEvent`](#0x1_ibe_config_TimelockRegistrationEvent)
-  [Struct `TimelockRevealEvent`](#0x1_ibe_config_TimelockRevealEvent)
-  [Struct `TimelockExpiredEvent`](#0x1_ibe_config_TimelockExpiredEvent)
-  [Constants](#@Constants_4)
-  [Function `initialize`](#0x1_ibe_config_initialize)
-  [Function `set_mpk`](#0x1_ibe_config_set_mpk)
-  [Function `get_mpk`](#0x1_ibe_config_get_mpk)
-  [Function `get_epoch`](#0x1_ibe_config_get_epoch)
-  [Function `is_ready`](#0x1_ibe_config_is_ready)
-  [Function `initialize_timelock_registry`](#0x1_ibe_config_initialize_timelock_registry)
-  [Function `register_timelock`](#0x1_ibe_config_register_timelock)
-  [Function `submit_dk_shares`](#0x1_ibe_config_submit_dk_shares)
-  [Function `reconstruct_and_store_dk`](#0x1_ibe_config_reconstruct_and_store_dk)
-  [Function `remove_pending_timelock_id`](#0x1_ibe_config_remove_pending_timelock_id)
-  [Function `on_new_block`](#0x1_ibe_config_on_new_block)
-  [Function `get_timelock`](#0x1_ibe_config_get_timelock)
-  [Function `get_deadline`](#0x1_ibe_config_get_deadline)
-  [Function `get_identity`](#0x1_ibe_config_get_identity)
-  [Function `get_decryption_key`](#0x1_ibe_config_get_decryption_key)
-  [Function `is_revealed`](#0x1_ibe_config_is_revealed)
-  [Function `is_expired`](#0x1_ibe_config_is_expired)
-  [Function `get_next_timelock_id`](#0x1_ibe_config_get_next_timelock_id)
-  [Function `compute_identity`](#0x1_ibe_config_compute_identity)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">0x1::bls12381_algebra</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/ibe.md#0x1_ibe">0x1::ibe</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



<a id="0x1_ibe_config_IBEPublicParams"></a>

## Resource `IBEPublicParams`



<pre><code><b>struct</b> <a href="ibe_config.md#0x1_ibe_config_IBEPublicParams">IBEPublicParams</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>mpk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>epoch: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ibe_config_TimelockInfo"></a>

## Struct `TimelockInfo`



<pre><code><b>struct</b> <a href="ibe_config.md#0x1_ibe_config_TimelockInfo">TimelockInfo</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>timelock_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>deadline_us: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>identity: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>decryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>is_revealed: bool</code>
</dt>
<dd>

</dd>
<dt>
<code>share_count: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>reveal_threshold: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>validator_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>submitted_shares: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;&gt;</code>
</dt>
<dd>
 Nested shares: submitted_shares[validator_index][virtual_player_share]
</dd>
<dt>
<code>validator_weights: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>submitters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<b>address</b>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ibe_config_TimelockRegistry"></a>

## Resource `TimelockRegistry`



<pre><code><b>struct</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>timelocks: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="ibe_config.md#0x1_ibe_config_TimelockInfo">ibe_config::TimelockInfo</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>pending_timelock_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>next_timelock_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>registration_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistrationEvent">ibe_config::TimelockRegistrationEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>reveal_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRevealEvent">ibe_config::TimelockRevealEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>expired_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockExpiredEvent">ibe_config::TimelockExpiredEvent</a>&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ibe_config_TimelockRegistrationEvent"></a>

## Struct `TimelockRegistrationEvent`



<pre><code><b>struct</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistrationEvent">TimelockRegistrationEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>timelock_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>deadline_us: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>sender: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>timestamp_us: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ibe_config_TimelockRevealEvent"></a>

## Struct `TimelockRevealEvent`



<pre><code><b>struct</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRevealEvent">TimelockRevealEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>timelock_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>timestamp_us: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_ibe_config_TimelockExpiredEvent"></a>

## Struct `TimelockExpiredEvent`



<pre><code><b>struct</b> <a href="ibe_config.md#0x1_ibe_config_TimelockExpiredEvent">TimelockExpiredEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>timelock_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>timestamp_us: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_4"></a>

## Constants


<a id="0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_DENOMINATOR"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_DENOMINATOR">DEFAULT_REVEAL_THRESHOLD_DENOMINATOR</a>: u64 = 3;
</code></pre>



<a id="0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_NUMERATOR"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_NUMERATOR">DEFAULT_REVEAL_THRESHOLD_NUMERATOR</a>: u64 = 2;
</code></pre>



<a id="0x1_ibe_config_E_DEADLINE_NOT_PASSED"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_DEADLINE_NOT_PASSED">E_DEADLINE_NOT_PASSED</a>: u64 = 3;
</code></pre>



<a id="0x1_ibe_config_E_DECRYPTION_KEY_NOT_REVEALED"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_DECRYPTION_KEY_NOT_REVEALED">E_DECRYPTION_KEY_NOT_REVEALED</a>: u64 = 5;
</code></pre>



<a id="0x1_ibe_config_E_IBE_NOT_READY"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_IBE_NOT_READY">E_IBE_NOT_READY</a>: u64 = 2;
</code></pre>



<a id="0x1_ibe_config_E_INVALID_MPK_LENGTH"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_INVALID_MPK_LENGTH">E_INVALID_MPK_LENGTH</a>: u64 = 1;
</code></pre>



<a id="0x1_ibe_config_E_INVALID_THRESHOLD"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>: u64 = 6;
</code></pre>



<a id="0x1_ibe_config_E_SHARE_ALREADY_SUBMITTED"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_SHARE_ALREADY_SUBMITTED">E_SHARE_ALREADY_SUBMITTED</a>: u64 = 7;
</code></pre>



<a id="0x1_ibe_config_E_TIMELOCK_NOT_FOUND"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>: u64 = 4;
</code></pre>



<a id="0x1_ibe_config_G1_LENGTH"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_G1_LENGTH">G1_LENGTH</a>: u64 = 48;
</code></pre>



<a id="0x1_ibe_config_G2_COMPRESSED_LENGTH"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_G2_COMPRESSED_LENGTH">G2_COMPRESSED_LENGTH</a>: u64 = 96;
</code></pre>



<a id="0x1_ibe_config_initialize"></a>

## Function `initialize`



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
    };
}
</code></pre>



</details>

<a id="0x1_ibe_config_set_mpk"></a>

## Function `set_mpk`



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

<a id="0x1_ibe_config_initialize_timelock_registry"></a>

## Function `initialize_timelock_registry`



<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_initialize_timelock_registry">initialize_timelock_registry</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_initialize_timelock_registry">initialize_timelock_registry</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(aptos_framework, <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
            timelocks: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
            pending_timelock_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u64&gt;(),
            next_timelock_id: 0,
            registration_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistrationEvent">TimelockRegistrationEvent</a>&gt;(aptos_framework),
            reveal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRevealEvent">TimelockRevealEvent</a>&gt;(aptos_framework),
            expired_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockExpiredEvent">TimelockExpiredEvent</a>&gt;(aptos_framework),
        });
    }
}
</code></pre>



</details>

<a id="0x1_ibe_config_register_timelock"></a>

## Function `register_timelock`



<pre><code><b>public</b> entry <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_register_timelock">register_timelock</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, deadline_us: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_register_timelock">register_timelock</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    deadline_us: u64
) <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();
    <b>assert</b>!(current_time &lt; deadline_us, <a href="ibe_config.md#0x1_ibe_config_E_DEADLINE_NOT_PASSED">E_DEADLINE_NOT_PASSED</a>);

    <b>if</b> (!<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework)) {
        <a href="ibe_config.md#0x1_ibe_config_initialize_timelock_registry">initialize_timelock_registry</a>(<a href="account.md#0x1_account">account</a>);
    };

    <b>let</b> registry = <b>borrow_global_mut</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>let</b> timelock_id = registry.next_timelock_id;
    registry.next_timelock_id = timelock_id + 1;

    <b>let</b> identity = <a href="ibe_config.md#0x1_ibe_config_compute_identity">compute_identity</a>(timelock_id, deadline_us);

    <b>let</b> timelock_info = <a href="ibe_config.md#0x1_ibe_config_TimelockInfo">TimelockInfo</a> {
        timelock_id,
        deadline_us,
        identity,
        decryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        is_revealed: <b>false</b>,
        share_count: 0,
        reveal_threshold: 0,
        validator_indices: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        submitted_shares: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        validator_weights: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
        submitters: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
    };

    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> registry.timelocks, timelock_id, timelock_info);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> registry.pending_timelock_ids, timelock_id);

    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> registry.registration_events, <a href="ibe_config.md#0x1_ibe_config_TimelockRegistrationEvent">TimelockRegistrationEvent</a> {
        timelock_id,
        deadline_us,
        sender: <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer_address_of">signer::address_of</a>(<a href="account.md#0x1_account">account</a>),
        timestamp_us: current_time,
    });
}
</code></pre>



</details>

<a id="0x1_ibe_config_submit_dk_shares"></a>

## Function `submit_dk_shares`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_submit_dk_shares">submit_dk_shares</a>(timelock_id: u64, shares: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;, validator_address: <b>address</b>, weight: u64, total_weight: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_submit_dk_shares">submit_dk_shares</a>(
    timelock_id: u64,
    shares: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;,
    validator_address: <b>address</b>,
    weight: u64,
    total_weight: u64
) <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>let</b> num_shares = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&shares);
    <b>assert</b>!(num_shares == weight, <a href="ibe_config.md#0x1_ibe_config_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>); // Should be exactly 'weight' shares

    <b>let</b> registry = <b>borrow_global_mut</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>let</b> timelock_info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> registry.timelocks, timelock_id);

    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();
    <b>assert</b>!(current_time &gt;= timelock_info.deadline_us, <a href="ibe_config.md#0x1_ibe_config_E_DEADLINE_NOT_PASSED">E_DEADLINE_NOT_PASSED</a>);
    <b>assert</b>!(!timelock_info.is_revealed, <a href="ibe_config.md#0x1_ibe_config_E_DECRYPTION_KEY_NOT_REVEALED">E_DECRYPTION_KEY_NOT_REVEALED</a>);
    <b>assert</b>!(!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_contains">vector::contains</a>(&timelock_info.submitters, &validator_address), <a href="ibe_config.md#0x1_ibe_config_E_SHARE_ALREADY_SUBMITTED">E_SHARE_ALREADY_SUBMITTED</a>);

    <b>if</b> (timelock_info.reveal_threshold == 0) {
        timelock_info.reveal_threshold = (total_weight * <a href="ibe_config.md#0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_NUMERATOR">DEFAULT_REVEAL_THRESHOLD_NUMERATOR</a>) / <a href="ibe_config.md#0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_DENOMINATOR">DEFAULT_REVEAL_THRESHOLD_DENOMINATOR</a> + 1;
    };

    <b>let</b> validator_index = <a href="stake.md#0x1_stake_get_validator_index">stake::get_validator_index</a>(validator_address);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> timelock_info.validator_indices, validator_index);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> timelock_info.submitted_shares, shares);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> timelock_info.validator_weights, weight);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> timelock_info.submitters, validator_address);
    timelock_info.share_count = timelock_info.share_count + weight;

    <b>if</b> (timelock_info.share_count &gt;= timelock_info.reveal_threshold) {
        <a href="ibe_config.md#0x1_ibe_config_reconstruct_and_store_dk">reconstruct_and_store_dk</a>(registry, timelock_id, total_weight);
    };
}
</code></pre>



</details>

<a id="0x1_ibe_config_reconstruct_and_store_dk"></a>

## Function `reconstruct_and_store_dk`



<pre><code><b>fun</b> <a href="ibe_config.md#0x1_ibe_config_reconstruct_and_store_dk">reconstruct_and_store_dk</a>(registry: &<b>mut</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">ibe_config::TimelockRegistry</a>, timelock_id: u64, total_weight: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ibe_config.md#0x1_ibe_config_reconstruct_and_store_dk">reconstruct_and_store_dk</a>(
    registry: &<b>mut</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>,
    timelock_id: u64,
    total_weight: u64
) {
    <b>let</b> timelock_info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> registry.timelocks, timelock_id);

    <b>let</b> reconstructed_dk = <a href="../../aptos-stdlib/doc/ibe.md#0x1_ibe_reconstruct_ibe_dk">ibe::reconstruct_ibe_dk</a>&lt;G1&gt;(
        timelock_info.validator_indices,
        timelock_info.submitted_shares,
        timelock_info.validator_weights,
        timelock_info.reveal_threshold,
        total_weight,
        timelock_info.identity
    );

    timelock_info.decryption_key = reconstructed_dk;
    timelock_info.is_revealed = <b>true</b>;
    <a href="ibe_config.md#0x1_ibe_config_remove_pending_timelock_id">remove_pending_timelock_id</a>(registry, timelock_id);

    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> registry.reveal_events, <a href="ibe_config.md#0x1_ibe_config_TimelockRevealEvent">TimelockRevealEvent</a> {
        timelock_id,
        timestamp_us: current_time,
    });
}
</code></pre>



</details>

<a id="0x1_ibe_config_remove_pending_timelock_id"></a>

## Function `remove_pending_timelock_id`



<pre><code><b>fun</b> <a href="ibe_config.md#0x1_ibe_config_remove_pending_timelock_id">remove_pending_timelock_id</a>(registry: &<b>mut</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">ibe_config::TimelockRegistry</a>, timelock_id: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ibe_config.md#0x1_ibe_config_remove_pending_timelock_id">remove_pending_timelock_id</a>(registry: &<b>mut</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>, timelock_id: u64) {
    <b>let</b> (found, index) = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_index_of">vector::index_of</a>(&registry.pending_timelock_ids, &timelock_id);
    <b>if</b> (found) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_swap_remove">vector::swap_remove</a>(&<b>mut</b> registry.pending_timelock_ids, index);
    };
}
</code></pre>



</details>

<a id="0x1_ibe_config_on_new_block"></a>

## Function `on_new_block`



<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm);

    <b>if</b> (!<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework)) {
        <b>return</b>
    };

    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();
    <b>let</b> registry = <b>borrow_global_mut</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);

    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&registry.pending_timelock_ids);
    <b>while</b> (i &lt; len) {
        <b>let</b> timelock_id = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&registry.pending_timelock_ids, i);
        <b>let</b> timelock_info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id);

        <b>if</b> (current_time &gt;= timelock_info.deadline_us) {
            <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> registry.expired_events, <a href="ibe_config.md#0x1_ibe_config_TimelockExpiredEvent">TimelockExpiredEvent</a> {
                timelock_id,
                timestamp_us: current_time,
            });
        };

        i = i + 1;
    };
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_timelock"></a>

## Function `get_timelock`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_timelock">get_timelock</a>(timelock_id: u64): (u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_timelock">get_timelock</a>(timelock_id: u64): (u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bool, u64) <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework), <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>);
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&registry.timelocks, timelock_id), <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>);
    <b>let</b> info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id);
    (info.deadline_us, info.identity, info.is_revealed, info.share_count)
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_deadline"></a>

## Function `get_deadline`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_deadline">get_deadline</a>(timelock_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_deadline">get_deadline</a>(timelock_id: u64): u64 <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework), <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>);
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&registry.timelocks, timelock_id), <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>);
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id).deadline_us
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_identity"></a>

## Function `get_identity`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_identity">get_identity</a>(timelock_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_identity">get_identity</a>(timelock_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework), <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>);
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&registry.timelocks, timelock_id), <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>);
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id).identity
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_decryption_key"></a>

## Function `get_decryption_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_decryption_key">get_decryption_key</a>(timelock_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_decryption_key">get_decryption_key</a>(timelock_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework), <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>);
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&registry.timelocks, timelock_id), <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>);
    <b>let</b> info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id);
    <b>assert</b>!(info.is_revealed, <a href="ibe_config.md#0x1_ibe_config_E_DECRYPTION_KEY_NOT_REVEALED">E_DECRYPTION_KEY_NOT_REVEALED</a>);
    info.decryption_key
}
</code></pre>



</details>

<a id="0x1_ibe_config_is_revealed"></a>

## Function `is_revealed`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_is_revealed">is_revealed</a>(timelock_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_is_revealed">is_revealed</a>(timelock_id: u64): bool <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework), <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>);
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&registry.timelocks, timelock_id)) {
        <b>return</b> <b>false</b>
    };
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id).is_revealed
}
</code></pre>



</details>

<a id="0x1_ibe_config_is_expired"></a>

## Function `is_expired`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_is_expired">is_expired</a>(timelock_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_is_expired">is_expired</a>(timelock_id: u64): bool <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>assert</b>!(<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework), <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>);
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&registry.timelocks, timelock_id)) {
        <b>return</b> <b>false</b>
    };
    <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>() &gt;= <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id).deadline_us
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_next_timelock_id"></a>

## Function `get_next_timelock_id`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_next_timelock_id">get_next_timelock_id</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_next_timelock_id">get_next_timelock_id</a>(): u64 <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework).next_timelock_id
}
</code></pre>



</details>

<a id="0x1_ibe_config_compute_identity"></a>

## Function `compute_identity`

Compute the IBE identity for a timelock.
Identity = SHA3-256(timelock_id || deadline_us)


<pre><code><b>fun</b> <a href="ibe_config.md#0x1_ibe_config_compute_identity">compute_identity</a>(timelock_id: u64, deadline_us: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="ibe_config.md#0x1_ibe_config_compute_identity">compute_identity</a>(timelock_id: u64, deadline_us: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    <b>let</b> input = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&timelock_id);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> input, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&deadline_us));
    sha3_256(input)
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
