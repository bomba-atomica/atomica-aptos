
<a id="0x1_timelock"></a>

# Module `0x1::timelock`



-  [Struct `DecryptionKeyShare`](#0x1_timelock_DecryptionKeyShare)
-  [Struct `TimelockConfig`](#0x1_timelock_TimelockConfig)
-  [Resource `TimelockState`](#0x1_timelock_TimelockState)
-  [Struct `StartKeyGenEvent`](#0x1_timelock_StartKeyGenEvent)
-  [Struct `TimelockRegisteredEvent`](#0x1_timelock_TimelockRegisteredEvent)
-  [Struct `DeadlineReachedEvent`](#0x1_timelock_DeadlineReachedEvent)
-  [Struct `DecryptionKeyRevealedEvent`](#0x1_timelock_DecryptionKeyRevealedEvent)
-  [Constants](#@Constants_0)
    -  [Atomica Timelock Service (Registry Model)](#@Atomica_Timelock_Service_(Registry_Model)_1)
        -  [Flow](#@Flow_2)
-  [Function `initialize`](#0x1_timelock_initialize)
-  [Function `register`](#0x1_timelock_register)
-  [Function `insert_pending_deadline`](#0x1_timelock_insert_pending_deadline)
-  [Function `on_new_block`](#0x1_timelock_on_new_block)
-  [Function `publish_public_key`](#0x1_timelock_publish_public_key)
-  [Function `publish_decryption_key_share`](#0x1_timelock_publish_decryption_key_share)
-  [Function `compute_identity`](#0x1_timelock_compute_identity)
-  [Function `u64_to_string`](#0x1_timelock_u64_to_string)
-  [Function `get_deadline`](#0x1_timelock_get_deadline)
-  [Function `get_decryption_key`](#0x1_timelock_get_decryption_key)


<pre><code><b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/hash.md#0x1_aptos_hash">0x1::aptos_hash</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/bls12381_algebra.md#0x1_bls12381_algebra">0x1::bls12381_algebra</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/crypto_algebra.md#0x1_crypto_algebra">0x1::crypto_algebra</a>;
<b>use</b> <a href="event.md#0x1_event">0x1::event</a>;
<b>use</b> <a href="ibe_signature.md#0x1_ibe_signature">0x1::ibe_signature</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option">0x1::option</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">0x1::signer</a>;
<b>use</b> <a href="stake.md#0x1_stake">0x1::stake</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string">0x1::string</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
<b>use</b> <a href="../../aptos-stdlib/doc/table.md#0x1_table">0x1::table</a>;
<b>use</b> <a href="threshold_dsa.md#0x1_threshold_dsa">0x1::threshold_dsa</a>;
<b>use</b> <a href="timestamp.md#0x1_timestamp">0x1::timestamp</a>;
<b>use</b> <a href="validator_consensus_info.md#0x1_validator_consensus_info">0x1::validator_consensus_info</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">0x1::vector</a>;
</code></pre>



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

<a id="0x1_timelock_TimelockState"></a>

## Resource `TimelockState`



<pre><code><b>struct</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>next_timelock_id: u64</code>
</dt>
<dd>
 Counter for assigning unique IDs
</dd>
<dt>
<code>pending_deadlines: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
</dt>
<dd>
 Sorted vector of pending deadlines (ascending)
</dd>
<dt>
<code>deadline_to_ids: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;&gt;</code>
</dt>
<dd>
 Map from deadline -> List of Timelock IDs
</dd>
<dt>
<code>id_to_deadline: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, u64&gt;</code>
</dt>
<dd>
 Map from timelock_id -> Deadline (for verification)
</dd>
<dt>
<code>shares: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;<a href="timelock.md#0x1_timelock_DecryptionKeyShare">timelock::DecryptionKeyShare</a>&gt;&gt;</code>
</dt>
<dd>
 Store collected key shares: timelock_id -> shares
</dd>
<dt>
<code>decryption_keys: <a href="../../aptos-stdlib/doc/table.md#0x1_table_Table">table::Table</a>&lt;u64, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;</code>
</dt>
<dd>
 Store revealed keys: timelock_id -> key bytes
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

<a id="0x1_timelock_TimelockRegisteredEvent"></a>

## Struct `TimelockRegisteredEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="timelock.md#0x1_timelock_TimelockRegisteredEvent">TimelockRegisteredEvent</a> <b>has</b> drop, store
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
<code>deadline: u64</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_timelock_DeadlineReachedEvent"></a>

## Struct `DeadlineReachedEvent`



<pre><code>#[<a href="event.md#0x1_event">event</a>]
<b>struct</b> <a href="timelock.md#0x1_timelock_DeadlineReachedEvent">DeadlineReachedEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>deadline: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>timelock_ids: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;</code>
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
<code>timelock_id: u64</code>
</dt>
<dd>

</dd>
<dt>
<code>deadline: u64</code>
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


<a id="0x1_timelock_EINVALID_TIMESTAMP"></a>



<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_EINVALID_TIMESTAMP">EINVALID_TIMESTAMP</a>: u64 = 5;
</code></pre>



<a id="0x1_timelock_ENOT_VALIDATOR"></a>



<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_ENOT_VALIDATOR">ENOT_VALIDATOR</a>: u64 = 2;
</code></pre>



<a id="0x1_timelock_EDEADLINE_NOT_PASSED"></a>



<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_EDEADLINE_NOT_PASSED">EDEADLINE_NOT_PASSED</a>: u64 = 4;
</code></pre>



<a id="0x1_timelock_EINVALID_SHARE"></a>



<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_EINVALID_SHARE">EINVALID_SHARE</a>: u64 = 3;
</code></pre>



<a id="0x1_timelock_ESHARE_VERIFICATION_FAILED"></a>



<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_ESHARE_VERIFICATION_FAILED">ESHARE_VERIFICATION_FAILED</a>: u64 = 6;
</code></pre>



<a id="0x1_timelock_ETIMELOCK_NOT_INITIALIZED"></a>


<a id="@Atomica_Timelock_Service_(Registry_Model)_1"></a>

### Atomica Timelock Service (Registry Model)


Manages the registration of timelocks and the revelation of decryption keys
based on timestamps (deadlines).


<a id="@Flow_2"></a>

#### Flow

1. User calls <code><a href="timelock.md#0x1_timelock_register">register</a>(deadline)</code>.
2. When <code>now &gt;= deadline</code>, <code><a href="timelock.md#0x1_timelock_DeadlineReachedEvent">DeadlineReachedEvent</a></code> is emitted.
3. Validators submit decryption key shares for the specific <code>timelock_id</code>.
4. Decryption key is aggregated and published.


<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_ETIMELOCK_NOT_INITIALIZED">ETIMELOCK_NOT_INITIALIZED</a>: u64 = 1;
</code></pre>



<a id="0x1_timelock_MPK_ID"></a>



<pre><code><b>const</b> <a href="timelock.md#0x1_timelock_MPK_ID">MPK_ID</a>: u64 = 1;
</code></pre>



<a id="0x1_timelock_initialize"></a>

## Function `initialize`

Initialize the system


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) {
        // Ensure dependencies initialized
        <a href="threshold_dsa.md#0x1_threshold_dsa_initialize">threshold_dsa::initialize</a>(framework);

        <b>move_to</b>(framework, <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
            next_timelock_id: 2, // Start after <a href="timelock.md#0x1_timelock_MPK_ID">MPK_ID</a> (1)
            pending_deadlines: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>(),
            deadline_to_ids: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
            id_to_deadline: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
            shares: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
            decryption_keys: <a href="../../aptos-stdlib/doc/table.md#0x1_table_new">table::new</a>(),
        });

        // Trigger MPK Setup
        <b>let</b> validators = <a href="stake.md#0x1_stake_cur_validator_consensus_infos">stake::cur_validator_consensus_infos</a>();
        <b>let</b> n = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&validators);
        <b>let</b> threshold = (n * 2 / 3) + 1;
        <b>if</b> (n == 0) { n = 1; threshold = 1; };

        emit(<a href="timelock.md#0x1_timelock_StartKeyGenEvent">StartKeyGenEvent</a> {
            interval: <a href="timelock.md#0x1_timelock_MPK_ID">MPK_ID</a>,
            config: <a href="timelock.md#0x1_timelock_TimelockConfig">TimelockConfig</a> { threshold, total_validators: n },
        });
    }
}
</code></pre>



</details>

<a id="0x1_timelock_register"></a>

## Function `register`

Register a new timelock request


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_register">register</a>(deadline: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_register">register</a>(deadline: u64) <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>let</b> id = state.next_timelock_id;
    state.next_timelock_id = id + 1;

    // Validation
    <b>let</b> now = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();
    <b>assert</b>!(deadline &gt; now, <a href="timelock.md#0x1_timelock_EINVALID_TIMESTAMP">EINVALID_TIMESTAMP</a>);

    // Store mappings
    <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.id_to_deadline, id, deadline);

    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.deadline_to_ids, deadline)) {
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.deadline_to_ids, deadline, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>());
        // Add <b>to</b> pending deadlines (sorted insert)
        <a href="timelock.md#0x1_timelock_insert_pending_deadline">insert_pending_deadline</a>(&<b>mut</b> state.pending_deadlines, deadline);
    };

    <b>let</b> ids = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> state.deadline_to_ids, deadline);
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(ids, id);

    emit(<a href="timelock.md#0x1_timelock_TimelockRegisteredEvent">TimelockRegisteredEvent</a> {
        timelock_id: id,
        deadline,
    });
}
</code></pre>



</details>

<a id="0x1_timelock_insert_pending_deadline"></a>

## Function `insert_pending_deadline`

Internal: Insert deadline into sorted vector


<pre><code><b>fun</b> <a href="timelock.md#0x1_timelock_insert_pending_deadline">insert_pending_deadline</a>(deadlines: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, deadline: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="timelock.md#0x1_timelock_insert_pending_deadline">insert_pending_deadline</a>(deadlines: &<b>mut</b> <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u64&gt;, deadline: u64) {
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(deadlines);
    <b>if</b> (len == 0) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(deadlines, deadline);
        <b>return</b>
    };
    // Optimization: Check <b>if</b> it belongs at the end (common case)
    <b>if</b> (deadline &gt;= *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(deadlines, len - 1)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(deadlines, deadline);
        <b>return</b>
    };

    // Find insertion point
    <b>let</b> i = 0;
    <b>while</b> (i &lt; len) {
        <b>if</b> (*<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(deadlines, i) &gt; deadline) {
            <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_insert">vector::insert</a>(deadlines, i, deadline);
            <b>return</b>
        };
        i = i + 1;
    };
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(deadlines, deadline);
}
</code></pre>



</details>

<a id="0x1_timelock_on_new_block"></a>

## Function `on_new_block`

On New Block: Check for passed deadlines


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock.md#0x1_timelock_on_new_block">on_new_block</a>(vm: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <a href="system_addresses.md#0x1_system_addresses_assert_vm">system_addresses::assert_vm</a>(vm);
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) <b>return</b>;

    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>let</b> now = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();

    // Process pending deadlines &lt;= now
    <b>while</b> (!<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_is_empty">vector::is_empty</a>(&state.pending_deadlines)) {
        <b>let</b> next_deadline = *<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(&state.pending_deadlines, 0);

        <b>if</b> (next_deadline &gt; now) {
            <b>break</b> // No more deadlines <b>to</b> process
        };

        // Remove from pending
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_remove">vector::remove</a>(&<b>mut</b> state.pending_deadlines, 0);

        // Get IDs and emit <a href="event.md#0x1_event">event</a>
        <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.deadline_to_ids, next_deadline)) {
            <b>let</b> ids = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.deadline_to_ids, next_deadline);
            emit(<a href="timelock.md#0x1_timelock_DeadlineReachedEvent">DeadlineReachedEvent</a> {
                deadline: next_deadline,
                timelock_ids: *ids,
            });
        };
    };
}
</code></pre>



</details>

<a id="0x1_timelock_publish_public_key"></a>

## Function `publish_public_key`

Submit a decryption key share


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_public_key">publish_public_key</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, timelock_id: u64, mpk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_public_key">publish_public_key</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, timelock_id: u64, mpk: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    <a href="threshold_dsa.md#0x1_threshold_dsa_publish_master_public_key">threshold_dsa::publish_master_public_key</a>(validator, timelock_id, mpk);
}
</code></pre>



</details>

<a id="0x1_timelock_publish_decryption_key_share"></a>

## Function `publish_decryption_key_share`



<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_decryption_key_share">publish_decryption_key_share</a>(validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, timelock_id: u64, share: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock.md#0x1_timelock_publish_decryption_key_share">publish_decryption_key_share</a>(
    validator: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    timelock_id: u64,
    share: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
) <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>let</b> validator_addr = std::signer::address_of(validator);
    <b>assert</b>!(<a href="stake.md#0x1_stake_is_current_epoch_validator">stake::is_current_epoch_validator</a>(validator_addr), <a href="timelock.md#0x1_timelock_ENOT_VALIDATOR">ENOT_VALIDATOR</a>);

    <b>let</b> state = <b>borrow_global_mut</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);

    // 1. Verify Deadline Passed
    <b>assert</b>!(<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.id_to_deadline, timelock_id), <a href="timelock.md#0x1_timelock_EINVALID_TIMESTAMP">EINVALID_TIMESTAMP</a>);
    <b>let</b> deadline = *<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.id_to_deadline, timelock_id);
    <b>let</b> now = <a href="timestamp.md#0x1_timestamp_now_microseconds">timestamp::now_microseconds</a>();

    // Allow slightly early submission? No, must strict.
    <b>assert</b>!(now &gt;= deadline, <a href="timelock.md#0x1_timelock_EDEADLINE_NOT_PASSED">EDEADLINE_NOT_PASSED</a>);

    // Deduplicate
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.decryption_keys, timelock_id)) <b>return</b>; // Already revealed

    // 2. Compute Identity
    // Format: "timelock_id:{id}:deadline_timestamp_microseconds:{deadline}"
    <b>let</b> identity = <a href="timelock.md#0x1_timelock_compute_identity">compute_identity</a>(timelock_id, deadline);

    // 3. Verify Share
    <b>let</b> is_valid = <a href="ibe_signature.md#0x1_ibe_signature_verify_private_key">ibe_signature::verify_private_key</a>(<a href="timelock.md#0x1_timelock_MPK_ID">MPK_ID</a>, identity, share);
    <b>assert</b>!(is_valid, <a href="timelock.md#0x1_timelock_ESHARE_VERIFICATION_FAILED">ESHARE_VERIFICATION_FAILED</a>);

    // 4. Store & Aggregate
    <b>if</b> (!<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.shares, timelock_id)) {
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.shares, timelock_id, <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>());
    };
    <b>let</b> shares = <a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow_mut">table::borrow_mut</a>(&<b>mut</b> state.shares, timelock_id);

    // Validator dedup
    <b>let</b> i = 0;
    <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shares);
    <b>while</b> (i &lt; len) {
        <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(shares, i).validator == validator_addr) <b>return</b>;
        i = i + 1;
    };

    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(shares, <a href="timelock.md#0x1_timelock_DecryptionKeyShare">DecryptionKeyShare</a> { validator: validator_addr, share });

    // Check threshold
    <b>let</b> voters = <a href="stake.md#0x1_stake_cur_validator_consensus_infos">stake::cur_validator_consensus_infos</a>();
    <b>let</b> n = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(&voters);
    <b>let</b> threshold = (n * 2 / 3) + 1;
    <b>if</b> (n == 0) { threshold = 1; };

    <b>if</b> (<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shares) &gt;= threshold) {
        // Aggregate
        <b>let</b> sum = zero&lt;G1&gt;();
        <b>let</b> count = 0;
        <b>let</b> i = 0;
        <b>let</b> len = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_length">vector::length</a>(shares);

        <b>while</b> (i &lt; len && count &lt; threshold) {
            <b>let</b> s = &<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_borrow">vector::borrow</a>(shares, i).share;
            <b>let</b> elem_opt = deserialize&lt;G1, FormatG1Compr&gt;(s);
            <b>if</b> (std::option::is_some(&elem_opt)) {
                <b>let</b> elem = std::option::extract(&<b>mut</b> elem_opt);
                sum = add(&sum, &elem);
                count = count + 1;
            };
            i = i + 1;
        };

        <b>let</b> key_bytes = serialize&lt;G1, FormatG1Compr&gt;(&sum);
        <a href="../../aptos-stdlib/doc/table.md#0x1_table_add">table::add</a>(&<b>mut</b> state.decryption_keys, timelock_id, key_bytes);

        emit(<a href="timelock.md#0x1_timelock_DecryptionKeyRevealedEvent">DecryptionKeyRevealedEvent</a> {
            timelock_id,
            deadline,
            decryption_key: key_bytes,
        });
    };
}
</code></pre>



</details>

<a id="0x1_timelock_compute_identity"></a>

## Function `compute_identity`

Construct canonical identity string and hash it


<pre><code><b>fun</b> <a href="timelock.md#0x1_timelock_compute_identity">compute_identity</a>(timelock_id: u64, deadline: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="timelock.md#0x1_timelock_compute_identity">compute_identity</a>(timelock_id: u64, deadline: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt; {
    // "timelock_id:{id}:deadline_timestamp_microseconds:{deadline}"
    <b>let</b> str = <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"timelock_id:");
    <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_append">string::append</a>(&<b>mut</b> str, <a href="timelock.md#0x1_timelock_u64_to_string">u64_to_string</a>(timelock_id));
    <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_append">string::append</a>(&<b>mut</b> str, <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b":deadline_timestamp_microseconds:"));
    <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_append">string::append</a>(&<b>mut</b> str, <a href="timelock.md#0x1_timelock_u64_to_string">u64_to_string</a>(deadline));

    keccak256(*<a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_bytes">string::bytes</a>(&str))
}
</code></pre>



</details>

<a id="0x1_timelock_u64_to_string"></a>

## Function `u64_to_string`



<pre><code><b>fun</b> <a href="timelock.md#0x1_timelock_u64_to_string">u64_to_string</a>(value: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_String">string::String</a>
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>fun</b> <a href="timelock.md#0x1_timelock_u64_to_string">u64_to_string</a>(value: u64): String {
    <b>if</b> (value == 0) {
        <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(b"0")
    };
    <b>let</b> buf = <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_empty">vector::empty</a>&lt;u8&gt;();
    <b>while</b> (value &gt; 0) {
        <b>let</b> digit = ((value % 10) <b>as</b> u8);
        <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_push_back">vector::push_back</a>(&<b>mut</b> buf, digit + 48); // '0' is 48
        value = value / 10;
    };
    <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector_reverse">vector::reverse</a>(&<b>mut</b> buf);
    <a href="../../aptos-stdlib/../move-stdlib/doc/string.md#0x1_string_utf8">string::utf8</a>(buf)
}
</code></pre>



</details>

<a id="0x1_timelock_get_deadline"></a>

## Function `get_deadline`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_deadline">get_deadline</a>(timelock_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;u64&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_deadline">get_deadline</a>(timelock_id: u64): Option&lt;u64&gt; <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.id_to_deadline, timelock_id)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.id_to_deadline, timelock_id))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>

<a id="0x1_timelock_get_decryption_key"></a>

## Function `get_decryption_key`



<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_decryption_key">get_decryption_key</a>(timelock_id: u64): <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_Option">option::Option</a>&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt;
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timelock.md#0x1_timelock_get_decryption_key">get_decryption_key</a>(timelock_id: u64): Option&lt;<a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;&gt; <b>acquires</b> <a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework)) <b>return</b> <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>();
    <b>let</b> state = <b>borrow_global</b>&lt;<a href="timelock.md#0x1_timelock_TimelockState">TimelockState</a>&gt;(@aptos_framework);
    <b>if</b> (<a href="../../aptos-stdlib/doc/table.md#0x1_table_contains">table::contains</a>(&state.decryption_keys, timelock_id)) {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_some">option::some</a>(*<a href="../../aptos-stdlib/doc/table.md#0x1_table_borrow">table::borrow</a>(&state.decryption_keys, timelock_id))
    } <b>else</b> {
        <a href="../../aptos-stdlib/../move-stdlib/doc/option.md#0x1_option_none">option::none</a>()
    }
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
