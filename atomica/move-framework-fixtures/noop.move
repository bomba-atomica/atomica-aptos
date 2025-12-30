module aptos_framework::noop {
    /// View function to check if this module is available
    #[view]
    public fun is_available(): bool {
        true
    }
}