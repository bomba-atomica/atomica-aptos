
<a id="0x1_timelock_config"></a>

# Module `0x1::timelock_config`

Configuration for timelock encryption intervals.

This module manages the interval duration for timelock key rotation. The interval
determines how frequently new timelock keys are generated via DKG, and when old
keys are revealed for decryption.

Default: 1 hour (production)
Test: Configurable via <code><a href="timelock_config.md#0x1_timelock_config_set_interval_for_testing">set_interval_for_testing</a>()</code> on non-mainnet chains


-  [Resource `TimelockConfig`](#0x1_timelock_config_TimelockConfig)
-  [Constants](#@Constants_0)
-  [Function `initialize`](#0x1_timelock_config_initialize)
-  [Function `set_interval_for_testing`](#0x1_timelock_config_set_interval_for_testing)
    -  [Security](#@Security_1)
    -  [Arguments](#@Arguments_2)
-  [Function `get_interval_microseconds`](#0x1_timelock_config_get_interval_microseconds)


<pre><code><b>use</b> <a href="chain_id.md#0x1_chain_id">0x1::chain_id</a>;
<b>use</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error">0x1::error</a>;
<b>use</b> <a href="system_addresses.md#0x1_system_addresses">0x1::system_addresses</a>;
</code></pre>



<a id="0x1_timelock_config_TimelockConfig"></a>

## Resource `TimelockConfig`

Global configuration for timelock intervals.


<pre><code><b>struct</b> <a href="timelock_config.md#0x1_timelock_config_TimelockConfig">TimelockConfig</a> <b>has</b> key
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>interval_microseconds: u64</code>
</dt>
<dd>
 Interval duration in microseconds.
 Default: 5 seconds = 5 * 1_000_000 microseconds
 TODO: Change to 1 hour (3600 * 1_000_000) for production deployment
</dd>
</dl>


</details>

<a id="@Constants_0"></a>

## Constants


<a id="0x1_timelock_config_EPRODUCTION_OVERRIDE_FORBIDDEN"></a>

Cannot override interval in production (mainnet)


<pre><code><b>const</b> <a href="timelock_config.md#0x1_timelock_config_EPRODUCTION_OVERRIDE_FORBIDDEN">EPRODUCTION_OVERRIDE_FORBIDDEN</a>: u64 = 2;
</code></pre>



<a id="0x1_timelock_config_ETIMELOCK_CONFIG_NOT_FOUND"></a>

Timelock interval configuration is not initialized


<pre><code><b>const</b> <a href="timelock_config.md#0x1_timelock_config_ETIMELOCK_CONFIG_NOT_FOUND">ETIMELOCK_CONFIG_NOT_FOUND</a>: u64 = 1;
</code></pre>



<a id="0x1_timelock_config_initialize"></a>

## Function `initialize`

Initialize with default 5-second interval.
Called during genesis to set up the timelock configuration.

NOTE: Currently set to 5 seconds for testing/development.
TODO: Change to 1 hour for production mainnet deployment.


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock_config.md#0x1_timelock_config_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b>(<b>friend</b>) <b>fun</b> <a href="timelock_config.md#0x1_timelock_config_initialize">initialize</a>(framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>) {
    <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock_config.md#0x1_timelock_config_TimelockConfig">TimelockConfig</a>&gt;(@aptos_framework)) {
        <b>move_to</b>(framework, <a href="timelock_config.md#0x1_timelock_config_TimelockConfig">TimelockConfig</a> {
            interval_microseconds: 5 * 1000000,  // 5 seconds (was 3600 * 1000000 = 1 hour)
        });
    }
}
</code></pre>



</details>

<a id="0x1_timelock_config_set_interval_for_testing"></a>

## Function `set_interval_for_testing`

Set interval for testing (devnet/testnet only).

This function allows overriding the default interval on test networks
to speed up testing (e.g., 5 seconds instead of 1 hour).


<a id="@Security_1"></a>

### Security

This function is blocked on mainnet (chain_id == 1) to prevent
production misconfigurations.


<a id="@Arguments_2"></a>

### Arguments

- framework: Must be @aptos_framework signer
- interval_us: New interval in microseconds


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock_config.md#0x1_timelock_config_set_interval_for_testing">set_interval_for_testing</a>(_framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, interval_us: u64)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="timelock_config.md#0x1_timelock_config_set_interval_for_testing">set_interval_for_testing</a>(
    _framework: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>,
    interval_us: u64
) <b>acquires</b> <a href="timelock_config.md#0x1_timelock_config_TimelockConfig">TimelockConfig</a> {
    // PREVIOUSLY: <a href="system_addresses.md#0x1_system_addresses_assert_aptos_framework">system_addresses::assert_aptos_framework</a>(framework);
    // Allow <a href="../../aptos-stdlib/doc/any.md#0x1_any">any</a> <a href="account.md#0x1_account">account</a> <b>to</b> set this in testnet for ease of testing (e.g. mint <a href="account.md#0x1_account">account</a>)

    // Prevent production override - mainnet <b>has</b> <a href="chain_id.md#0x1_chain_id">chain_id</a> == 1
    <b>let</b> current_chain_id = <a href="chain_id.md#0x1_chain_id_get">chain_id::get</a>();
    <b>assert</b>!(
        current_chain_id != 1,
        <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_permission_denied">error::permission_denied</a>(<a href="timelock_config.md#0x1_timelock_config_EPRODUCTION_OVERRIDE_FORBIDDEN">EPRODUCTION_OVERRIDE_FORBIDDEN</a>)
    );

    // Update the config at @aptos_framework
    // We <b>assume</b> it <b>exists</b> (initialized by <a href="genesis.md#0x1_genesis">genesis</a>)
    <b>if</b> (<b>exists</b>&lt;<a href="timelock_config.md#0x1_timelock_config_TimelockConfig">TimelockConfig</a>&gt;(@aptos_framework)) {
        <b>let</b> config = <b>borrow_global_mut</b>&lt;<a href="timelock_config.md#0x1_timelock_config_TimelockConfig">TimelockConfig</a>&gt;(@aptos_framework);
        config.interval_microseconds = interval_us;
    } <b>else</b> {
        // Should not happen <b>if</b> initialized correctly
        <b>abort</b> <a href="../../aptos-stdlib/../move-stdlib/doc/error.md#0x1_error_not_found">error::not_found</a>(<a href="timelock_config.md#0x1_timelock_config_ETIMELOCK_CONFIG_NOT_FOUND">ETIMELOCK_CONFIG_NOT_FOUND</a>)
    }
}
</code></pre>



</details>

<a id="0x1_timelock_config_get_interval_microseconds"></a>

## Function `get_interval_microseconds`

Get the current interval duration in microseconds. Returns the configured interval, or the default (5 seconds) if not initialized. Used by the timelock module to determine rotation timing.

NOTE: Default is 5 seconds for testing/development.
TODO: Change to 1 hour for production mainnet deployment.


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="timelock_config.md#0x1_timelock_config_get_interval_microseconds">get_interval_microseconds</a>(): u64
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="timelock_config.md#0x1_timelock_config_get_interval_microseconds">get_interval_microseconds</a>(): u64 <b>acquires</b> <a href="timelock_config.md#0x1_timelock_config_TimelockConfig">TimelockConfig</a> {
    <b>if</b> (!<b>exists</b>&lt;<a href="timelock_config.md#0x1_timelock_config_TimelockConfig">TimelockConfig</a>&gt;(@aptos_framework)) {
        <b>return</b> 5 * 1000000  // 5 seconds (was 3600 * 1000000 = 1 hour)
    };
    <b>borrow_global</b>&lt;<a href="timelock_config.md#0x1_timelock_config_TimelockConfig">TimelockConfig</a>&gt;(@aptos_framework).interval_microseconds
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
