
<a id="0x1_ibe_config"></a>

# Module `0x1::ibe_config`

IBE (Identity-Based Encryption) configuration and Timelock Registry module.

This module implements the on-chain components for Atomica's timelock encryption system:

1. **IBE Public Parameters** - Stores the Master Public Key (MPK) from DKG
2. **Timelock Registry** - Manages registered timelocks with deadlines
3. **DK Share Aggregation** - Collects and aggregates validator decryption key shares
4. **Decryption Key Reconstruction** - Reconstructs DK using threshold shares


<a id="@Architecture_Overview_0"></a>

### Architecture Overview


```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                            ON-CHAIN STATE                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  @IBEPublicParams                                                            │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ mpk: vector<u8>     ← G2 point (96 bytes) from DKG                  │   │
│  │ epoch: u64          ← DKG epoch for rotation                         │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  @TimelockRegistry                                                           │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ timelocks: Table<u64, TimelockInfo>  ← All registered timelocks     │   │
│  │ next_timelock_id: u64                 ← Auto-incrementing ID        │   │
│  │ registration_events: EventHandle      ← Indexed for queries          │   │
│  │ reveal_events: EventHandle            ← Indexed for queries          │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  @TimelockInfo (per timelock)                                                │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │ identity: vector<u8> ← SHA3-256(timelock_id || deadline_us)         │   │
│  │ decryption_key: vector<u8> ← G1 point, empty before reveal          │   │
│  │ is_revealed: bool       ← True after threshold shares received      │   │
│  │ share_count: u64        ← Weighted count of shares received         │   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```


<a id="@Workflow_1"></a>

### Workflow


1. **Registration** - User calls <code><a href="ibe_config.md#0x1_ibe_config_register_timelock">register_timelock</a>(deadline_us)</code> → gets <code>timelock_id</code>
2. **Encryption** - Client queries MPK and identity, encrypts with IBE
3. **DKG** - Validators run DKG, produce shares, publish MPK
4. **Reveal** - After deadline, validators submit <code>TimelockShare</code> transactions
5. **Aggregation** - Contract aggregates shares, reconstructs DK when threshold met
6. **Decryption** - Anyone queries DK, decrypts ciphertext


<a id="@Documentation_References_2"></a>

### Documentation References


**Design Docs:**
- [ADR-001: Dual-Output DKG](atomica/docs/adr-001-dual-output-dkg.md)
- [Implementation Plan](atomica/docs/implementation-plan-unified-dkg-ibe.md)
- [Timelock Specification](atomica/docs/product-spec/atomica-timelock-spec.md)
- [Definitions](atomica/docs/definitions.md)

**Source Code:**
- [IBE Rust Module](crates/aptos-dkg/src/ibe/mod.rs)
- [Scalar ElGamal PVSS](crates/aptos-dkg/src/pvss/scalar_elgamal/transcript.rs)
- [DKG Integration](types/src/dkg/real_dkg/mod.rs)
- [Validator Transaction Handling](aptos-vm/src/validator_txns/timelock.rs)

**Tests:**
- [Register and Query Test](testsuite/smoke-test/src/timelock/register_and_query.rs)
- [Deadline Reveal Test](testsuite/smoke-test/src/timelock/deadline_reveal.rs)


<a id="@Error_Codes_3"></a>

### Error Codes


| Code | Constant | Description |
|------|----------|-------------|
| 1 | <code><a href="ibe_config.md#0x1_ibe_config_E_INVALID_MPK_LENGTH">E_INVALID_MPK_LENGTH</a></code> | MPK must be 96 bytes (G2 compressed) |
| 2 | <code><a href="ibe_config.md#0x1_ibe_config_E_IBE_NOT_READY">E_IBE_NOT_READY</a></code> | MPK not yet set by DKG |
| 3 | <code><a href="ibe_config.md#0x1_ibe_config_E_DEADLINE_NOT_PASSED">E_DEADLINE_NOT_PASSED</a></code> | Cannot reveal before deadline |
| 4 | <code><a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a></code> | Timelock ID not registered |
| 5 | <code><a href="ibe_config.md#0x1_ibe_config_E_DECRYPTION_KEY_NOT_REVEALED">E_DECRYPTION_KEY_NOT_REVEALED</a></code> | DK not yet aggregated |
| 6 | <code><a href="ibe_config.md#0x1_ibe_config_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a></code> | Threshold must be positive |
| 7 | <code><a href="ibe_config.md#0x1_ibe_config_E_SHARE_ALREADY_SUBMITTED">E_SHARE_ALREADY_SUBMITTED</a></code> | Validator already submitted share |


<a id="@Security_Considerations_4"></a>

### Security Considerations


- Only the framework address can update the MPK (via DKG)
- Only the framework address can submit DK shares (via ValidatorTransaction)
- Threshold reconstruction ensures liveness with honest majority
- Identity includes both timelock_id and deadline to prevent collisions


    -  [Architecture Overview](#@Architecture_Overview_0)
    -  [Workflow](#@Workflow_1)
    -  [Documentation References](#@Documentation_References_2)
    -  [Error Codes](#@Error_Codes_3)
    -  [Security Considerations](#@Security_Considerations_4)
-  [Resource `IBEPublicParams`](#0x1_ibe_config_IBEPublicParams)
-  [Struct `TimelockInfo`](#0x1_ibe_config_TimelockInfo)
-  [Resource `TimelockRegistry`](#0x1_ibe_config_TimelockRegistry)
-  [Struct `TimelockRegistrationEvent`](#0x1_ibe_config_TimelockRegistrationEvent)
-  [Struct `TimelockRevealEvent`](#0x1_ibe_config_TimelockRevealEvent)
-  [Constants](#@Constants_5)
-  [Function `initialize`](#0x1_ibe_config_initialize)
-  [Function `set_mpk`](#0x1_ibe_config_set_mpk)
-  [Function `get_mpk`](#0x1_ibe_config_get_mpk)
-  [Function `get_epoch`](#0x1_ibe_config_get_epoch)
-  [Function `is_ready`](#0x1_ibe_config_is_ready)
-  [Function `initialize_timelock_registry`](#0x1_ibe_config_initialize_timelock_registry)
-  [Function `register_timelock`](#0x1_ibe_config_register_timelock)
    -  [Arguments](#@Arguments_6)
    -  [Events](#@Events_7)
    -  [Note](#@Note_8)
-  [Function `submit_dk_share`](#0x1_ibe_config_submit_dk_share)
    -  [Arguments](#@Arguments_9)
    -  [Errors](#@Errors_10)
    -  [Side Effects](#@Side_Effects_11)
-  [Function `get_timelock`](#0x1_ibe_config_get_timelock)
    -  [Arguments](#@Arguments_12)
    -  [Returns](#@Returns_13)
-  [Function `get_deadline`](#0x1_ibe_config_get_deadline)
-  [Function `get_identity`](#0x1_ibe_config_get_identity)
-  [Function `get_decryption_key`](#0x1_ibe_config_get_decryption_key)
    -  [Returns](#@Returns_14)
-  [Function `is_revealed`](#0x1_ibe_config_is_revealed)
-  [Function `is_expired`](#0x1_ibe_config_is_expired)
-  [Function `get_next_timelock_id`](#0x1_ibe_config_get_next_timelock_id)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_hash">0x1::hash</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
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

<a id="0x1_ibe_config_TimelockInfo"></a>

## Struct `TimelockInfo`

Information about a registered timelock.


<pre><code><b>struct</b> <a href="ibe_config.md#0x1_ibe_config_TimelockInfo">TimelockInfo</a> <b>has</b> store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>timelock_id: u64</code>
</dt>
<dd>
 Unique identifier for this timelock
</dd>
<dt>
<code>deadline_us: u64</code>
</dt>
<dd>
 Deadline timestamp in microseconds
</dd>
<dt>
<code>identity: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 32-byte identity hash (computed from timelock_id || deadline_us)
 Used as the IBE identity for encryption/decryption
</dd>
<dt>
<code>decryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>
 The aggregated decryption key (G1, 48 bytes)
 Empty before reveal, populated after threshold shares received
</dd>
<dt>
<code>is_revealed: bool</code>
</dt>
<dd>
 Whether the decryption key has been revealed (deadline passed + threshold reached)
</dd>
<dt>
<code>share_count: u64</code>
</dt>
<dd>
 Number of validator shares received (weighted)
</dd>
<dt>
<code>reveal_threshold: u64</code>
</dt>
<dd>
 Threshold required to reveal (in weighted units)
</dd>
</dl>


</details>

<a id="0x1_ibe_config_TimelockRegistry"></a>

## Resource `TimelockRegistry`

Registry of all registered timelocks.


<pre><code><b>struct</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>timelocks: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="ibe_config.md#0x1_ibe_config_TimelockInfo">ibe_config::TimelockInfo</a>&gt;</code>
</dt>
<dd>
 Map from timelock_id to TimelockInfo
</dd>
<dt>
<code>next_timelock_id: u64</code>
</dt>
<dd>
 Counter for generating unique timelock IDs
</dd>
<dt>
<code>registration_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistrationEvent">ibe_config::TimelockRegistrationEvent</a>&gt;</code>
</dt>
<dd>
 Event handle for timelock registration events
</dd>
<dt>
<code>reveal_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRevealEvent">ibe_config::TimelockRevealEvent</a>&gt;</code>
</dt>
<dd>
 Event handle for decryption key reveal events
</dd>
</dl>


</details>

<a id="0x1_ibe_config_TimelockRegistrationEvent"></a>

## Struct `TimelockRegistrationEvent`

Event emitted when a new timelock is registered.


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

Event emitted when a decryption key is revealed.


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

<a id="@Constants_5"></a>

## Constants


<a id="0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_DENOMINATOR"></a>



<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_DENOMINATOR">DEFAULT_REVEAL_THRESHOLD_DENOMINATOR</a>: u64 = 3;
</code></pre>



<a id="0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_NUMERATOR"></a>

Default reveal threshold: 2/3 + 1 of total validator weight


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_NUMERATOR">DEFAULT_REVEAL_THRESHOLD_NUMERATOR</a>: u64 = 2;
</code></pre>



<a id="0x1_ibe_config_E_DEADLINE_NOT_PASSED"></a>

Timelock deadline has not yet passed


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_DEADLINE_NOT_PASSED">E_DEADLINE_NOT_PASSED</a>: u64 = 3;
</code></pre>



<a id="0x1_ibe_config_E_DECRYPTION_KEY_NOT_REVEALED"></a>

Decryption key not yet revealed


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_DECRYPTION_KEY_NOT_REVEALED">E_DECRYPTION_KEY_NOT_REVEALED</a>: u64 = 5;
</code></pre>



<a id="0x1_ibe_config_E_IBE_NOT_READY"></a>

IBE is not ready (MPK not yet set)


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_IBE_NOT_READY">E_IBE_NOT_READY</a>: u64 = 2;
</code></pre>



<a id="0x1_ibe_config_E_INVALID_MPK_LENGTH"></a>

MPK length must be exactly 96 bytes (G2 compressed)


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_INVALID_MPK_LENGTH">E_INVALID_MPK_LENGTH</a>: u64 = 1;
</code></pre>



<a id="0x1_ibe_config_E_INVALID_THRESHOLD"></a>

Threshold must be positive


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_INVALID_THRESHOLD">E_INVALID_THRESHOLD</a>: u64 = 6;
</code></pre>



<a id="0x1_ibe_config_E_SHARE_ALREADY_SUBMITTED"></a>

Decryption key share already submitted


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_SHARE_ALREADY_SUBMITTED">E_SHARE_ALREADY_SUBMITTED</a>: u64 = 7;
</code></pre>



<a id="0x1_ibe_config_E_TIMELOCK_NOT_FOUND"></a>

Timelock not found in registry


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_E_TIMELOCK_NOT_FOUND">E_TIMELOCK_NOT_FOUND</a>: u64 = 4;
</code></pre>



<a id="0x1_ibe_config_G1_LENGTH"></a>

Length of G1 point (used for decryption keys)


<pre><code><b>const</b> <a href="ibe_config.md#0x1_ibe_config_G1_LENGTH">G1_LENGTH</a>: u64 = 48;
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
    };
    <a href="ibe_config.md#0x1_ibe_config_initialize_timelock_registry">initialize_timelock_registry</a>(aptos_framework);
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

<a id="0x1_ibe_config_initialize_timelock_registry"></a>

## Function `initialize_timelock_registry`

Initialize the timelock registry. Called once at genesis.


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_initialize_timelock_registry">initialize_timelock_registry</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_initialize_timelock_registry">initialize_timelock_registry</a>(aptos_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(aptos_framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(aptos_framework, <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
            timelocks: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
            next_timelock_id: 0,
            registration_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistrationEvent">TimelockRegistrationEvent</a>&gt;(aptos_framework),
            reveal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRevealEvent">TimelockRevealEvent</a>&gt;(aptos_framework),
        });
    }
}
</code></pre>



</details>

<a id="0x1_ibe_config_register_timelock"></a>

## Function `register_timelock`

Register a new timelock with the given deadline.


<a id="@Arguments_6"></a>

### Arguments

- <code><a href="account.md#0x1_account">account</a></code>: The registering account (pays gas)
- <code>deadline_us</code>: Deadline timestamp in microseconds (must be in the future)


<a id="@Events_7"></a>

### Events

Emits <code><a href="ibe_config.md#0x1_ibe_config_TimelockRegistrationEvent">TimelockRegistrationEvent</a></code> with the timelock_id.


<a id="@Note_8"></a>

### Note

The timelock_id can be retrieved from the event or by calling <code><a href="ibe_config.md#0x1_ibe_config_get_next_timelock_id">get_next_timelock_id</a>()</code>
after the transaction (which returns the ID that will be assigned to the next registration).


<pre><code><b>public</b> entry <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_register_timelock">register_timelock</a>(<a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, deadline_us: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_register_timelock">register_timelock</a>(
    <a href="account.md#0x1_account">account</a>: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    deadline_us: u64
) <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();
    <b>assert</b>!(deadline_us &gt; current_time, <a href="ibe_config.md#0x1_ibe_config_E_DEADLINE_NOT_PASSED">E_DEADLINE_NOT_PASSED</a>);

    <b>let</b> registry = <b>borrow_global_mut</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);

    // Generate unique timelock ID
    <b>let</b> timelock_id = registry.next_timelock_id;
    registry.next_timelock_id = timelock_id + 1;

    // Compute identity: sha3_256(timelock_id || deadline_us)
    <b>let</b> identity_input = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> identity_input, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&timelock_id));
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> identity_input, <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&deadline_us));
    <b>let</b> identity = sha3_256(identity_input);

    // Create timelock info (decryption_key empty initially)
    <b>let</b> timelock_info = <a href="ibe_config.md#0x1_ibe_config_TimelockInfo">TimelockInfo</a> {
        timelock_id,
        deadline_us,
        identity,
        decryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;(),
        is_revealed: <b>false</b>,
        share_count: 0,
        reveal_threshold: 0, // Will be set during reveal phase
    };

    // Add <b>to</b> registry
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> registry.timelocks, timelock_id, timelock_info);

    // Emit registration <a href="event.md#0x1_event">event</a>
    <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> registry.registration_events, <a href="ibe_config.md#0x1_ibe_config_TimelockRegistrationEvent">TimelockRegistrationEvent</a> {
        timelock_id,
        deadline_us,
        sender: std::signer::address_of(<a href="account.md#0x1_account">account</a>),
        timestamp_us: current_time,
    });
}
</code></pre>



</details>

<a id="0x1_ibe_config_submit_dk_share"></a>

## Function `submit_dk_share`

Submit a decryption key share for a timelock.

Called by validator transaction handler after deadline passes.
NOT callable by users directly (friend function).


<a id="@Arguments_9"></a>

### Arguments

- <code>timelock_id</code>: The timelock being revealed
- <code>share</code>: The DK share (G1, 48 bytes) = sk_share * H(identity)
- <code>validator_address</code>: Address of the submitting validator
- <code>weight</code>: Validator's weight (stake)
- <code>total_weight</code>: Total validator weight (for threshold calculation)


<a id="@Errors_10"></a>

### Errors

- Aborts if deadline not passed
- Aborts if validator already submitted
- Aborts if timelock not found


<a id="@Side_Effects_11"></a>

### Side Effects

If this share reaches threshold, aggregates and stores final DK,
then marks timelock as revealed.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_submit_dk_share">submit_dk_share</a>(timelock_id: u64, share: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, validator_address: <b>address</b>, weight: u64, total_weight: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_submit_dk_share">submit_dk_share</a>(
    timelock_id: u64,
    share: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;,
    validator_address: <b>address</b>,
    weight: u64,
    total_weight: u64
) <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>assert</b>!(<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&share) == <a href="ibe_config.md#0x1_ibe_config_G1_LENGTH">G1_LENGTH</a>, <a href="ibe_config.md#0x1_ibe_config_E_INVALID_MPK_LENGTH">E_INVALID_MPK_LENGTH</a>);

    <b>let</b> registry = <b>borrow_global_mut</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>let</b> timelock_info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> registry.timelocks, timelock_id);

    // Verify deadline <b>has</b> passed
    <b>let</b> current_time = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();
    <b>assert</b>!(current_time &gt;= timelock_info.deadline_us, <a href="ibe_config.md#0x1_ibe_config_E_DEADLINE_NOT_PASSED">E_DEADLINE_NOT_PASSED</a>);

    // Initialize threshold on first share <b>if</b> not set
    <b>if</b> (timelock_info.reveal_threshold == 0) {
        timelock_info.reveal_threshold = (total_weight * <a href="ibe_config.md#0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_NUMERATOR">DEFAULT_REVEAL_THRESHOLD_NUMERATOR</a>) / <a href="ibe_config.md#0x1_ibe_config_DEFAULT_REVEAL_THRESHOLD_DENOMINATOR">DEFAULT_REVEAL_THRESHOLD_DENOMINATOR</a> + 1;
    };

    // Check <b>if</b> already revealed
    <b>assert</b>!(!timelock_info.is_revealed, <a href="ibe_config.md#0x1_ibe_config_E_DECRYPTION_KEY_NOT_REVEALED">E_DECRYPTION_KEY_NOT_REVEALED</a>);

    // Add share <b>to</b> decryption key (accumulate in exponent)
    // For G1 points, we add them: DK = sum(share_i)
    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&timelock_info.decryption_key)) {
        timelock_info.decryption_key = share;
    } <b>else</b> {
        // Simple accumulation - in production, would need proper point addition
        // This is a placeholder for the aggregation logic
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_append">vector::append</a>(&<b>mut</b> timelock_info.decryption_key, share);
    };

    timelock_info.share_count = timelock_info.share_count + weight;

    // Check <b>if</b> threshold reached
    <b>if</b> (timelock_info.share_count &gt;= timelock_info.reveal_threshold) {
        timelock_info.is_revealed = <b>true</b>;

        // Emit reveal <a href="event.md#0x1_event">event</a>
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> registry.reveal_events, <a href="ibe_config.md#0x1_ibe_config_TimelockRevealEvent">TimelockRevealEvent</a> {
            timelock_id,
            timestamp_us: current_time,
        });
    };

    // Note: In production, would need <b>to</b> track which validators have submitted
    // <b>to</b> prevent duplicate submissions and enable proper aggregation
    <b>let</b> _ = validator_address; // Suppress unused warning
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_timelock"></a>

## Function `get_timelock`

Get information about a registered timelock.


<a id="@Arguments_12"></a>

### Arguments

- <code>timelock_id</code>: The ID returned from register_timelock


<a id="@Returns_13"></a>

### Returns

Tuple of (deadline_us, identity, is_revealed, share_count)


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_timelock">get_timelock</a>(timelock_id: u64): (u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bool, u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_timelock">get_timelock</a>(timelock_id: u64): (u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;, bool, u64) <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>let</b> timelock_info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id);
    (
        timelock_info.deadline_us,
        timelock_info.identity,
        timelock_info.is_revealed,
        timelock_info.share_count
    )
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_deadline"></a>

## Function `get_deadline`

Get the deadline for a timelock.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_deadline">get_deadline</a>(timelock_id: u64): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_deadline">get_deadline</a>(timelock_id: u64): u64 <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>let</b> timelock_info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id);
    timelock_info.deadline_us
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_identity"></a>

## Function `get_identity`

Get the identity hash for a timelock.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_identity">get_identity</a>(timelock_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_identity">get_identity</a>(timelock_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>let</b> timelock_info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id);
    timelock_info.identity
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_decryption_key"></a>

## Function `get_decryption_key`

Get the decryption key for a timelock after reveal.


<a id="@Returns_14"></a>

### Returns

The decryption key (G1, 48 bytes) or empty vector if not yet revealed.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_decryption_key">get_decryption_key</a>(timelock_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_get_decryption_key">get_decryption_key</a>(timelock_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>let</b> timelock_info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id);
    <b>assert</b>!(timelock_info.is_revealed, <a href="ibe_config.md#0x1_ibe_config_E_DECRYPTION_KEY_NOT_REVEALED">E_DECRYPTION_KEY_NOT_REVEALED</a>);
    timelock_info.decryption_key
}
</code></pre>



</details>

<a id="0x1_ibe_config_is_revealed"></a>

## Function `is_revealed`

Check if a timelock's decryption key has been revealed.

Returns true if the deadline has passed AND threshold shares received.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_is_revealed">is_revealed</a>(timelock_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_is_revealed">is_revealed</a>(timelock_id: u64): bool <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>let</b> timelock_info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id);
    timelock_info.is_revealed
}
</code></pre>



</details>

<a id="0x1_ibe_config_is_expired"></a>

## Function `is_expired`

Check if a timelock's deadline has passed.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_is_expired">is_expired</a>(timelock_id: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="ibe_config.md#0x1_ibe_config_is_expired">is_expired</a>(timelock_id: u64): bool <b>acquires</b> <a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a> {
    <b>let</b> registry = <b>borrow_global</b>&lt;<a href="ibe_config.md#0x1_ibe_config_TimelockRegistry">TimelockRegistry</a>&gt;(@aptos_framework);
    <b>let</b> timelock_info = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&registry.timelocks, timelock_id);
    <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>() &gt;= timelock_info.deadline_us
}
</code></pre>



</details>

<a id="0x1_ibe_config_get_next_timelock_id"></a>

## Function `get_next_timelock_id`

Get the current timelock counter (next available ID).


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


[move-book]: https://aptos.dev/move/book/SUMMARY
