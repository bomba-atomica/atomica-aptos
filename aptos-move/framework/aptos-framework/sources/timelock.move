module aptos_framework::timelock {
    use std::option::{Self, Option};
    use std::vector;
    use std::string::{Self, String};
    use aptos_std::table::{Self, Table};
    use aptos_framework::event::emit;
    use aptos_framework::timestamp;
    use aptos_framework::system_addresses;
    use aptos_framework::stake;
    use aptos_std::crypto_algebra::{zero, add, serialize, deserialize};
    use aptos_std::bls12381_algebra::{G1, FormatG1Compr};
    use aptos_std::aptos_hash::keccak256;
    
    // Dependencies
    use aptos_framework::threshold_dsa;
    use aptos_framework::ibe_signature; 

    friend aptos_framework::block;
    friend aptos_framework::genesis;

    /// # Atomica Timelock Service (Registry Model)
    ///
    /// Manages the registration of timelocks and the revelation of decryption keys
    /// based on timestamps (deadlines).
    ///
    /// ## Flow
    /// 1. User calls `register(deadline)`.
    /// 2. When `now >= deadline`, `DeadlineReachedEvent` is emitted.
    /// 3. Validators submit decryption key shares for the specific `timelock_id`.
    /// 4. Decryption key is aggregated and published.

    const ETIMELOCK_NOT_INITIALIZED: u64 = 1;
    const ENOT_VALIDATOR: u64 = 2;
    const EINVALID_SHARE: u64 = 3;
    const EDEADLINE_NOT_PASSED: u64 = 4;
    const EINVALID_TIMESTAMP: u64 = 5;
    const ESHARE_VERIFICATION_FAILED: u64 = 6;
    
    const MPK_ID: u64 = 1; // Canonical ID for the Timelock Service MPK

    struct DecryptionKeyShare has store, drop {
        validator: address,
        share: vector<u8>,
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

    /// Emitted to trigger DKG Setup for MPK (Legacy name preserved for Rust compat)
    #[event]
    struct StartKeyGenEvent has drop, store {
        interval: u64, // Acts as ID (should be MPK_ID = 1)
        config: TimelockConfig,
    }

    #[event]
    struct TimelockRegisteredEvent has drop, store {
        timelock_id: u64,
        deadline: u64,
    }

    #[event]
    struct DeadlineReachedEvent has drop, store {
        deadline: u64,
        timelock_ids: vector<u64>,
    }

    #[event]
    struct DecryptionKeyRevealedEvent has drop, store {
        timelock_id: u64,
        deadline: u64,
        decryption_key: vector<u8>,
    }

    /// Initialize the system
    public(friend) fun initialize(framework: &signer) {
        system_addresses::assert_aptos_framework(framework);
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] initialize called"));
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
                interval: MPK_ID,
                config: TimelockConfig { threshold, total_validators: n },
            });
            aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] Emitted StartKeyGenEvent in initialize"));
        }
    }

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

    /// On New Block: Check for passed deadlines
    public(friend) fun on_new_block(vm: &signer) acquires TimelockState {
        system_addresses::assert_vm(vm);
        if (!exists<TimelockState>(@aptos_framework)) return;

        let state = borrow_global_mut<TimelockState>(@aptos_framework);
        let now = timestamp::now_microseconds();

        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] on_new_block called, mpk_dkg_started="));
        aptos_std::debug::print(&state.mpk_dkg_started);

        // One-time DKG trigger if missed during genesis
        if (!state.mpk_dkg_started) {
            let validators = stake::cur_validator_consensus_infos();
            let n = vector::length(&validators);
            if (n > 0) {
                let threshold = (n * 2 / 3) + 1;
                emit(StartKeyGenEvent {
                    interval: MPK_ID,
                    config: TimelockConfig { threshold, total_validators: n },
                });
                state.mpk_dkg_started = true;
                aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] Emitted StartKeyGenEvent in on_new_block"));
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
                emit(DeadlineReachedEvent {
                    deadline: next_deadline,
                    timelock_ids: *ids,
                });
            };
        };
    }

    /// Submit a decryption key share
    public entry fun publish_public_key(validator: &signer, timelock_id: u64, mpk: vector<u8>) {
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] publish_public_key called, timelock_id="));
        aptos_std::debug::print(&timelock_id);
        threshold_dsa::publish_master_public_key(validator, timelock_id, mpk);
    }
    
    public entry fun publish_decryption_key_share(
        validator: &signer,
        timelock_id: u64,
        share: vector<u8>
    ) acquires TimelockState {
        aptos_std::debug::print(&std::string::utf8(b"[TIMELOCK] publish_decryption_key_share called, timelock_id="));
        aptos_std::debug::print(&timelock_id);
        let validator_addr = std::signer::address_of(validator);
        assert!(stake::is_current_epoch_validator(validator_addr), ENOT_VALIDATOR);

        let state = borrow_global_mut<TimelockState>(@aptos_framework);

        // 1. Verify Deadline Passed
        assert!(table::contains(&state.id_to_deadline, timelock_id), EINVALID_TIMESTAMP);
        let deadline = *table::borrow(&state.id_to_deadline, timelock_id);
        let now = timestamp::now_microseconds();
        
        // Allow slightly early submission? No, must strict.
        assert!(now >= deadline, EDEADLINE_NOT_PASSED);

        // Deduplicate
        if (table::contains(&state.decryption_keys, timelock_id)) return; // Already revealed

        // 2. Compute Identity
        // Format: "timelock_id:{id}:deadline_timestamp_microseconds:{deadline}"
        let identity = compute_identity(timelock_id, deadline);
        
        // 3. Verify Share
        let is_valid = ibe_signature::verify_private_key(MPK_ID, identity, share);
        assert!(is_valid, ESHARE_VERIFICATION_FAILED);

        // 4. Store & Aggregate
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

        vector::push_back(shares, DecryptionKeyShare { validator: validator_addr, share });

        // Check threshold
        let voters = stake::cur_validator_consensus_infos();
        let n = vector::length(&voters);
        let threshold = (n * 2 / 3) + 1;
        if (n == 0) { threshold = 1; };

        if (vector::length(shares) >= threshold) {
            // Aggregate
            let sum = zero<G1>();
            let count = 0;
            let i = 0;
            let len = vector::length(shares);
            
            while (i < len && count < threshold) {
                let s = &vector::borrow(shares, i).share;
                let elem_opt = deserialize<G1, FormatG1Compr>(s);
                if (std::option::is_some(&elem_opt)) {
                    let elem = std::option::extract(&mut elem_opt);
                    sum = add(&sum, &elem);
                    count = count + 1;
                };
                i = i + 1;
            };

            let key_bytes = serialize<G1, FormatG1Compr>(&sum);
            table::add(&mut state.decryption_keys, timelock_id, key_bytes);
            
            emit(DecryptionKeyRevealedEvent {
                timelock_id,
                deadline,
                decryption_key: key_bytes,
            });
        };
    }

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
            vector::push_back(&mut buf, digit + 48); // '0' is 48
            value = value / 10;
        };
        vector::reverse(&mut buf);
        string::utf8(buf)
    }

    // View Functions

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

    #[test_only]
    use aptos_framework::account::{create_signer_for_test, create_account_for_test};

    #[test(framework = @aptos_framework)]
    public fun test_flow(framework: &signer) acquires TimelockState {
        timestamp::set_time_has_started_for_testing(framework);
        create_account_for_test(@aptos_framework);
        stake::initialize_for_test(framework);
        initialize(framework);
        let vm = create_signer_for_test(@0x0);

        timestamp::update_global_time_for_test(100);
        register(200); // ID 0
        
        timestamp::update_global_time_for_test(201);
        on_new_block(&vm);
        // IDs for deadline 200 should be emitted
        // Logic check: pending_deadlines had [200], now empty.
    }
}
