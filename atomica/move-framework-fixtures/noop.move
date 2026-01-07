module aptos_framework::noop {
    #[view]
    /// View function to check if this module is available
    public fun is_available(): bool {
        true
    }
}