
<a id="0x1_noop"></a>

# Module `0x1::noop`



-  [Struct `NoopEvent`](#0x1_noop_NoopEvent)
-  [Function `do_nothing`](#0x1_noop_do_nothing)
-  [Function `is_available`](#0x1_noop_is_available)


<pre><code></code></pre>



<a id="0x1_noop_NoopEvent"></a>

## Struct `NoopEvent`



<pre><code><b>struct</b> <a href="noop.md#0x1_noop_NoopEvent">NoopEvent</a> <b>has</b> drop, store
</code></pre>



<details>
<summary>Fields</summary>


<dl>
<dt>
<code>message: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;</code>
</dt>
<dd>

</dd>
</dl>


</details>

<a id="0x1_noop_do_nothing"></a>

## Function `do_nothing`

A simple no-op function that emits an event to prove this module exists


<pre><code><b>public</b> entry <b>fun</b> <a href="noop.md#0x1_noop_do_nothing">do_nothing</a>(_account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _message: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;)
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> entry <b>fun</b> <a href="noop.md#0x1_noop_do_nothing">do_nothing</a>(_account: &<a href="../../aptos-stdlib/../move-stdlib/doc/signer.md#0x1_signer">signer</a>, _message: <a href="../../aptos-stdlib/../move-stdlib/doc/vector.md#0x1_vector">vector</a>&lt;u8&gt;) {
    // Emit an <a href="event.md#0x1_event">event</a> <b>to</b> prove this function was called
    // This allows us <b>to</b> verify the <b>module</b> was deployed from the compiled framework
}
</code></pre>



</details>

<a id="0x1_noop_is_available"></a>

## Function `is_available`

View function to check if this module is available


<pre><code>#[view]
<b>public</b> <b>fun</b> <a href="noop.md#0x1_noop_is_available">is_available</a>(): bool
</code></pre>



<details>
<summary>Implementation</summary>


<pre><code><b>public</b> <b>fun</b> <a href="noop.md#0x1_noop_is_available">is_available</a>(): bool {
    <b>true</b>
}
</code></pre>



</details>


[move-book]: https://aptos.dev/move/book/SUMMARY
