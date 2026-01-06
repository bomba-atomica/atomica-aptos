
<a id="0x1_timelock"></a>

# Module `0x1::timelock`



-  [Struct `TimelockConfig`](#0x1_timelock_TimelockConfig)
-  [Struct `IntervalConfig`](#0x1_timelock_IntervalConfig)
-  [Struct `DecryptionKeyShare`](#0x1_timelock_DecryptionKeyShare)
-  [Resource `TimelockState`](#0x1_timelock_TimelockState)
-  [Struct `StartKeyGenEvent`](#0x1_timelock_StartKeyGenEvent)
-  [Struct `RequestRevealEvent`](#0x1_timelock_RequestRevealEvent)
-  [Struct `DecryptionKeyRevealedEvent`](#0x1_timelock_DecryptionKeyRevealedEvent)
-  [Constants](#@Constants_0)
    -  [Atomica Timelock Service (IBE-based)](#@Atomica_Timelock_Service_(IBE-based)_1)
        -  [References](#@References_2)
        -  [Protocol Overview](#@Protocol_Overview_3)
        -  [Architecture](#@Architecture_4)
-  [Function `initialize`](#0x1_timelock_initialize)
-  [Function `perform_rotation`](#0x1_timelock_perform_rotation)
-  [Function `on_new_block`](#0x1_timelock_on_new_block)
-  [Function `trigger_rotation`](#0x1_timelock_trigger_rotation)
-  [Function `force_rotation_for_testing`](#0x1_timelock_force_rotation_for_testing)
-  [Function `publish_master_public_key`](#0x1_timelock_publish_master_public_key)
    -  [[BF01] Setup Phase](#@[BF01]_Setup_Phase_5)
-  [Function `publish_decryption_key_share`](#0x1_timelock_publish_decryption_key_share)
    -  [[BF01] Extract Phase (Distributed)](#@[BF01]_Extract_Phase_(Distributed)_6)
-  [Function `get_current_interval`](#0x1_timelock_get_current_interval)
-  [Function `get_interval_config`](#0x1_timelock_get_interval_config)
-  [Function `get_master_public_key`](#0x1_timelock_get_master_public_key)
-  [Function `is_decryption_key_revealed`](#0x1_timelock_is_decryption_key_revealed)
-  [Function `get_decryption_key`](#0x1_timelock_get_decryption_key)
-  [Specification](#@Specification_7)
    -  [Function `initialize`](#@Specification_7_initialize)
    -  [Function `on_new_block`](#@Specification_7_on_new_block)
    -  [Function `publish_master_public_key`](#@Specification_7_publish_master_public_key)
    -  [Function `publish_decryption_key_share`](#@Specification_7_publish_decryption_key_share)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs">0x1::bcs</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">0x1::bls12381_algebra</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/debug.md#0x1_debug">0x1::debug</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="ibe_signature.md#0x1_ibe_signature">0x1::ibe_signature</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="threshold_dsa.md#0x1_threshold_dsa">0x1::threshold_dsa</a>;
<b>use</b> <a href="timelock_config.md#0x1_timelock_config">0x1::timelock_config</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info">0x1::validator_consensus_info</a>;
</code></pre>



<a id="0x1_timelock_TimelockConfig"></a>

## Struct `TimelockConfig`



<pre><code><b>struct</b> <a href="timelock.md#0x1_timelock_TimelockConfig">TimelockConfig</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>threshold: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_validators: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_timelock_IntervalConfig"></a>

## Struct `IntervalConfig`



<pre><code><b>struct</b> <a href="timelock.md#0x1_timelock_IntervalConfig">IntervalConfig</a> <b>has</b> <b>copy</b>, drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>threshold: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>total_validators: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>created_at: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_timelock_DecryptionKeyShare"></a>

## Struct `DecryptionKeyShare`



<pre><code><b>struct</b> <a href="timelock.md#0x1_timelock_DecryptionKeyShare">DecryptionKeyShare</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>validator: <b>address</b></code>
</dt>
<dd>

</dd>
<dt>
<code>share: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_timelock_TimelockState"></a>

## Resource `TimelockState`



<pre><code><b>struct</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>current_interval: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>last_rotation_time: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>decryption_key_shares: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="timelock.md#0x1_timelock_DecryptionKeyShare">timelock::DecryptionKeyShare</a>&gt;&gt;</code>
</dt>
<dd>
 Store collected key shares before aggregation
</dd>
<dt>
<code>decryption_keys: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Store revealed decryption keys (DK)
</dd>
<dt>
<code>interval_configs: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="timelock.md#0x1_timelock_IntervalConfig">timelock::IntervalConfig</a>&gt;</code>
</dt>
<dd>
 Store historical interval configurations
</dd>
</dl>


</details>

<a id="0x1_timelock_StartKeyGenEvent"></a>

## Struct `StartKeyGenEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="timelock.md#0x1_timelock_StartKeyGenEvent">StartKeyGenEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>interval: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>config: <a href="timelock.md#0x1_timelock_TimelockConfig">timelock::TimelockConfig</a></code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_timelock_RequestRevealEvent"></a>

## Struct `RequestRevealEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="timelock.md#0x1_timelock_RequestRevealEvent">RequestRevealEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>interval: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_timelock_DecryptionKeyRevealedEvent"></a>

## Struct `DecryptionKeyRevealedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="timelock.md#0x1_timelock_DecryptionKeyRevealedEvent">DecryptionKeyRevealedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>interval: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>decryption_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_timelock_ENOT_VALIDATOR"></a>

Not a validator.


<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_ENOT_VALIDATOR">ENOT_VALIDATOR</a>: u64 = 2;
</code></pre>



<a id="0x1_timelock_EINVALID_INTERVAL"></a>

Invalid interval for reveal operation.


<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_EINVALID_INTERVAL">EINVALID_INTERVAL</a>: u64 = 5;
</code></pre>



<a id="0x1_timelock_EINVALID_SHARE"></a>

Invalid share format.


<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_EINVALID_SHARE">EINVALID_SHARE</a>: u64 = 3;
</code></pre>



<a id="0x1_timelock_EROTATION_TOO_EARLY"></a>

Rotation triggered too early.


<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_EROTATION_TOO_EARLY">EROTATION_TOO_EARLY</a>: u64 = 4;
</code></pre>



<a id="0x1_timelock_ESHARE_VERIFICATION_FAILED"></a>

Share verification failed against MPK/Identity.


<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_ESHARE_VERIFICATION_FAILED">ESHARE_VERIFICATION_FAILED</a>: u64 = 6;
</code></pre>



<a id="0x1_timelock_ETIMELOCK_NOT_INITIALIZED"></a>


<a id="@Atomica_Timelock_Service_(IBE-based)_1"></a>

### Atomica Timelock Service (IBE-based)


This module implements the on-chain registry and orchestration for a Timelock Encryption service
based on **Identity-Based Encryption (IBE)** as defined by Boneh and Franklin [BF01].


<a id="@References_2"></a>

#### References


*   **[BF01]**: Boneh, D., & Franklin, M. (2001). "Identity-based encryption from the Weil pairing."


<a id="@Protocol_Overview_3"></a>

#### Protocol Overview


The system treats time intervals as "Identities" in an IBE scheme.

1.  **Setup ($P_{pub}$)**: Validators engage in a Distributed Key Generation (DKG) to produce a shared Master Secret Key ($s$)
and publish the Master Public Key ($P_{pub} = s \cdot g_2$) on-chain.
*   See <code>publish_master_public_key</code>.

2.  **Encryption (Off-Chain)**: Users encrypt messages for a future time interval $T$ using $P_{pub}$ and identity $ID = T$.
*   $C = \text{Encrypt}(P_{pub}, ID, M)$.

3.  **Reveal / Extract ($d_{ID}$)**: When time $T$ arrives, validators compute partial private keys (signature shares) for $ID = T$.
*   Share: $\sigma_i = s_i \cdot H_1(ID)$.

4.  **Aggregation**: The contract verifies and aggregates these shares to reconstruct the full private key $d_{ID} = s \cdot H_1(ID)$.
*   This $d_{ID}$ allows anyone to decrypt $C$.
*   See <code>publish_decryption_key_share</code>.


<a id="@Architecture_4"></a>

#### Architecture


*   **<code><a href="timelock.md#0x1_timelock">timelock</a>.<b>move</b></code>**: This module. Orchestrates the lifecycle (Intervals, Rotation, Reveal).
*   **<code><a href="threshold_dsa.md#0x1_threshold_dsa">threshold_dsa</a>.<b>move</b></code>**: Manages the underlying MPK storage and curve verification.
*   **<code><a href="ibe_signature.md#0x1_ibe_signature">ibe_signature</a>.<b>move</b></code>**: Defines the $H_1$ mapping from Identity to Point.
The singleton was not initialized.


<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_ETIMELOCK_NOT_INITIALIZED">ETIMELOCK_NOT_INITIALIZED</a>: u64 = 1;
</code></pre>



<a id="0x1_timelock_initialize"></a>

## Function `initialize`

Initialize the timelock system.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    // Initialize dependency modules
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        // Ensure <a href="threshold_dsa.md#0x1_threshold_dsa">threshold_dsa</a> is initialized
        <a href="threshold_dsa.md#0x1_threshold_dsa_initialize">threshold_dsa::initialize</a>(framework);

        <b>move_to</b>(framework, <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
            current_interval: 0,
            last_rotation_time: 0, // Will be updated on first <a href="block.md#0x1_block">block</a>
            decryption_key_shares: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
            decryption_keys: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
            interval_configs: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
        });
    }
}
</code></pre>



</details>

<a id="0x1_timelock_perform_rotation"></a>

## Function `perform_rotation`

Internal function to perform rotation logic


<pre><code><b>fun</b> <a href="timelock.md#0x1_timelock_perform_rotation">perform_rotation</a>(state: &<b>mut</b> <a href="timelock.md#0x1_timelock_TimelockState">timelock::TimelockState</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="timelock.md#0x1_timelock_perform_rotation">perform_rotation</a>(state: &<b>mut</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>) {
    <b>let</b> now = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();
    <b>let</b> old_interval = state.current_interval;

    // Emit reveal <a href="event.md#0x1_event">event</a> for the <b>old</b> interval
    emit(<a href="timelock.md#0x1_timelock_RequestRevealEvent">RequestRevealEvent</a> {
        interval: old_interval,
    });

    state.current_interval = state.current_interval + 1;
    state.last_rotation_time = now;

    // DEBUG: Log interval rotation
    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] Interval rotated <b>to</b>"));
    aptos_std::debug::print(&state.current_interval);

    // Get current validator set <b>to</b> determine threshold
    <b>let</b> validators = <a href="stake.md#0x1_stake_cur_validator_consensus_infos">stake::cur_validator_consensus_infos</a>();
    <b>let</b> validator_addresses = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;<b>address</b>&gt;();
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&validators);
    <b>while</b> (i &lt; len) {
        <b>let</b> v = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&validators, i);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> validator_addresses, <a href="validator_consensus_info.md#0x1_validator_consensus_info_get_addr">validator_consensus_info::get_addr</a>(v));
        i = i + 1;
    };
    <b>let</b> total_validators = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&validators);
    // Byztantine Fault Tolerance threshold: 2f + 1, <b>where</b> N = 3f + 1
    // Simple formula: floor(N * 2 / 3) + 1
    <b>let</b> threshold = (total_validators * 2 / 3) + 1;
    <b>if</b> (total_validators == 0) { threshold = 1; }; // Fallback for testing/<a href="genesis.md#0x1_genesis">genesis</a>

    <b>let</b> config = <a href="timelock.md#0x1_timelock_TimelockConfig">TimelockConfig</a> {
        threshold,
        total_validators,
    };

    // Store interval config for future reveal validation
    <b>let</b> interval_config = <a href="timelock.md#0x1_timelock_IntervalConfig">IntervalConfig</a> {
        threshold,
        total_validators,
        created_at: now,
    };
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.interval_configs, state.current_interval, interval_config);

    emit(<a href="timelock.md#0x1_timelock_StartKeyGenEvent">StartKeyGenEvent</a> {
        interval: state.current_interval,
        config,
    });
    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] Emitted <a href="timelock.md#0x1_timelock_StartKeyGenEvent">StartKeyGenEvent</a>"));
}
</code></pre>



</details>

<a id="0x1_timelock_on_new_block"></a>

## Function `on_new_block`

Called by block prologue to trigger rotations.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm);

    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block called"));

    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> does not exist - returning"));
        <b>return</b>
    };

    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>let</b> now = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();

    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block: current_interval="));
    aptos_std::debug::print(&state.current_interval);
    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block: now="));
    aptos_std::debug::print(&now);
    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block: last_rotation_time="));
    aptos_std::debug::print(&state.last_rotation_time);

    // Initialize last_rotation_time <b>if</b> it's 0 (<a href="genesis.md#0x1_genesis">genesis</a>/first run)
    <b>if</b> (state.last_rotation_time == 0) {
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] Initializing last_rotation_time <b>to</b> current time"));
        state.last_rotation_time = now;
        <b>return</b>
    };

    // Check <b>if</b> configured interval <b>has</b> passed (get from <a href="timelock_config.md#0x1_timelock_config">timelock_config</a>)
    <b>let</b> interval_micros = <a href="timelock_config.md#0x1_timelock_config_get_interval_microseconds">timelock_config::get_interval_microseconds</a>();
    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] interval_micros="));
    aptos_std::debug::print(&interval_micros);

    <b>let</b> elapsed = now - state.last_rotation_time;
    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] elapsed="));
    aptos_std::debug::print(&elapsed);
    aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] should_rotate="));
    aptos_std::debug::print(&(elapsed &gt; interval_micros));

    <b>if</b> (now - state.last_rotation_time &gt; interval_micros) {
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] Calling perform_rotation"));
        <a href="timelock.md#0x1_timelock_perform_rotation">perform_rotation</a>(state);
    }
}
</code></pre>



</details>

<a id="0x1_timelock_trigger_rotation"></a>

## Function `trigger_rotation`

Manual rotation trigger that can be called by anyone after the scheduled time.
This allows testing and emergency rotation when automatic rotation fails.


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_trigger_rotation">trigger_rotation</a>(_account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_trigger_rotation">trigger_rotation</a>(_account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        <b>return</b>
    };

    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>let</b> now = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();

    // Initialize last_rotation_time <b>if</b> it's 0 (<a href="genesis.md#0x1_genesis">genesis</a>/first run)
    <b>if</b> (state.last_rotation_time == 0) {
        state.last_rotation_time = now;
        <b>return</b>
    };

    // Check <b>if</b> configured interval <b>has</b> passed (get from <a href="timelock_config.md#0x1_timelock_config">timelock_config</a>)
    <b>let</b> interval_micros = <a href="timelock_config.md#0x1_timelock_config_get_interval_microseconds">timelock_config::get_interval_microseconds</a>();
    <b>assert</b>!(now - state.last_rotation_time &gt; interval_micros, <a href="timelock.md#0x1_timelock_EROTATION_TOO_EARLY">EROTATION_TOO_EARLY</a>);

    <a href="timelock.md#0x1_timelock_perform_rotation">perform_rotation</a>(state);
}
</code></pre>



</details>

<a id="0x1_timelock_force_rotation_for_testing"></a>

## Function `force_rotation_for_testing`

Force rotation for testing purposes.
Bypasses the time check. Only available on non-mainnet chains.


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_force_rotation_for_testing">force_rotation_for_testing</a>(_account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_force_rotation_for_testing">force_rotation_for_testing</a>(_account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>assert</b>!(<a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() != 1, <a href="timelock.md#0x1_timelock_EROTATION_TOO_EARLY">EROTATION_TOO_EARLY</a>);

    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        <b>return</b>
    };

    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <a href="timelock.md#0x1_timelock_perform_rotation">perform_rotation</a>(state);
}
</code></pre>



</details>

<a id="0x1_timelock_publish_master_public_key"></a>

## Function `publish_master_public_key`

Validators call this to publish the Master Public Key ($P_{pub}$) for a future interval.


<a id="@[BF01]_Setup_Phase_5"></a>

### [BF01] Setup Phase


This corresponds to the **Setup** algorithm. Ideally, this runs once for the system lifetime or per epoch.
The $P_{pub}$ is stored in <code><a href="threshold_dsa.md#0x1_threshold_dsa">threshold_dsa</a></code> and allows users to derive Public Keys for any identity $ID$.


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_master_public_key">publish_master_public_key</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, interval: u64, pk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_master_public_key">publish_master_public_key</a>(
    validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    interval: u64,
    pk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) {
    // Delegate <b>to</b> <a href="threshold_dsa.md#0x1_threshold_dsa">threshold_dsa</a> <b>module</b>
    <a href="threshold_dsa.md#0x1_threshold_dsa_publish_master_public_key">threshold_dsa::publish_master_public_key</a>(validator, interval, pk);
}
</code></pre>



</details>

<a id="0x1_timelock_publish_decryption_key_share"></a>

## Function `publish_decryption_key_share`

Validators call this to publish their partial Decryption Key ($d_{ID}$) share for a past interval.


<a id="@[BF01]_Extract_Phase_(Distributed)_6"></a>

### [BF01] Extract Phase (Distributed)


When the time interval $ID$ passes, the "Private Key Generator" (PKG)—in this case, the validator set—
cooperatively constructs the private key $d_{ID}$ corresponding to the identity $ID$.

*   **Input**: Validator share $\sigma_i$.
*   **Logic**:
1.  Verify $\sigma_i$ against $P_{pub}$ and $ID$ (using <code><a href="ibe_signature.md#0x1_ibe_signature_verify_private_key">ibe_signature::verify_private_key</a></code>).
2.  Accumulate shares until threshold is met.
3.  Aggregate to form $d_{ID} = \sum \sigma_i$.
4.  Publish $d_{ID}$.

Once $d_{ID}$ is published, any ciphertext encrypted for $ID$ can be decrypted.


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_decryption_key_share">publish_decryption_key_share</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, interval: u64, share: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_decryption_key_share">publish_decryption_key_share</a>(
    validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    interval: u64,
    share: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>let</b> validator_addr = std::signer::address_of(validator);

    // 1. Verify validator authorization
    <b>assert</b>!(<a href="stake.md#0x1_stake_is_current_epoch_validator">stake::is_current_epoch_validator</a>(validator_addr), <a href="timelock.md#0x1_timelock_ENOT_VALIDATOR">ENOT_VALIDATOR</a>);

    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);

    // Security Check: Only allow revealing PAST intervals
    <b>assert</b>!(interval &lt; state.current_interval, <a href="timelock.md#0x1_timelock_EINVALID_INTERVAL">EINVALID_INTERVAL</a>);

    // If outcome already revealed, ignore
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.decryption_keys, interval)) {
        <b>return</b>
    };

    // 2. CRYPTOGRAPHIC VERIFICATION
    // Construct Identity from interval (u64 -&gt; bytes)
    <b>let</b> identity = <a href="../../aptos-stdlib/../move-stdlib/doc/bcs.md#0x1_bcs_to_bytes">bcs::to_bytes</a>(&interval);

    // Verify the share against the MPK for this interval
    // Note: verify_private_key handles MPK lookup in <a href="threshold_dsa.md#0x1_threshold_dsa">threshold_dsa</a>
    <b>let</b> is_valid = <a href="ibe_signature.md#0x1_ibe_signature_verify_private_key">ibe_signature::verify_private_key</a>(interval, identity, share);
    <b>assert</b>!(is_valid, <a href="timelock.md#0x1_timelock_ESHARE_VERIFICATION_FAILED">ESHARE_VERIFICATION_FAILED</a>);

    // 3. Store valid share
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.decryption_key_shares, interval)) {
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.decryption_key_shares, interval, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>());
    };
    <b>let</b> shares_list = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> state.decryption_key_shares, interval);

    // Dedup
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shares_list);
    <b>while</b> (i &lt; len) {
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(shares_list, i).validator == validator_addr) {
            <b>return</b>
        };
        i = i + 1;
    };

    // Share is already verified cryptographically, but we need <b>to</b> deserialize for aggregation.
    // deserialize should succeed <b>if</b> verify succeeded, but we check.
    <b>let</b> share_opt = deserialize&lt;G1, FormatG1Compr&gt;(&share);
    <b>assert</b>!(std::option::is_some(&share_opt), <a href="timelock.md#0x1_timelock_EINVALID_SHARE">EINVALID_SHARE</a>);

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(shares_list, <a href="timelock.md#0x1_timelock_DecryptionKeyShare">DecryptionKeyShare</a> {
        validator: validator_addr,
        share: share,
    });

    // 4. Check threshold
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.interval_configs, interval), <a href="timelock.md#0x1_timelock_EINVALID_INTERVAL">EINVALID_INTERVAL</a>);
    <b>let</b> config = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.interval_configs, interval);
    <b>let</b> threshold = config.threshold;
    <b>let</b> valid_count = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shares_list);

    <b>if</b> (valid_count &gt;= threshold) {
        // 5. Aggregate
        <b>let</b> sum = zero&lt;G1&gt;();
        <b>let</b> i = 0;
        <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shares_list);
        <b>let</b> aggregated_count = 0;

        <b>while</b> (i &lt; len && aggregated_count &lt; threshold) {
            <b>let</b> s_bytes = &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(shares_list, i).share;
            <b>let</b> element_opt = deserialize&lt;G1, FormatG1Compr&gt;(s_bytes);
            <b>if</b> (std::option::is_some(&element_opt)) {
                <b>let</b> element = std::option::extract(&<b>mut</b> element_opt);
                sum = add(&sum, &element);
                aggregated_count = aggregated_count + 1;
            };
            i = i + 1;
        };

        <b>let</b> aggregated_bytes = serialize&lt;G1, FormatG1Compr&gt;(&sum);
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.decryption_keys, interval, aggregated_bytes);

        // Emit <a href="event.md#0x1_event">event</a>
        emit(<a href="timelock.md#0x1_timelock_DecryptionKeyRevealedEvent">DecryptionKeyRevealedEvent</a> {
            interval,
            decryption_key: aggregated_bytes,
        });
    }
}
</code></pre>



</details>

<a id="0x1_timelock_get_current_interval"></a>

## Function `get_current_interval`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_current_interval">get_current_interval</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_current_interval">get_current_interval</a>(): u64 <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        <b>return</b> 0
    };
    <b>borrow_global</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework).current_interval
}
</code></pre>



</details>

<a id="0x1_timelock_get_interval_config"></a>

## Function `get_interval_config`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_interval_config">get_interval_config</a>(interval: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="timelock.md#0x1_timelock_IntervalConfig">timelock::IntervalConfig</a>&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_interval_config">get_interval_config</a>(interval: u64): Option&lt;<a href="timelock.md#0x1_timelock_IntervalConfig">IntervalConfig</a>&gt; <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.interval_configs, interval)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.interval_configs, interval))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_timelock_get_master_public_key"></a>

## Function `get_master_public_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_master_public_key">get_master_public_key</a>(interval: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_master_public_key">get_master_public_key</a>(interval: u64): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; {
    // Delegate <b>to</b> <a href="threshold_dsa.md#0x1_threshold_dsa">threshold_dsa</a>
    <a href="threshold_dsa.md#0x1_threshold_dsa_get_master_public_key">threshold_dsa::get_master_public_key</a>(interval)
}
</code></pre>



</details>

<a id="0x1_timelock_is_decryption_key_revealed"></a>

## Function `is_decryption_key_revealed`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_is_decryption_key_revealed">is_decryption_key_revealed</a>(interval: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_is_decryption_key_revealed">is_decryption_key_revealed</a>(interval: u64): bool <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.decryption_keys, interval)
}
</code></pre>



</details>

<a id="0x1_timelock_get_decryption_key"></a>

## Function `get_decryption_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_decryption_key">get_decryption_key</a>(interval: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_decryption_key">get_decryption_key</a>(interval: u64): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.decryption_keys, interval)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.decryption_keys, interval))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="@Specification_7"></a>

## Specification



<pre><code><b>pragma</b> verify = <b>true</b>;
<b>pragma</b> aborts_if_is_strict;
</code></pre>


Helper to get the TimelockState resource


<a id="0x1_timelock_spec_timelock_state"></a>


<pre><code><b>fun</b> <a href="timelock.md#0x1_timelock_spec_timelock_state">spec_timelock_state</a>(): <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
   <b>borrow_global</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)
}
</code></pre>


Invariant: current_interval is always non-negative (implied by u64, but useful anchor)
Real invariant: last_rotation_time is never in the future relative to environment time

Invariant: current_interval is always non-negative (implied by u64, but useful anchor)
Real invariant: last_rotation_time is never in the future relative to environment time


<pre><code><b>invariant</b> <b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework) ==&gt;
    <a href="timelock.md#0x1_timelock_spec_timelock_state">spec_timelock_state</a>().last_rotation_time &lt;= aptos_framework::timestamp::spec_now_microseconds();
</code></pre>



<a id="@Specification_7_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>let</b> addr = std::signer::address_of(framework);
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(addr);
</code></pre>



<a id="@Specification_7_on_new_block"></a>

### Function `on_new_block`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>let</b> addr = std::signer::address_of(vm);
<b>aborts_if</b> addr != @vm_reserved;
</code></pre>



<a id="@Specification_7_publish_master_public_key"></a>

### Function `publish_master_public_key`


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_master_public_key">publish_master_public_key</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, interval: u64, pk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<a id="@Specification_7_publish_decryption_key_share"></a>

### Function `publish_decryption_key_share`


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_decryption_key_share">publish_decryption_key_share</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, interval: u64, share: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
