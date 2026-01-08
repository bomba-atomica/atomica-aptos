module aptos_framework::timelock {
    use std::option::{Self, Option};
    use std::vector;
    use std::string::{Self, String};
    use aptos_std::table::{Self, Table};
    use aptos_framework::event::emit;
    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;
    use aptos_framework::stake;
    use aptos_std::aptos_hash::keccak256;
    
    // Dependencies - delegating cryptographic operations to specialized modules
    use aptos_framework::threshold_dsa;

    friend aptos_framework::block;
    friend aptos_framework::genesis;

    /// # Atomica Timelock Service
    ///
    /// Manages the registration of timelocks and the revelation of decryption keys
    /// based on timestamps (deadlines).
    ///
    /// ## Architecture
    /// - **timelock.move**: Timelock registration, deadline management, state storage
    /// - **threshold_dsa.move**: DKG, MPK, threshold BLS verification and aggregation
    /// - **ibe_signature.move**: IBE identity-to-point mapping
    ///
    /// ## Flow
    /// 1. User calls `register(deadline)`.
    /// 2. When `now >= deadline`, `DeadlineReachedEvent` is emitted.
    /// 3. Validators submit decryption key shares for the specific `timelock_id`.
    /// 4. Decryption key is aggregated (via threshold_dsa) and published.

    const ETIMELOCK_NOT_INITIALIZED: u64 = 1;
    const ENOT_VALIDATOR: u64 = 2;
    const EINVALID_SHARE: u64 = 3;
    const EDEADLINE_NOT_PASSED: u64 = 4;
    const EINVALID_TIMESTAMP: u64 = 5;
    const ESHARE_VERIFICATION_FAILED: u64 = 6;
    
    const MPK_ID: u64 = 1; // Canonical ID for the Timelock Service MPK

    /// A decryption key share submitted by a validator
    /// (Shared struct - also used by threshold_dsa module)
    struct DecryptionKeyShare has store, drop {
        timelock_id: u64,
        validator: address,
        validator_idx: u64,  // Validator's index in the DKG participant set (for Lagrange weights)
        share: vector<u8>,   // G1 point: s_i × Q_id where s_i is validator's scalar share
    }

    struct TimelockConfig has copy, drop, store {
        threshold: u64,
        total_validators: u64,
    }

    struct TimelockState has key {
        /// Counter for assigning unique IDs
        next_timelock_id: u64,
        
        /// Sorted vector of pending deadlines (ascending)
        pending_deadlines: vector<u64>,
        
        /// Map from deadline -> List of Timelock IDs
        deadline_to_ids: Table<u64, vector<u64>>,
        
        /// Map from timelock_id -> Deadline (for verification)
        id_to_deadline: Table<u64, u64>,

        /// Store collected key shares: timelock_id -> shares
        shares: Table<u64, vector<DecryptionKeyShare>>,
        
        /// Store revealed keys: timelock_id -> key bytes
        decryption_keys: Table<u64, vector<u8>>,
        
        /// Flag to track if MPK DKG was started
        mpk_dkg_started: bool,
    }

    // Events

    #[event]
    struct StartKeyGenEvent has drop, store {
        epoch: u64, // Acts as ID (should be MPK_ID = 1)
        config: TimelockConfig,
    }

    #[event]
    struct TimelockRegisteredEvent has drop, store {
        timelock_id: u64,
        deadline: u64,
    }

    #[event]
    struct RequestRevealEvent has drop, store {
        deadline: u64,
        timelock_ids: vector<u64>,
    }

    #[event]
    struct SecretRevealedEvent has drop, store {
        timelock_id: u64,
        deadline: u64,
        secret: vector<u8>,
    }

    // =========================================================================
    // Initialization
    // =========================================================================

    /// Initialize the timelock system
    public(friend) fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        if (!exists<TimelockState>(@aptos_framework)) {
            // Ensure dependencies initialized
            threshold_dsa::initialize(framework);

            move_to(framework, TimelockState {
                next_timelock_id: 2, // Start after MPK_ID (1)
                pending_deadlines: vector::empty(),
                deadline_to_ids: table::new(),
                id_to_deadline: table::new(),
                shares: table::new(),
                decryption_keys: table::new(),
                mpk_dkg_started: false,
            });

            // Trigger MPK Setup
            let validators = stake::cur_validator_consensus_infos();
            let n = vector::length(&validators);
            let threshold = (n * 2 / 3) + 1;
            if (n == 0) { n = 1; threshold = 1; };

            emit(StartKeyGenEvent {
                epoch: MPK_ID,
                config: TimelockConfig { threshold, total_validators: n },
            });
        }
    }

    // =========================================================================
    // Timelock Registration
    // =========================================================================

    /// Register a new timelock request
    public entry fun register(deadline: u64) acquires TimelockState {
        let state = borrow_global_mut<TimelockState>(@aptos_framework);
        let id = state.next_timelock_id;
        state.next_timelock_id = id + 1;

        // Validation
        let now = timestamp::now_microseconds();
        assert!(deadline > now, EINVALID_TIMESTAMP);

        // Store mappings
        table::add(&mut state.id_to_deadline, id, deadline);
        
        if (!table::contains(&state.deadline_to_ids, deadline)) {
            table::add(&mut state.deadline_to_ids, deadline, vector::empty());
            // Add to pending deadlines (sorted insert)
            insert_pending_deadline(&mut state.pending_deadlines, deadline);
        };
        
        let ids = table::borrow_mut(&mut state.deadline_to_ids, deadline);
        vector::push_back(ids, id);

        emit(TimelockRegisteredEvent {
            timelock_id: id,
            deadline,
        });
    }

    /// Internal: Insert deadline into sorted vector
    fun insert_pending_deadline(deadlines: &mut vector<u64>, deadline: u64) {
        let len = vector::length(deadlines);
        if (len == 0) {
            vector::push_back(deadlines, deadline);
            return
        };
        // Optimization: Check if it belongs at the end (common case)
        if (deadline >= *vector::borrow(deadlines, len - 1)) {
            vector::push_back(deadlines, deadline);
            return
        };
        
        // Find insertion point
        let i = 0;
        while (i < len) {
            if (*vector::borrow(deadlines, i) > deadline) {
                vector::insert(deadlines, i, deadline);
                return
            };
            i = i + 1;
        };
        vector::push_back(deadlines, deadline);
    }

    // =========================================================================
    // Deadline Processing
    // =========================================================================

    /// On New Block: Check for passed deadlines
    public(friend) fun on_new_block(vm: &signer) acquires TimelockState {
        system_addresses::assert_vm(vm);
        if (!exists<TimelockState>(@aptos_framework)) return;

        let state = borrow_global_mut<TimelockState>(@aptos_framework);
        let now = timestamp::now_microseconds();

        // One-time DKG trigger if missed during genesis
        if (!state.mpk_dkg_started) {
            let validators = stake::cur_validator_consensus_infos();
            let n = vector::length(&validators);
            if (n > 0) {
                let threshold = (n * 2 / 3) + 1;
                emit(StartKeyGenEvent {
                    epoch: MPK_ID,
                    config: TimelockConfig { threshold, total_validators: n },
                });
                state.mpk_dkg_started = true;
            };
        };

        // Process pending deadlines <= now
        while (!vector::is_empty(&state.pending_deadlines)) {
            let next_deadline = *vector::borrow(&state.pending_deadlines, 0);
            
            if (next_deadline > now) {
                break // No more deadlines to process
            };

            // Remove from pending
            vector::remove(&mut state.pending_deadlines, 0);

            // Get IDs and emit event
            if (table::contains(&state.deadline_to_ids, next_deadline)) {
                let ids = table::borrow(&state.deadline_to_ids, next_deadline);
                emit(RequestRevealEvent {
                    deadline: next_deadline,
                    timelock_ids: *ids,
                });
            };
        };
    }

    // =========================================================================
    // Key Publication
    // =========================================================================

    /// Submit Master Public Key (delegates to threshold_dsa)
    public entry fun publish_public_key(validator: &signer, timelock_id: u64, mpk: vector<u8>) {
        threshold_dsa::publish_master_public_key(validator, timelock_id, mpk);
    }
    
    /// Submit a decryption key share for a timelock
    /// 
    /// This function handles:
    /// 1. Deadline verification
    /// 2. Validator authorization
    /// 3. Share collection and threshold checking
    /// 
    /// Cryptographic operations (verification, aggregation) are delegated to threshold_dsa
    public entry fun publish_decryption_key_share(
        validator: &signer,
        timelock_id: u64,
        validator_idx: u64,  // Validator's index in DKG participant set
        share: vector<u8>
    ) acquires TimelockState {
        let validator_addr = std::signer::address_of(validator);
        assert!(stake::is_current_epoch_validator(validator_addr), ENOT_VALIDATOR);

        let state = borrow_global_mut<TimelockState>(@aptos_framework);

        // 1. Verify Deadline Passed
        assert!(table::contains(&state.id_to_deadline, timelock_id), EINVALID_TIMESTAMP);
        let deadline = *table::borrow(&state.id_to_deadline, timelock_id);
        let now = timestamp::now_microseconds();
        assert!(now >= deadline, EDEADLINE_NOT_PASSED);

        // Deduplicate - already revealed
        if (table::contains(&state.decryption_keys, timelock_id)) return;
        
        // 2. Verify Share format (delegated to threshold_dsa module)
        let is_valid = threshold_dsa::verify_timelock_share(share);
        assert!(is_valid, ESHARE_VERIFICATION_FAILED);

        // 4. Store Share
        if (!table::contains(&state.shares, timelock_id)) {
            table::add(&mut state.shares, timelock_id, vector::empty());
        };
        let shares = table::borrow_mut(&mut state.shares, timelock_id);

        // Validator dedup
        let i = 0;
        let len = vector::length(shares);
        while (i < len) {
            if (vector::borrow(shares, i).validator == validator_addr) return;
            i = i + 1;
        };

        vector::push_back(shares, DecryptionKeyShare { validator: validator_addr, timelock_id, validator_idx, share });

        // 5. Check Threshold and Aggregate
        let voters = stake::cur_validator_consensus_infos();
        let n = vector::length(&voters);
        let threshold = (n * 2 / 3) + 1;
        if (n == 0) { threshold = 1; };

        if (vector::length(shares) >= threshold) {
            let share_bytes_list = vector::empty<vector<u8>>();
            let validator_indices = vector::empty<u64>();
            let i = 0;
            let len = vector::length(shares);
            while (i < len) {
                let s = vector::borrow(shares, i);
                vector::push_back(&mut share_bytes_list, s.share);
                vector::push_back(&mut validator_indices, s.validator_idx);
                i = i + 1;
            };
            let dk = threshold_dsa::aggregate_timelock_shares(&share_bytes_list, &validator_indices, n);
            
            if (vector::length(&dk) > 0) {
                table::add(&mut state.decryption_keys, timelock_id, dk);
                
                emit(SecretRevealedEvent {
                    timelock_id,
                    deadline,
                    secret: dk,
                });
            };
        };
    }

    // =========================================================================
    // Helper Functions
    // =========================================================================

    /// Construct canonical identity string and hash it
    fun compute_identity(timelock_id: u64, deadline: u64): vector<u8> {
        // "timelock_id:{id}:deadline_timestamp_microseconds:{deadline}"
        let str = string::utf8(b"timelock_id:");
        string::append(&mut str, u64_to_string(timelock_id));
        string::append(&mut str, string::utf8(b":deadline_timestamp_microseconds:"));
        string::append(&mut str, u64_to_string(deadline));
        
        keccak256(*string::bytes(&str))
    }

    fun u64_to_string(value: u64): String {
        if (value == 0) {
            return string::utf8(b"0")
        };
        let buf = vector::empty<u8>();
        while (value > 0) {
            let digit = ((value % 10) as u8);
            vector::push_back(&mut buf, digit + 48);
            value = value / 10;
        };
        vector::reverse(&mut buf);
        string::utf8(buf)
    }

    // =========================================================================
    // View Functions
    // =========================================================================

    #[view]
    public fun get_deadline(timelock_id: u64): Option<u64> acquires TimelockState {
        if (!exists<TimelockState>(@aptos_framework)) return option::none();
        let state = borrow_global<TimelockState>(@aptos_framework);
        if (table::contains(&state.id_to_deadline, timelock_id)) {
            option::some(*table::borrow(&state.id_to_deadline, timelock_id))
        } else {
            option::none()
        }
    }

    #[view]
    public fun get_decryption_key(timelock_id: u64): Option<vector<u8>> acquires TimelockState {
        if (!exists<TimelockState>(@aptos_framework)) return option::none();
        let state = borrow_global<TimelockState>(@aptos_framework);
        if (table::contains(&state.decryption_keys, timelock_id)) {
            option::some(*table::borrow(&state.decryption_keys, timelock_id))
        } else {
            option::none()
        }
    }

    // =========================================================================
    // Unit Tests
    // =========================================================================

    #[test_only]
    use aptos_framework::account::{create_signer_for_test, create_account_for_test};

    #[test]
    fun test_u64_to_string_zero() {
        let result = u64_to_string(0);
        assert!(std::string::bytes(&result) == &b"0", 1);
    }

    #[test]
    fun test_u64_to_string_single_digit() {
        let result = u64_to_string(5);
        assert!(std::string::bytes(&result) == &b"5", 1);
    }

    #[test]
    fun test_u64_to_string_multi_digit() {
        let result = u64_to_string(12345);
        assert!(std::string::bytes(&result) == &b"12345", 1);
    }

    #[test]
    fun test_compute_identity_format() {
        let identity = compute_identity(42, 1000000);
        assert!(vector::length(&identity) == 32, 1); // keccak256 output
    }

    #[test]
    fun test_compute_identity_deterministic() {
        let id1 = compute_identity(1, 1000);
        let id2 = compute_identity(1, 1000);
        let id3 = compute_identity(2, 1000);
        let id4 = compute_identity(1, 2000);
        
        assert!(id1 == id2, 1);
        assert!(id1 != id3, 2);
        assert!(id1 != id4, 3);
    }

    #[test(framework = @aptos_framework)]
    fun test_initialize(framework: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        
        assert!(!exists<TimelockState>(@aptos_framework), 1);
        initialize(framework);
        assert!(exists<TimelockState>(@aptos_framework), 2);
        
        let state = borrow_global<TimelockState>(@aptos_framework);
        assert!(state.next_timelock_id == 2, 3);
        assert!(vector::length(&state.pending_deadlines) == 0, 4);
    }

    #[test(framework = @aptos_framework)]
    fun test_register_single_timelock(framework: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        initialize(framework);
        
        let deadline = 1000000;
        register(deadline);
        
        let state = borrow_global<TimelockState>(@aptos_framework);
        assert!(table::contains(&state.id_to_deadline, 2), 1);
        assert!(*table::borrow(&state.id_to_deadline, 2) == deadline, 2);
        assert!(vector::length(&state.pending_deadlines) == 1, 3);
    }

    #[test(framework = @aptos_framework)]
    fun test_register_multiple_timelocks_different_deadlines(framework: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        initialize(framework);
        
        register(1000000);
        register(2000000);
        
        let state = borrow_global<TimelockState>(@aptos_framework);
        assert!(vector::length(&state.pending_deadlines) == 2, 1);
        assert!(*vector::borrow(&state.pending_deadlines, 0) == 1000000, 2);
        assert!(*vector::borrow(&state.pending_deadlines, 1) == 2000000, 3);
    }

    #[test(framework = @aptos_framework)]
    #[expected_failure(abort_code = 5, location = Self)]
    fun test_register_invalid_deadline_past(framework: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        initialize(framework);
        timestamp::update_global_time_for_test(1000000);
        register(999999);
    }

    #[test(framework = @aptos_framework)]
    fun test_on_new_block_triggers_deadline(framework: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        initialize(framework);
        
        let vm = create_signer_for_test(@0x0);
        register(1000);
        
        assert!(vector::length(&borrow_global<TimelockState>(@aptos_framework).pending_deadlines) == 1, 1);
        
        timestamp::update_global_time_for_test(1001);
        on_new_block(&vm);
        
        let state = borrow_global<TimelockState>(@aptos_framework);
        assert!(vector::length(&state.pending_deadlines) == 0, 2);
    }

    #[test(framework = @aptos_framework)]
    fun test_get_deadline_not_exists(framework: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        initialize(framework);
        
        let result = get_deadline(999);
        assert!(option::is_none(&result), 1);
    }

    #[test(framework = @aptos_framework)]
    fun test_get_deadline_exists(framework: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        initialize(framework);
        
        register(1000000);
        let result = get_deadline(2);
        assert!(option::is_some(&result), 1);
        assert!(*option::borrow(&result) == 1000000, 2);
    }

    #[test(framework = @aptos_framework)]
    fun test_full_registration_flow(framework: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        initialize(framework);
        
        let vm = create_signer_for_test(@0x0);
        
        let deadline = 1000000;
        register(deadline);
        assert!(get_deadline(2) == option::some(deadline), 1);
        assert!(get_decryption_key(2) == option::none(), 2);
        
        timestamp::update_global_time_for_test(deadline + 1);
        on_new_block(&vm);
        
        let state = borrow_global<TimelockState>(@aptos_framework);
        assert!(vector::length(&state.pending_deadlines) == 0, 3);
        assert!(get_decryption_key(2) == option::none(), 4); // No shares yet
    }

    #[test(framework = @aptos_framework)]
    fun test_register_deadline_sorted_insertion(framework: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        initialize(framework);
        
        register(3000);
        register(1000);
        register(2000);
        
        let state = borrow_global<TimelockState>(@aptos_framework);
        let deadlines = &state.pending_deadlines;
        assert!(vector::length(deadlines) == 3, 1);
        assert!(*vector::borrow(deadlines, 0) == 1000, 2);
        assert!(*vector::borrow(deadlines, 1) == 2000, 3);
        assert!(*vector::borrow(deadlines, 2) == 3000, 4);
    }
}
