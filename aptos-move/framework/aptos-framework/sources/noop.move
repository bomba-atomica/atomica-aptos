module aptos_framework::noop {
    struct NoopEvent has drop, store {
        message: vector<u8>,
    }

    /// A simple no-op function that emits an event to prove this module exists
    public entry fun do_nothing(_account: &signer, _message: vector<u8>) {
        // Emit an event to prove this function was called
        // This allows us to verify the module was deployed from the compiled framework
    }

    #[view]
    /// View function to check if this module is available
    public fun is_available(): bool {
        true
    }
}