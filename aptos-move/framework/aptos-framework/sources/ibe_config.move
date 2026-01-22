/// IBE (Identity-Based Encryption) configuration and Timelock Registry module.
///
/// This module implements the on-chain components for Atomica's timelock encryption system.
///
/// ## Core Types
///
/// - `IBEPublicParams`: Stores the Master Public Key (MPK) from DKG
/// - `TimelockRegistry`: Manages all registered timelocks
/// - `TimelockInfo`: Per-timelock state including shares and DK
///
/// ## Workflow
///
/// 1. Registration - User calls `register_timelock(deadline_us)` → gets `timelock_id`
/// 2. Encryption - Client queries MPK and identity, encrypts with IBE
/// 3. DKG - Validators run DKG, produce shares, publish MPK
/// 4. Reveal - After deadline, validators submit scalar shares
/// 5. Reconstruction - Native function reconstructs DK when threshold met
/// 6. Decryption - Anyone queries DK, decrypts ciphertext
///
/// ## Data Format
///
/// The native function `ibe::reconstruct_ibe_dk` accepts:
/// - `validator_indices`: `vector<u64>` - Validator indices (0-based)
/// - `scalar_shares`: `vector<vector<u8>>` - 32-byte little-endian scalars
/// - `weights`: `vector<u64>` - Full weights for ALL validators
/// - `total_weight`: `u64` - Sum of all weights
/// - `identity`: `vector<u8>` - 32-byte IBE identity
///
/// Returns 48-byte compressed G1 (the reconstructed DK).
///
/// ## Error Codes
///
/// | Code | Description |
/// |------|-------------|
/// | 1 | MPK must be 96 bytes (G2 compressed) |
/// | 2 | MPK not yet set by DKG |
/// | 3 | Deadline has not passed |
/// | 4 | Timelock ID not registered |
/// | 5 | DK not yet revealed |
/// | 6 | Threshold must be positive |
/// | 7 | Validator already submitted share |
module aptos_framework::ibe_config {
    use std::vector;
    use std::hash::sha3_256;
    use std::bcs;
    use std::signer;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::event;
    use aptos_framework::account;
    use aptos_framework::stake;
    use aptos_std::table::{Self, Table};
    use aptos_std::bls12381_algebra::G1;
    use aptos_std::ibe;

    friend aptos_framework::reconfiguration_with_dkg;
    friend aptos_framework::block;

    const E_INVALID_MPK_LENGTH: u64 = 1;
    const E_IBE_NOT_READY: u64 = 2;
    const E_DEADLINE_NOT_PASSED: u64 = 3;
    const E_TIMELOCK_NOT_FOUND: u64 = 4;
    const E_DECRYPTION_KEY_NOT_REVEALED: u64 = 5;
    const E_INVALID_THRESHOLD: u64 = 6;
    const E_SHARE_ALREADY_SUBMITTED: u64 = 7;

    const G2_COMPRESSED_LENGTH: u64 = 96;
    const G1_LENGTH: u64 = 48;
    const DEFAULT_REVEAL_THRESHOLD_NUMERATOR: u64 = 2;
    const DEFAULT_REVEAL_THRESHOLD_DENOMINATOR: u64 = 3;

    struct IBEPublicParams has key {
        mpk: vector<u8>,
        epoch: u64,
    }

    struct TimelockInfo has store {
        timelock_id: u64,
        deadline_us: u64,
        identity: vector<u8>,
        decryption_key: vector<u8>,
        is_revealed: bool,
        share_count: u64,
        reveal_threshold: u64,
        validator_indices: vector<u64>,
        /// Nested shares: submitted_shares[validator_index][virtual_player_share]
        submitted_shares: vector<vector<vector<u8>>>,
        validator_weights: vector<u64>,
        submitters: vector<address>,
    }

    struct TimelockRegistry has key {
        timelocks: Table<u64, TimelockInfo>,
        pending_timelock_ids: vector<u64>,
        next_timelock_id: u64,
        registration_events: event::EventHandle<TimelockRegistrationEvent>,
        reveal_events: event::EventHandle<TimelockRevealEvent>,
        expired_events: event::EventHandle<TimelockExpiredEvent>,
    }

    struct TimelockRegistrationEvent has drop, store {
        timelock_id: u64,
        deadline_us: u64,
        sender: address,
        timestamp_us: u64,
    }

    struct TimelockRevealEvent has drop, store {
        timelock_id: u64,
        timestamp_us: u64,
    }

    struct TimelockExpiredEvent has drop, store {
        timelock_id: u64,
        timestamp_us: u64,
    }

    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<IBEPublicParams>(@aptos_framework)) {
            move_to(aptos_framework, IBEPublicParams {
                mpk: vector::empty(),
                epoch: 0,
            });
        };
    }

    public(friend) fun set_mpk(mpk: vector<u8>, epoch: u64) acquires IBEPublicParams {
        assert!(
            vector::length(&mpk) == G2_COMPRESSED_LENGTH,
            E_INVALID_MPK_LENGTH
        );
        let params = borrow_global_mut<IBEPublicParams>(@aptos_framework);
        params.mpk = mpk;
        params.epoch = epoch;
    }

    #[view]
    public fun get_mpk(): vector<u8> acquires IBEPublicParams {
        if (!exists<IBEPublicParams>(@aptos_framework)) {
            return vector::empty()
        };
        borrow_global<IBEPublicParams>(@aptos_framework).mpk
    }

    #[view]
    public fun get_epoch(): u64 acquires IBEPublicParams {
        if (!exists<IBEPublicParams>(@aptos_framework)) {
            return 0
        };
        borrow_global<IBEPublicParams>(@aptos_framework).epoch
    }

    #[view]
    public fun is_ready(): bool acquires IBEPublicParams {
        if (!exists<IBEPublicParams>(@aptos_framework)) {
            return false
        };
        vector::length(&borrow_global<IBEPublicParams>(@aptos_framework).mpk) == G2_COMPRESSED_LENGTH
    }

    public fun initialize_timelock_registry(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<TimelockRegistry>(@aptos_framework)) {
            move_to(aptos_framework, TimelockRegistry {
                timelocks: table::new(),
                pending_timelock_ids: vector::empty<u64>(),
                next_timelock_id: 0,
                registration_events: account::new_event_handle<TimelockRegistrationEvent>(aptos_framework),
                reveal_events: account::new_event_handle<TimelockRevealEvent>(aptos_framework),
                expired_events: account::new_event_handle<TimelockExpiredEvent>(aptos_framework),
            });
        }
    }

    public entry fun register_timelock(
        account: &signer,
        deadline_us: u64
    ) acquires TimelockRegistry {
        let current_time = timestamp::now_microseconds();
        assert!(current_time < deadline_us, E_DEADLINE_NOT_PASSED);

        if (!exists<TimelockRegistry>(@aptos_framework)) {
            initialize_timelock_registry(account);
        };

        let registry = borrow_global_mut<TimelockRegistry>(@aptos_framework);
        let timelock_id = registry.next_timelock_id;
        registry.next_timelock_id = timelock_id + 1;

        let identity = compute_identity(timelock_id, deadline_us);

        let timelock_info = TimelockInfo {
            timelock_id,
            deadline_us,
            identity,
            decryption_key: vector::empty(),
            is_revealed: false,
            share_count: 0,
            reveal_threshold: 0,
            validator_indices: vector::empty(),
            submitted_shares: vector::empty(),
            validator_weights: vector::empty(),
            submitters: vector::empty(),
        };

        table::add(&mut registry.timelocks, timelock_id, timelock_info);
        vector::push_back(&mut registry.pending_timelock_ids, timelock_id);

        event::emit_event(&mut registry.registration_events, TimelockRegistrationEvent {
            timelock_id,
            deadline_us,
            sender: signer::address_of(account),
            timestamp_us: current_time,
        });
    }

    public(friend) fun submit_dk_shares(
        timelock_id: u64,
        shares: vector<vector<u8>>,
        validator_address: address,
        weight: u64,
        total_weight: u64
    ) acquires TimelockRegistry {
        let num_shares = vector::length(&shares);
        assert!(num_shares == weight, E_INVALID_THRESHOLD); // Should be exactly 'weight' shares

        let registry = borrow_global_mut<TimelockRegistry>(@aptos_framework);
        let timelock_info = table::borrow_mut(&mut registry.timelocks, timelock_id);

        let current_time = timestamp::now_microseconds();
        assert!(current_time >= timelock_info.deadline_us, E_DEADLINE_NOT_PASSED);
        assert!(!timelock_info.is_revealed, E_DECRYPTION_KEY_NOT_REVEALED);
        assert!(!vector::contains(&timelock_info.submitters, &validator_address), E_SHARE_ALREADY_SUBMITTED);

        if (timelock_info.reveal_threshold == 0) {
            timelock_info.reveal_threshold = (total_weight * DEFAULT_REVEAL_THRESHOLD_NUMERATOR) / DEFAULT_REVEAL_THRESHOLD_DENOMINATOR + 1;
        };

        let validator_index = stake::get_validator_index(validator_address);
        vector::push_back(&mut timelock_info.validator_indices, validator_index);
        vector::push_back(&mut timelock_info.submitted_shares, shares);
        vector::push_back(&mut timelock_info.validator_weights, weight);
        vector::push_back(&mut timelock_info.submitters, validator_address);
        timelock_info.share_count = timelock_info.share_count + weight;

        if (timelock_info.share_count >= timelock_info.reveal_threshold) {
            reconstruct_and_store_dk(registry, timelock_id, total_weight);
        };
    }

    fun reconstruct_and_store_dk(
        registry: &mut TimelockRegistry,
        timelock_id: u64,
        total_weight: u64
    ) {
        let timelock_info = table::borrow_mut(&mut registry.timelocks, timelock_id);

        let reconstructed_dk = ibe::reconstruct_ibe_dk<G1>(
            timelock_info.validator_indices,
            timelock_info.submitted_shares,
            timelock_info.validator_weights,
            timelock_info.reveal_threshold,
            total_weight,
            timelock_info.identity
        );

        timelock_info.decryption_key = reconstructed_dk;
        timelock_info.is_revealed = true;
        remove_pending_timelock_id(registry, timelock_id);

        let current_time = timestamp::now_microseconds();
        event::emit_event(&mut registry.reveal_events, TimelockRevealEvent {
            timelock_id,
            timestamp_us: current_time,
        });
    }

    fun remove_pending_timelock_id(registry: &mut TimelockRegistry, timelock_id: u64) {
        let (found, index) = vector::index_of(&registry.pending_timelock_ids, &timelock_id);
        if (found) {
            vector::swap_remove(&mut registry.pending_timelock_ids, index);
        };
    }

    public(friend) fun on_new_block(vm: &signer) acquires TimelockRegistry {
        system_addresses::assert_vm(vm);

        if (!exists<TimelockRegistry>(@aptos_framework)) {
            return
        };

        let current_time = timestamp::now_microseconds();
        let registry = borrow_global_mut<TimelockRegistry>(@aptos_framework);

        let i = 0;
        let len = vector::length(&registry.pending_timelock_ids);
        while (i < len) {
            let timelock_id = *vector::borrow(&registry.pending_timelock_ids, i);
            let timelock_info = table::borrow(&registry.timelocks, timelock_id);

            if (current_time >= timelock_info.deadline_us) {
                event::emit_event(&mut registry.expired_events, TimelockExpiredEvent {
                    timelock_id,
                    timestamp_us: current_time,
                });
            };

            i = i + 1;
        };
    }

    #[view]
    public fun get_timelock(timelock_id: u64): (u64, vector<u8>, bool, u64) acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        assert!(table::contains(&registry.timelocks, timelock_id), E_TIMELOCK_NOT_FOUND);
        let info = table::borrow(&registry.timelocks, timelock_id);
        (info.deadline_us, info.identity, info.is_revealed, info.share_count)
    }

    #[view]
    public fun get_deadline(timelock_id: u64): u64 acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        assert!(table::contains(&registry.timelocks, timelock_id), E_TIMELOCK_NOT_FOUND);
        table::borrow(&registry.timelocks, timelock_id).deadline_us
    }

    #[view]
    public fun get_identity(timelock_id: u64): vector<u8> acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        assert!(table::contains(&registry.timelocks, timelock_id), E_TIMELOCK_NOT_FOUND);
        table::borrow(&registry.timelocks, timelock_id).identity
    }

    #[view]
    public fun get_decryption_key(timelock_id: u64): vector<u8> acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        assert!(table::contains(&registry.timelocks, timelock_id), E_TIMELOCK_NOT_FOUND);
        let info = table::borrow(&registry.timelocks, timelock_id);
        assert!(info.is_revealed, E_DECRYPTION_KEY_NOT_REVEALED);
        info.decryption_key
    }

    #[view]
    public fun is_revealed(timelock_id: u64): bool acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        if (!table::contains(&registry.timelocks, timelock_id)) {
            return false
        };
        table::borrow(&registry.timelocks, timelock_id).is_revealed
    }

    #[view]
    public fun is_expired(timelock_id: u64): bool acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let registry = borrow_global<TimelockRegistry>(@aptos_framework);
        if (!table::contains(&registry.timelocks, timelock_id)) {
            return false
        };
        timestamp::now_microseconds() >= table::borrow(&registry.timelocks, timelock_id).deadline_us
    }

    #[view]
    public fun get_next_timelock_id(): u64 acquires TimelockRegistry {
        borrow_global<TimelockRegistry>(@aptos_framework).next_timelock_id
    }

    /// Compute the IBE identity for a timelock.
    /// Identity = SHA3-256(timelock_id || deadline_us)
    fun compute_identity(timelock_id: u64, deadline_us: u64): vector<u8> {
        let input = bcs::to_bytes(&timelock_id);
        vector::append(&mut input, bcs::to_bytes(&deadline_us));
        sha3_256(input)
    }

    // ================================
    // Test-Only Helper Functions
    // ================================

    #[test_only]
    public fun initialize_for_testing(aptos_framework: &signer) {
        account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        initialize(aptos_framework);
        initialize_timelock_registry(aptos_framework);
    }

    #[test_only]
    public fun set_mpk_for_testing(mpk: vector<u8>, epoch: u64) acquires IBEPublicParams {
        let params = borrow_global_mut<IBEPublicParams>(@aptos_framework);
        params.mpk = mpk;
        params.epoch = epoch;
    }

    #[test_only]
    public fun on_new_block_for_testing(vm: &signer) acquires TimelockRegistry {
        on_new_block(vm);
    }

    #[test_only]
    public fun submit_dk_shares_for_testing(
        timelock_id: u64,
        shares: vector<vector<u8>>,
        validator_index: u64,
        validator_address: address,
        weight: u64,
        total_weight: u64
    ) acquires TimelockRegistry {
        let registry = borrow_global_mut<TimelockRegistry>(@aptos_framework);
        let timelock_info = table::borrow_mut(&mut registry.timelocks, timelock_id);

        let current_time = timestamp::now_microseconds();
        assert!(current_time >= timelock_info.deadline_us, E_DEADLINE_NOT_PASSED);
        assert!(!timelock_info.is_revealed, E_DECRYPTION_KEY_NOT_REVEALED);
        assert!(!vector::contains(&timelock_info.submitters, &validator_address), E_SHARE_ALREADY_SUBMITTED);

        if (timelock_info.reveal_threshold == 0) {
            timelock_info.reveal_threshold = (total_weight * DEFAULT_REVEAL_THRESHOLD_NUMERATOR) / DEFAULT_REVEAL_THRESHOLD_DENOMINATOR + 1;
        };

        vector::push_back(&mut timelock_info.validator_indices, validator_index);
        vector::push_back(&mut timelock_info.submitted_shares, shares);
        vector::push_back(&mut timelock_info.validator_weights, weight);
        vector::push_back(&mut timelock_info.submitters, validator_address);
        timelock_info.share_count = timelock_info.share_count + weight;

        if (timelock_info.share_count >= timelock_info.reveal_threshold) {
            reconstruct_and_store_dk(registry, timelock_id, total_weight);
        };
    }

    // ================================
    // Test Specifications
    // ================================
    //
    // The following specifications describe WHAT each test should verify.
    // Tests should be implemented at a higher level (Rust integration tests)
    // using real PVSS golden vectors and actual DKG output.
    //
    // === IBE Initialization Tests ===
    //
    // Test: initialize creates IBEPublicParams with empty MPK
    // - Verifies IBEPublicParams exists at framework address
    // - Verifies MPK is empty vector
    // - Verifies epoch is 0
    //
    // Test: set_mpk updates MPK and epoch
    // - Provides 96-byte MPK (simulated G2)
    // - Verifies MPK is stored correctly
    // - Verifies epoch is updated
    //
    // Test: is_ready returns false before MPK set
    // - Calls is_ready before set_mpk
    // - Verifies returns false
    //
    // Test: is_ready returns true after valid MPK set
    // - Sets 96-byte MPK
    // - Calls is_ready
    // - Verifies returns true
    //
    // Test: set_mpk rejects invalid length
    // - Attempts to set MPK with length != 96
    // - Verifies transaction aborts
    //
    // === Timelock Registration Tests ===
    //
    // Test: register_timelock creates new TimelockInfo
    // - Registers timelock with future deadline
    // - Verifies TimelockInfo exists with correct fields
    // - Verifies identity is computed correctly
    // - Verifies events are emitted
    //
    // Test: get_identity returns correct 32-byte hash
    // - Registers timelock
    // - Retrieves identity via get_identity
    // - Verifies identity is 32 bytes
    // - Verifies identity matches SHA3-256(timelock_id || deadline_us)
    //
    // Test: identity is unique across (timelock_id, deadline_us) pairs
    // - Registers multiple timelocks with different IDs or deadlines
    // - Verifies each identity is distinct
    //
    // Test: get_timelock returns correct state
    // - Registers timelock
    // - Calls get_timelock
    // - Verifies deadline, identity, is_revealed, share_count
    //
    // Test: get_deadline returns registered deadline
    // - Registers timelock
    // - Retrieves deadline
    // - Verifies matches input
    //
    // === DK Share Submission Tests ===
    //
    // Test: submit_dk_share stores share correctly
    // - Registers timelock and advances past deadline
    // - Submits share (48-byte G1)
    // - Verifies share is stored
    // - Verifies validator index is recorded
    // - Verifies weight is recorded
    // - Verifies share_count is incremented
    //
    // Test: submit_dk_share rejects duplicate validator
    // - Same validator submits twice
    // - Verifies transaction aborts
    //
    // Test: submit_dk_share rejects before deadline
    // - Attempts to submit before deadline
    // - Verifies transaction aborts
    //
    // Test: submit_dk_share rejects invalid share length
    // - Attempts to submit share with length != 48
    // - Verifies transaction aborts
    //
    // === DK Reconstruction Tests ===
    //
    // Test: reconstruction triggers at threshold
    // - Registers timelock
    // - Submits shares totaling >= threshold weight
    // - Verifies is_revealed becomes true
    // - Verifies decryption_key is populated
    // - Verifies DK matches expected value from golden vectors
    //
    // Test: reconstruction uses correct weights
    // - Uses validators with different weights (e.g., [2, 1, 2])
    // - Submits shares from subset of validators
    // - Verifies reconstruction succeeds when weight threshold met
    // - Verifies DK matches expected value
    //
    // Test: reconstruction works with sparse validator indices
    // - Network has 5 validators
    // - Only validators 0 and 2 submit shares
    // - Verifies reconstruction still works
    // - Verifies DK matches expected value
    //
    // Test: get_decryption_key returns populated key
    // - After threshold is reached
    // - Calls get_decryption_key
    // - Verifies returns 48-byte G1
    // - Verifies matches reconstructed DK
    //
    // Test: get_decryption_key aborts before reveal
    // - Before threshold is reached
    // - Calls get_decryption_key
    // - Verifies transaction aborts
    //
    // === Event Emission Tests ===
    //
    // Test: registration emits TimelockRegistrationEvent
    // - Registers timelock
    // - Verifies event contains correct timelock_id, deadline, sender
    //
    // Test: reveal emits TimelockRevealEvent
    // - After threshold is reached
    // - Verifies event contains correct timelock_id and timestamp
    //
    // Test: expiration emits TimelockExpiredEvent
    // - on_new_block called after deadline
    // - Verifies event is emitted
    //
    // === Integration Tests ===
    //
    // Test: full reveal flow with golden vectors
    // - Uses real PVSS golden vectors from atomica/golden_vectors/
    // - Registers timelock
    // - Submits shares from participating validators
    // - Verifies reconstruction produces expected DK
    // - Verifies DK correctly decrypts test ciphertext
    //
    // Test: identity computation matches golden vectors
    // - Computes identity for known (timelock_id, deadline_us) pairs
    // - Verifies matches golden vector expected values
    //
    // Test: multiple timelocks with different identities
    // - Registers multiple timelocks
    // - Verifies each has unique identity
    // - Verifies DK reconstruction works independently
}
