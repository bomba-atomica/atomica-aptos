
<a id="0x1_timelock"></a>

# Module `0x1::timelock`



-  [Struct `TimelockConfig`](#0x1_timelock_TimelockConfig)
-  [Struct `IntervalConfig`](#0x1_timelock_IntervalConfig)
-  [Struct `ValidatorShare`](#0x1_timelock_ValidatorShare)
-  [Resource `TimelockState`](#0x1_timelock_TimelockState)
-  [Struct `StartKeyGenEvent`](#0x1_timelock_StartKeyGenEvent)
-  [Struct `KeyPublishedEvent`](#0x1_timelock_KeyPublishedEvent)
-  [Struct `RequestRevealEvent`](#0x1_timelock_RequestRevealEvent)
-  [Struct `SecretRevealedEvent`](#0x1_timelock_SecretRevealedEvent)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_timelock_initialize)
-  [Function `on_dkg_complete`](#0x1_timelock_on_dkg_complete)
-  [Function `perform_rotation`](#0x1_timelock_perform_rotation)
-  [Function `on_new_block`](#0x1_timelock_on_new_block)
-  [Function `trigger_rotation`](#0x1_timelock_trigger_rotation)
-  [Function `force_rotation_for_testing`](#0x1_timelock_force_rotation_for_testing)
-  [Function `publish_public_key`](#0x1_timelock_publish_public_key)
-  [Function `publish_secret_share`](#0x1_timelock_publish_secret_share)
-  [Function `get_current_interval`](#0x1_timelock_get_current_interval)
-  [Function `get_interval_config`](#0x1_timelock_get_interval_config)
-  [Function `get_public_key`](#0x1_timelock_get_public_key)
-  [Function `is_secret_revealed`](#0x1_timelock_is_secret_revealed)
-  [Function `get_secret`](#0x1_timelock_get_secret)
-  [Specification](#@Specification_1)
    -  [Function `initialize`](#@Specification_1_initialize)
    -  [Function `on_new_block`](#@Specification_1_on_new_block)
    -  [Function `publish_public_key`](#@Specification_1_publish_public_key)
    -  [Function `publish_secret_share`](#@Specification_1_publish_secret_share)


<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">0x1::bls12381_algebra</a>;
<b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/debug.md#0x1_debug">0x1::debug</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
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

<a id="0x1_timelock_ValidatorShare"></a>

## Struct `ValidatorShare`



<pre><code><b>struct</b> <a href="timelock.md#0x1_timelock_ValidatorShare">ValidatorShare</a> <b>has</b> drop, store
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
<code>public_keys: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Store public keys (for encryption)
</dd>
<dt>
<code>validator_shares: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="timelock.md#0x1_timelock_ValidatorShare">timelock::ValidatorShare</a>&gt;&gt;</code>
</dt>
<dd>
 Store collected shares before aggregation
</dd>
<dt>
<code>revealed_secrets: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Store revealed secret keys/signatures (for decryption)
</dd>
<dt>
<code>interval_configs: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="timelock.md#0x1_timelock_IntervalConfig">timelock::IntervalConfig</a>&gt;</code>
</dt>
<dd>
 Store historical interval configurations
</dd>
<dt>
<code>start_keygen_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="timelock.md#0x1_timelock_StartKeyGenEvent">timelock::StartKeyGenEvent</a>&gt;</code>
</dt>
<dd>
 Events
</dd>
<dt>
<code>key_published_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="timelock.md#0x1_timelock_KeyPublishedEvent">timelock::KeyPublishedEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>request_reveal_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="timelock.md#0x1_timelock_RequestRevealEvent">timelock::RequestRevealEvent</a>&gt;</code>
</dt>
<dd>

</dd>
<dt>
<code>secret_revealed_events: <a href="event.md#0x1_event_EventHandle">event::EventHandle</a>&lt;<a href="timelock.md#0x1_timelock_SecretRevealedEvent">timelock::SecretRevealedEvent</a>&gt;</code>
</dt>
<dd>

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

<a id="0x1_timelock_KeyPublishedEvent"></a>

## Struct `KeyPublishedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="timelock.md#0x1_timelock_KeyPublishedEvent">KeyPublishedEvent</a> <b>has</b> drop, store
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
<code>public_key: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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

<a id="0x1_timelock_SecretRevealedEvent"></a>

## Struct `SecretRevealedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="timelock.md#0x1_timelock_SecretRevealedEvent">SecretRevealedEvent</a> <b>has</b> drop, store
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
<code>secret: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
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



<a id="0x1_timelock_ETIMELOCK_NOT_INITIALIZED"></a>

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
    <b>move_to</b>(framework, <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
        current_interval: 0,
        last_rotation_time: 0, // Will be updated on first <a href="block.md#0x1_block">block</a>
        public_keys: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
        validator_shares: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
        revealed_secrets: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
        interval_configs: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
        start_keygen_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="timelock.md#0x1_timelock_StartKeyGenEvent">StartKeyGenEvent</a>&gt;(framework),
        key_published_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="timelock.md#0x1_timelock_KeyPublishedEvent">KeyPublishedEvent</a>&gt;(framework),
        request_reveal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="timelock.md#0x1_timelock_RequestRevealEvent">RequestRevealEvent</a>&gt;(framework),
        secret_revealed_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="timelock.md#0x1_timelock_SecretRevealedEvent">SecretRevealedEvent</a>&gt;(framework),
    });
}
</code></pre>



</details>

<a id="0x1_timelock_on_dkg_complete"></a>

## Function `on_dkg_complete`

Called when DKG completes to publish the transcript for timelock use.
This is a friend function called from reconfiguration_with_dkg module.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_on_dkg_complete">on_dkg_complete</a>(transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_on_dkg_complete">on_dkg_complete</a>(transcript: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        <b>return</b>
    };

    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>let</b> current_interval = state.current_interval;

    // Only publish <b>if</b> not already present
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.public_keys, current_interval)) {
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.public_keys, current_interval, transcript);

        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="timelock.md#0x1_timelock_KeyPublishedEvent">KeyPublishedEvent</a> {
            interval: current_interval,
            public_key: transcript,
        });
    };
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
    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="timelock.md#0x1_timelock_RequestRevealEvent">RequestRevealEvent</a> {
        interval: old_interval,
    });

    state.current_interval = state.current_interval + 1;
    state.last_rotation_time = now;

    // DEBUG: Log interval rotation
    std::debug::print(&b"[TIMELOCK] Interval rotated <b>to</b>");
    std::debug::print(&state.current_interval);

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

    <a href="event.md#0x1_event_emit">event::emit</a>(<a href="timelock.md#0x1_timelock_StartKeyGenEvent">StartKeyGenEvent</a> {
        interval: state.current_interval,
        config,
    });
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
    <b>if</b> (now - state.last_rotation_time &gt; interval_micros) {
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
    <b>assert</b>!(<a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>() != 1, <a href="timelock.md#0x1_timelock_EROTATION_TOO_EARLY">EROTATION_TOO_EARLY</a>); // Re-<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">error</a> or new one? EPRODUCTION... logic

    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        <b>return</b>
    };

    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <a href="timelock.md#0x1_timelock_perform_rotation">perform_rotation</a>(state);
}
</code></pre>



</details>

<a id="0x1_timelock_publish_public_key"></a>

## Function `publish_public_key`

validators call this to publish the public key for a future interval


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_public_key">publish_public_key</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, interval: u64, pk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_public_key">publish_public_key</a>(
    validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    interval: u64,
    pk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>let</b> validator_addr = std::signer::address_of(validator);
    // Verify sender is a validator
    <b>assert</b>!(<a href="stake.md#0x1_stake_is_current_epoch_validator">stake::is_current_epoch_validator</a>(validator_addr), <a href="timelock.md#0x1_timelock_ENOT_VALIDATOR">ENOT_VALIDATOR</a>);

    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.public_keys, interval)) {
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.public_keys, interval, pk);

        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="timelock.md#0x1_timelock_KeyPublishedEvent">KeyPublishedEvent</a> {
            interval,
            public_key: pk,
        });
    };
}
</code></pre>



</details>

<a id="0x1_timelock_publish_secret_share"></a>

## Function `publish_secret_share`

validators call this to publish the secret share/signature for a past interval


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_secret_share">publish_secret_share</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, interval: u64, share: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_secret_share">publish_secret_share</a>(
    validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    interval: u64,
    share: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>let</b> validator_addr = std::signer::address_of(validator);
    // DEBUG: Log secret share publication attempt
    std::debug::print(&b"[TIMELOCK] publish_secret_share called");
    std::debug::print(&interval);
    std::debug::print(&validator_addr);

    // 1. Verify validator authorization
    <b>assert</b>!(<a href="stake.md#0x1_stake_is_current_epoch_validator">stake::is_current_epoch_validator</a>(validator_addr), <a href="timelock.md#0x1_timelock_ENOT_VALIDATOR">ENOT_VALIDATOR</a>);

    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);

    // CRITICAL SECURITY: Only allow revealing PAST intervals
    // Validators must not be able <b>to</b> reveal the current interval's secret.
    // The <a href="timelock.md#0x1_timelock">timelock</a> guarantee is that secrets remain hidden until the interval rotates.
    // Without this check, malicious validators could immediately reveal secrets for the
    // current interval, completely breaking the <a href="timelock.md#0x1_timelock">timelock</a> security model.
    <b>assert</b>!(interval &lt; state.current_interval, <a href="timelock.md#0x1_timelock_EINVALID_INTERVAL">EINVALID_INTERVAL</a>);

    // If already revealed, ignore (or could <b>abort</b>)
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.revealed_secrets, interval)) {
        <b>return</b>
    };

    // 2. Store the share
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.validator_shares, interval)) {
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.validator_shares, interval, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>());
    };
    <b>let</b> shares_list = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> state.validator_shares, interval);

    // Dedup: check <b>if</b> validator already submitted
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shares_list);
    <b>while</b> (i &lt; len) {
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(shares_list, i).validator == validator_addr) {
            <b>return</b> // Already submitted
        };
        i = i + 1;
    };

    // 2. Validate share format BEFORE storing
    <b>let</b> share_opt = deserialize&lt;G1, FormatG1Compr&gt;(&share);
    <b>assert</b>!(std::option::is_some(&share_opt), <a href="timelock.md#0x1_timelock_EINVALID_SHARE">EINVALID_SHARE</a>);

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(shares_list, <a href="timelock.md#0x1_timelock_ValidatorShare">ValidatorShare</a> {
        validator: validator_addr,
        share: share,
    });

    // 3. Check <b>if</b> threshold is met using VALID shares only
    // Since we validate on insertion (line 254), all stored shares are valid G1 points.
    <b>let</b> valid_count = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shares_list);

    // Use stored interval config for threshold validation
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.interval_configs, interval), <a href="timelock.md#0x1_timelock_EINVALID_INTERVAL">EINVALID_INTERVAL</a>);
    <b>let</b> config = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.interval_configs, interval);
    <b>let</b> threshold = config.threshold;

    <b>if</b> (valid_count &gt;= threshold) {
        // 4. Aggregate VALID shares only
        // 4. Aggregate shares
        <b>let</b> sum = zero&lt;G1&gt;();
        <b>let</b> i = 0;
        // distinct from valid_count, just <b>loop</b> iterator
        <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shares_list);
        <b>let</b> aggregated_count = 0;

        <b>while</b> (i &lt; len && aggregated_count &lt; threshold) {
            <b>let</b> s_bytes = &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(shares_list, i).share;
            // We must re-deserialize <b>to</b> add, but we can trust it is Some
            <b>let</b> element_opt = deserialize&lt;G1, FormatG1Compr&gt;(s_bytes);
            // Safety check, though redundant <b>if</b> storage is trusted
            <b>if</b> (std::option::is_some(&element_opt)) {
                <b>let</b> element = std::option::extract(&<b>mut</b> element_opt);
                sum = add(&sum, &element);
                aggregated_count = aggregated_count + 1;
            };
            i = i + 1;
        };

        <b>let</b> aggregated_bytes = serialize&lt;G1, FormatG1Compr&gt;(&sum);
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.revealed_secrets, interval, aggregated_bytes);

        // Emit <a href="event.md#0x1_event">event</a>
        <a href="event.md#0x1_event_emit">event::emit</a>(<a href="timelock.md#0x1_timelock_SecretRevealedEvent">SecretRevealedEvent</a> {
            interval,
            secret: aggregated_bytes,
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

<a id="0x1_timelock_get_public_key"></a>

## Function `get_public_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_public_key">get_public_key</a>(interval: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_public_key">get_public_key</a>(interval: u64): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.public_keys, interval)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.public_keys, interval))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_timelock_is_secret_revealed"></a>

## Function `is_secret_revealed`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_is_secret_revealed">is_secret_revealed</a>(interval: u64): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_is_secret_revealed">is_secret_revealed</a>(interval: u64): bool <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        <b>return</b> <b>false</b>
    };
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.revealed_secrets, interval)
}
</code></pre>



</details>

<a id="0x1_timelock_get_secret"></a>

## Function `get_secret`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_secret">get_secret</a>(interval: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_secret">get_secret</a>(interval: u64): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    };
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.revealed_secrets, interval)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.revealed_secrets, interval))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="@Specification_1"></a>

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



<a id="@Specification_1_initialize"></a>

### Function `initialize`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>let</b> addr = std::signer::address_of(framework);
<b>aborts_if</b> !<a href="system_addresses.md#0x1_system_addresses_is_aptos_framework_address">system_addresses::is_aptos_framework_address</a>(addr);
<b>aborts_if</b> <b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(addr);
<b>ensures</b> <b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(addr);
</code></pre>



<a id="@Specification_1_on_new_block"></a>

### Function `on_new_block`


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>




<pre><code><b>let</b> addr = std::signer::address_of(vm);
<b>aborts_if</b> addr != @vm_reserved;
</code></pre>



<a id="@Specification_1_publish_public_key"></a>

### Function `publish_public_key`


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_public_key">publish_public_key</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, interval: u64, pk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>




<a id="@Specification_1_publish_secret_share"></a>

### Function `publish_secret_share`


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_secret_share">publish_secret_share</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, interval: u64, share: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>


[move-book]: https://aptos.dev/move/book/SUMMARY
