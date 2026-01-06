spec aptos_framework::timelock {
    spec module {

        pragma verify = true;
        pragma aborts_if_is_strict;
    }

    /// Helper to get the TimelockState resource
    spec fun spec_timelock_state(): TimelockState {
        borrow_global<TimelockState>(@aptos_framework)
    }

    /// Invariant: current_interval is always non-negative (implied by u64, but useful anchor)
    /// Real invariant: last_rotation_time is never in the future relative to environment time
    invariant exists<TimelockState>(@aptos_framework) ==> 
        spec_timelock_state().last_rotation_time <= aptos_framework::timestamp::spec_now_microseconds();

    spec initialize {
        let addr = std::signer::address_of(framework);
        aborts_if !system_addresses::is_aptos_framework_address(addr);
        aborts_if exists<TimelockState>(addr);
        ensures exists<TimelockState>(addr);
    }

    spec on_new_block {
        // Can abort if no TimelockState (though code handles graceful return if missing?)
        // Code check: `if (!exists<TimelockState>(@aptos_framework)) { return };`
        // So it shouldn't abort on missing resource.
        
        let addr = std::signer::address_of(vm);
        aborts_if addr != @vm_reserved; // system_addresses::assert_vm(vm)
        
        // Complex logic around time and table operations
        // For PoC verification, we focus on safety vs availability
    }

    spec publish_master_public_key {
        // TODO: access control spec
        // pragma verify = false; // Until access control is implemented
    }

    spec publish_decryption_key_share {
        // pragma verify = false;
    }
}
