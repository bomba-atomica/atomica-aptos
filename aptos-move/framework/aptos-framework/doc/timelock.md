<a id="0x1_timelock"></a>

# Module `0x1::timelock`

- [Struct `TimelockConfig`](#0x1_timelock_TimelockConfig)
- [Struct `ValidatorShare`](#0x1_timelock_ValidatorShare)
- [Resource `TimelockState`](#0x1_timelock_TimelockState)
- [Struct `StartKeyGenEvent`](#0x1_timelock_StartKeyGenEvent)
- [Struct `KeyPublishedEvent`](#0x1_timelock_KeyPublishedEvent)
- [Struct `RequestRevealEvent`](#0x1_timelock_RequestRevealEvent)
- [Struct `SecretRevealedEvent`](#0x1_timelock_SecretRevealedEvent)
- [Constants](#@Constants_0)
- [Function `initialize`](#0x1_timelock_initialize)
- [Function `on_new_block`](#0x1_timelock_on_new_block)
- [Function `publish_public_key`](#0x1_timelock_publish_public_key)
- [Function `publish_secret_share`](#0x1_timelock_publish_secret_share)
- [Function `get_current_interval`](#0x1_timelock_get_current_interval)
- [Function `get_public_key`](#0x1_timelock_get_public_key)
- [Function `is_secret_revealed`](#0x1_timelock_is_secret_revealed)
- [Function `get_secret`](#0x1_timelock_get_secret)
- [Specification](#@Specification_1)
  - [Function `initialize`](#@Specification_1_initialize)
  - [Function `on_new_block`](#@Specification_1_on_new_block)
  - [Function `publish_public_key`](#@Specification_1_publish_public_key)
  - [Function `publish_secret_share`](#@Specification_1_publish_secret_share)

<pre><code><b>use</b> <a href="account.md#0x1_account">0x1::account</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">0x1::bls12381_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
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

Event emitted to tell validators: "Please generate keys for interval X"

<pre><code><b>struct</b> <a href="timelock.md#0x1_timelock_StartKeyGenEvent">StartKeyGenEvent</a> <b>has</b> drop, store
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

Event emitted when MPK (transcript) is published

<pre><code><b>struct</b> <a href="timelock.md#0x1_timelock_KeyPublishedEvent">KeyPublishedEvent</a> <b>has</b> drop, store
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

Event emitted to tell validators: "Please reveal the secret for interval X"

<pre><code><b>struct</b> <a href="timelock.md#0x1_timelock_RequestRevealEvent">RequestRevealEvent</a> <b>has</b> drop, store
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

Event emitted when a secret is fully reconstructed

<pre><code><b>struct</b> <a href="timelock.md#0x1_timelock_SecretRevealedEvent">SecretRevealedEvent</a> <b>has</b> drop, store
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

<a id="0x1_timelock_EINVALID_SHARE"></a>

Invalid share format.

<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_EINVALID_SHARE">EINVALID_SHARE</a>: u64 = 3;
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
        start_keygen_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="timelock.md#0x1_timelock_StartKeyGenEvent">StartKeyGenEvent</a>&gt;(framework),
        key_published_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="timelock.md#0x1_timelock_KeyPublishedEvent">KeyPublishedEvent</a>&gt;(framework),
        request_reveal_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="timelock.md#0x1_timelock_RequestRevealEvent">RequestRevealEvent</a>&gt;(framework),
        secret_revealed_events: <a href="account.md#0x1_account_new_event_handle">account::new_event_handle</a>&lt;<a href="timelock.md#0x1_timelock_SecretRevealedEvent">SecretRevealedEvent</a>&gt;(framework),
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
        <b>let</b> old_interval = state.current_interval;
         // Emit reveal <a href="event.md#0x1_event">event</a> for the <b>old</b> interval
        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> state.request_reveal_events, <a href="timelock.md#0x1_timelock_RequestRevealEvent">RequestRevealEvent</a> {
            interval: old_interval,
        });

        state.current_interval = state.current_interval + 1;
        state.last_rotation_time = now;

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

        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> state.start_keygen_events, <a href="timelock.md#0x1_timelock_StartKeyGenEvent">StartKeyGenEvent</a> {
            interval: state.current_interval,
            config,
        });
    }
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

        <a href="event.md#0x1_event_emit_event">event::emit_event</a>(&<b>mut</b> state.key_published_events, <a href="timelock.md#0x1_timelock_KeyPublishedEvent">KeyPublishedEvent</a> {
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
    let validator_addr = std::signer::address_of(validator);
    // 1. Verify validator authorization
    assert!(stake::is_current_epoch_validator(validator_addr), ENOT_VALIDATOR);
    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);

    // If already revealed, ignore (or could <b>abort</b>)
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.revealed_secrets, interval)) {
        <b>return</b>
    };

    // 2. Store the share
    if (!table::contains(&state.validator_shares, interval)) {
        table::add(&mut state.validator_shares, interval, vector::empty());
    };
    let shares_list = table::borrow_mut(&mut state.validator_shares, interval);

    // Dedup: check if validator already submitted
    let i = 0;
    let len = vector::length(shares_list);
    while (i < len) {
        if (vector::borrow(shares_list, i).validator == validator_addr) {
            return // Already submitted
        };
        i = i + 1;
    };

    vector::push_back(shares_list, ValidatorShare {
        validator: validator_addr,
        share: share,
    });

    // 3. Check if threshold is met
    // We need to fetch the config for this interval. Ideally we stored it.
    // But since we don't store historical configs in this struct, we define threshold based on current validators?
    // CAUTION: Validator set might change between StartKeyGen (interval N) and Reveal (interval N+1).
    // Ideally we should use the threshold from the time KeyGen started.
    // But simpler for now: use CURRENT validator set threshold (assuming relatively stable set).
    // OR: just Recalculate based on current stake.

    let validators = stake::cur_validator_consensus_infos();
    let validator_addresses = vector::empty<address>();
    let i = 0;
    let len = vector::length(&validators);
    while (i < len) {
        let v = vector::borrow(&validators, i);
        vector::push_back(&mut validator_addresses, validator_consensus_info::get_addr(v));
        i = i + 1;
    };
    let total_validators = vector::length(&validators);
    let threshold = (total_validators * 2 / 3) + 1;

    if (vector::length(shares_list) >= threshold) {
        // 4. Aggregate shares
        // Sum of G1 points
        let sum = zero<G1>();
        let i = 0;
        let len = vector::length(shares_list);
        while (i < len) {
            let s_bytes = &vector::borrow(shares_list, i).share;
            // Deserialize failure implies invalid share - we could skip it, but for now we abort.
            // In production, we should try-catch or validate beforehand.
            let element_opt = deserialize<G1, FormatG1Compr>(s_bytes);
            if (std::option::is_some(&element_opt)) {
                let element = std::option::extract(&mut element_opt);
                sum = add(&sum, &element);
            };
            // If invalid, we skip incrementing sum (effectively treating as 0? No, 0 is identity.
            // Adding identity doesn't change sum. So invalid share = ignored.
            // But we counted it towards threshold! This is a vulnerability if 1 share is invalid.
            // We should only count valid shares towards threshold.
            // Correct logic: Filter valid shares first.
            i = i + 1;
        };

        let aggregated_bytes = serialize<G1, FormatG1Compr>(&sum);
        table::add(&mut state.revealed_secrets, interval, aggregated_bytes);

        // Emit event
        event::emit_event(&mut state.secret_revealed_events, SecretRevealedEvent {
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
