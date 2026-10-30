/// IBE configuration and timelock registry.
///
/// On-chain part of Atomica's timelock encryption. Design:
/// `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md` (sections 4, 7 and 8).
///
/// ## Types
///
/// - `IBEPublicParams`: the master public key (MPK) and the epoch it belongs to.
/// - `TimelockEpochConfig`: written once per epoch from the timelock chunky DKG result. Holds the MPK,
///   the reconstruction threshold, the per-validator weights and the epoch interval.
/// - `TimelockRegistry`: all registered timelocks plus the queue of pending ones.
/// - `TimelockInfo`: per-timelock state: epoch, deadline, status, shares and the revealed key.
///
/// ## Lifecycle of one timelock
///
/// 1. `register_timelock(deadline_us)` records the epoch and the identity. It aborts unless the MPK of the
///    current epoch is ready and the deadline leaves room for the reveal window before the epoch ends.
/// 2. After the deadline the validator transaction handler submits verified key shares (`submit_dk_shares`).
///    The share check happens in Rust before the share reaches this module.
/// 3. When verified weight reaches the threshold the handler passes the reconstructed key to
///    `finalize_timelock_reveal`. The key is checked again here with `ibe::verify_decryption_key`.
/// 4. Status moves one way: `Pending` to `Revealed` or `Expired(reason)`. It never reopens.
///
/// ## Epoch rule
///
/// A timelock lives in one epoch. A `Pending` timelock of an earlier epoch reads as `Expired(epoch-ended)`
/// from the first block of the new epoch, because every view and every guard compares `info.epoch` with the
/// current epoch. `on_new_block` then writes the status and drops the stored shares, in bounded batches.
///
/// ## Identity
///
/// `compute_identity` returns
/// `SHA3-256("ATOMICA_TIMELOCK_IBE_V2_ID" || chain_id(u8) || epoch(u64 LE) || timelock_id(u64 LE) || deadline_us(u64 LE))`.
module aptos_framework::ibe_config {
    use std::bcs;
    use std::hash::sha3_256;
    use std::signer;
    use std::vector;
    use aptos_std::ibe;
    use aptos_std::table::{Self, Table};
    use aptos_framework::big_ordered_map::{Self, BigOrderedMap};
    use aptos_framework::chain_id;
    use aptos_framework::event;
    use aptos_framework::reconfiguration;
    use aptos_framework::stake;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;

    friend aptos_framework::reconfiguration_with_dkg;
    friend aptos_framework::block;

    // ================================
    // Errors
    // ================================

    /// MPK must be 96 bytes (compressed G2).
    const E_INVALID_MPK_LENGTH: u64 = 1;
    /// No MPK for the current epoch.
    const E_IBE_NOT_READY: u64 = 2;
    /// The deadline has not passed yet (share submission) .
    const E_DEADLINE_NOT_PASSED: u64 = 3;
    /// Timelock id is not registered.
    const E_TIMELOCK_NOT_FOUND: u64 = 4;
    /// The key is not revealed.
    const E_DECRYPTION_KEY_NOT_REVEALED: u64 = 5;
    /// The share count differs from the validator's weight, or the threshold is invalid.
    const E_INVALID_THRESHOLD: u64 = 6;
    /// Validator already has an accepted share set for this timelock.
    const E_SHARE_ALREADY_SUBMITTED: u64 = 7;
    /// The deadline is not in the future.
    const E_DEADLINE_IN_PAST: u64 = 8;
    /// The deadline is later than the epoch end minus the guard.
    const E_DEADLINE_AFTER_EPOCH_END: u64 = 9;
    /// The share, the timelock or the config belongs to another epoch.
    const E_WRONG_EPOCH: u64 = 10;
    /// The timelock is not `Pending`.
    const E_NOT_PENDING: u64 = 11;
    /// Verified weight is below the reconstruction threshold.
    const E_BELOW_THRESHOLD: u64 = 12;
    /// The registry was not initialized at genesis.
    const E_REGISTRY_NOT_INITIALIZED: u64 = 13;
    /// The validator index is not in the epoch config.
    const E_UNKNOWN_VALIDATOR_INDEX: u64 = 14;
    /// The reveal window has elapsed.
    const E_REVEAL_WINDOW_ELAPSED: u64 = 15;
    /// The epoch config is invalid (empty weights, zero threshold, threshold above total weight).
    const E_INVALID_EPOCH_CONFIG: u64 = 16;

    // ================================
    // Constants
    // ================================

    const G2_COMPRESSED_LENGTH: u64 = 96;
    const G1_LENGTH: u64 = 48;

    const MICROS_PER_MINUTE: u64 = 60_000_000;
    /// A deadline must be at least this long before the epoch end (spec section 8, rule 2).
    const EPOCH_END_GUARD_US: u64 = 900_000_000;
    /// The reveal window after the deadline (spec section 7, rule 3).
    const REVEAL_WINDOW_US: u64 = 600_000_000;
    /// Upper bound of timelocks expired per block (spec section 7, rule 6). Initial value, tune by measurement.
    const MAX_EXPIRE_PER_BLOCK: u64 = 50;

    /// Status codes returned by `get_status`.
    const STATUS_PENDING: u8 = 0;
    const STATUS_REVEALED: u8 = 1;
    const STATUS_EXPIRED: u8 = 2;

    /// Expiry reasons returned by `get_expiry_reason`. `0` means no expiry reason.
    const REASON_NONE: u8 = 0;
    const REASON_EPOCH_ENDED: u8 = 1;
    const REASON_REVEAL_WINDOW_ELAPSED: u8 = 2;
    const REASON_KEY_CHECK_FAILED: u8 = 3;

    const IDENTITY_TAG: vector<u8> = b"ATOMICA_TIMELOCK_IBE_V2_ID";

    // ================================
    // State
    // ================================

    struct IBEPublicParams has key {
        mpk: vector<u8>,
        epoch: u64
    }

    /// Written once per epoch, after the epoch change, from the timelock chunky DKG result.
    struct TimelockEpochConfig has key, drop {
        epoch: u64,
        mpk: vector<u8>,
        /// Weight needed to reconstruct. Comes from the DKG, not from the vtxn author.
        reconstruct_threshold_weight: u64,
        total_weight: u64,
        /// Per-validator weights from the DKG rounding profile, indexed by validator index.
        weights: vector<u64>,
        /// Length of the epoch. The epoch ends at `last_reconfiguration_time + epoch_interval_us`.
        epoch_interval_us: u64,
        public_parameters_hash: vector<u8>,
        subtranscript_hash: vector<u8>
    }

    enum TimelockStatus has copy, drop, store {
        Pending,
        Revealed,
        Expired { reason: u8 }
    }

    struct TimelockInfo has store {
        timelock_id: u64,
        /// Epoch the timelock was registered in.
        epoch: u64,
        deadline_us: u64,
        /// `min(deadline_us + 10 minutes, epoch end)`.
        reveal_expiry_us: u64,
        identity: vector<u8>,
        decryption_key: vector<u8>,
        status: TimelockStatus,
        /// Accepted share weight.
        share_count: u64,
        /// Reconstruction threshold weight, copied from the epoch config at registration.
        reveal_threshold: u64,
        validator_indices: vector<u64>,
        /// `submitted_shares[i]` holds the shares of `validator_indices[i]`, one per virtual player.
        submitted_shares: vector<vector<vector<u8>>>,
        validator_weights: vector<u64>,
        submitters: vector<address>
    }

    /// Ordering key of the pending queue.
    struct PendingKey has copy, drop, store {
        reveal_expiry_us: u64,
        timelock_id: u64
    }

    struct TimelockRegistry has key {
        timelocks: Table<u64, TimelockInfo>,
        /// Pending timelocks in order of `(reveal_expiry_us, timelock_id)`.
        pending: BigOrderedMap<PendingKey, bool>,
        next_timelock_id: u64
    }

    // ================================
    // Events
    // ================================

    #[event]
    struct TimelockRegistrationEvent has drop, store {
        timelock_id: u64,
        epoch: u64,
        deadline_us: u64,
        reveal_expiry_us: u64,
        sender: address,
        timestamp_us: u64
    }

    #[event]
    struct TimelockRevealEvent has drop, store {
        timelock_id: u64,
        timestamp_us: u64
    }

    #[event]
    struct TimelockExpiredEvent has drop, store {
        timelock_id: u64,
        /// See `REASON_*`.
        reason: u8,
        timestamp_us: u64
    }

    // ================================
    // Initialization and per-epoch config
    // ================================

    /// Called in genesis. Creates the public params and the registry.
    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<IBEPublicParams>(@aptos_framework)) {
            move_to(aptos_framework, IBEPublicParams { mpk: vector::empty(), epoch: 0 });
        };
        initialize_timelock_registry(aptos_framework);
    }

    public fun initialize_timelock_registry(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<TimelockRegistry>(@aptos_framework)) {
            move_to(
                aptos_framework,
                TimelockRegistry {
                    timelocks: table::new(),
                    pending: big_ordered_map::new_with_type_size_hints(16, 16, 1, 1),
                    next_timelock_id: 0
                }
            );
        }
    }

    /// Stores the MPK of an epoch. Kept for callers that only know the MPK.
    public(friend) fun set_mpk(mpk: vector<u8>, epoch: u64) acquires IBEPublicParams {
        assert!(mpk.length() == G2_COMPRESSED_LENGTH, E_INVALID_MPK_LENGTH);
        let params = &mut IBEPublicParams[@aptos_framework];
        params.mpk = mpk;
        params.epoch = epoch;
    }

    /// Writes the epoch config. Must run after the epoch change, so that
    /// `reconfiguration::last_reconfiguration_time` is the start of `epoch`.
    /// Replaces any config of an earlier epoch. Also updates `IBEPublicParams`.
    public(friend) fun set_epoch_config(
        aptos_framework: &signer,
        epoch: u64,
        mpk: vector<u8>,
        reconstruct_threshold_weight: u64,
        weights: vector<u64>,
        epoch_interval_us: u64,
        public_parameters_hash: vector<u8>,
        subtranscript_hash: vector<u8>
    ) acquires IBEPublicParams, TimelockEpochConfig {
        system_addresses::assert_aptos_framework(aptos_framework);
        set_epoch_config_internal(
            aptos_framework,
            epoch,
            mpk,
            reconstruct_threshold_weight,
            weights,
            epoch_interval_us,
            public_parameters_hash,
            subtranscript_hash
        );
    }

    fun set_epoch_config_internal(
        aptos_framework: &signer,
        epoch: u64,
        mpk: vector<u8>,
        reconstruct_threshold_weight: u64,
        weights: vector<u64>,
        epoch_interval_us: u64,
        public_parameters_hash: vector<u8>,
        subtranscript_hash: vector<u8>
    ) acquires IBEPublicParams, TimelockEpochConfig {
        assert!(mpk.length() == G2_COMPRESSED_LENGTH, E_INVALID_MPK_LENGTH);
        let total_weight = weights.fold(0u64, |acc, w| acc + w);
        assert!(
            reconstruct_threshold_weight > 0 && reconstruct_threshold_weight <= total_weight,
            E_INVALID_EPOCH_CONFIG
        );
        assert!(epoch_interval_us > 0, E_INVALID_EPOCH_CONFIG);

        let config = TimelockEpochConfig {
            epoch,
            mpk,
            reconstruct_threshold_weight,
            total_weight,
            weights,
            epoch_interval_us,
            public_parameters_hash,
            subtranscript_hash
        };
        if (exists<TimelockEpochConfig>(@aptos_framework)) {
            *&mut TimelockEpochConfig[@aptos_framework] = config;
        } else {
            move_to(aptos_framework, config);
        };
        set_mpk(mpk, epoch);
    }

    // ================================
    // Config views
    // ================================

    #[view]
    public fun get_mpk(): vector<u8> acquires IBEPublicParams {
        if (!exists<IBEPublicParams>(@aptos_framework)) {
            return vector::empty()
        };
        IBEPublicParams[@aptos_framework].mpk
    }

    #[view]
    public fun get_epoch(): u64 acquires IBEPublicParams {
        if (!exists<IBEPublicParams>(@aptos_framework)) {
            return 0
        };
        IBEPublicParams[@aptos_framework].epoch
    }

    #[view]
    /// True when the epoch config is for the current epoch and holds a valid MPK.
    public fun is_ready(): bool acquires TimelockEpochConfig {
        if (!exists<TimelockEpochConfig>(@aptos_framework)) {
            return false
        };
        let config = &TimelockEpochConfig[@aptos_framework];
        config.epoch == reconfiguration::current_epoch()
            && config.mpk.length() == G2_COMPRESSED_LENGTH
    }

    #[view]
    /// `(epoch, reconstruct_threshold_weight, total_weight)` of the epoch config.
    public fun get_epoch_config(): (u64, u64, u64) acquires TimelockEpochConfig {
        assert!(exists<TimelockEpochConfig>(@aptos_framework), E_IBE_NOT_READY);
        let config = &TimelockEpochConfig[@aptos_framework];
        (config.epoch, config.reconstruct_threshold_weight, config.total_weight)
    }

    #[view]
    /// End of the current epoch: `last_reconfiguration_time + epoch_interval_us`.
    public fun epoch_end_us(): u64 acquires TimelockEpochConfig {
        assert!(exists<TimelockEpochConfig>(@aptos_framework), E_IBE_NOT_READY);
        reconfiguration::last_reconfiguration_time()
            + TimelockEpochConfig[@aptos_framework].epoch_interval_us
    }

    // ================================
    // Registration
    // ================================

    /// Registers a timelock for `deadline_us`.
    ///
    /// Aborts unless the MPK of the current epoch is ready, the deadline is in the future and the deadline is at
    /// most `epoch_end - 15 minutes`.
    public entry fun register_timelock(account: &signer, deadline_us: u64) acquires TimelockRegistry, TimelockEpochConfig {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_REGISTRY_NOT_INITIALIZED);
        assert!(is_ready(), E_IBE_NOT_READY);

        let now = timestamp::now_microseconds();
        assert!(now < deadline_us, E_DEADLINE_IN_PAST);

        let epoch_end = epoch_end_us();
        let config = &TimelockEpochConfig[@aptos_framework];
        let epoch = config.epoch;
        assert!(epoch_end >= EPOCH_END_GUARD_US, E_DEADLINE_AFTER_EPOCH_END);
        assert!(deadline_us <= epoch_end - EPOCH_END_GUARD_US, E_DEADLINE_AFTER_EPOCH_END);
        let reveal_expiry_us = deadline_us + REVEAL_WINDOW_US;
        if (reveal_expiry_us > epoch_end) {
            reveal_expiry_us = epoch_end;
        };
        let reveal_threshold = config.reconstruct_threshold_weight;

        let registry = &mut TimelockRegistry[@aptos_framework];
        let timelock_id = registry.next_timelock_id;
        registry.next_timelock_id = timelock_id + 1;

        let identity = compute_identity(chain_id::get(), epoch, timelock_id, deadline_us);

        registry.timelocks.add(
            timelock_id,
            TimelockInfo {
                timelock_id,
                epoch,
                deadline_us,
                reveal_expiry_us,
                identity,
                decryption_key: vector::empty(),
                status: TimelockStatus::Pending,
                share_count: 0,
                reveal_threshold,
                validator_indices: vector::empty(),
                submitted_shares: vector::empty(),
                validator_weights: vector::empty(),
                submitters: vector::empty()
            }
        );
        registry.pending.add(PendingKey { reveal_expiry_us, timelock_id }, true);

        event::emit(
            TimelockRegistrationEvent {
                timelock_id,
                epoch,
                deadline_us,
                reveal_expiry_us,
                sender: signer::address_of(account),
                timestamp_us: now
            }
        );
    }

    // ================================
    // Share submission and reveal
    // ================================

    /// Stores a share set of `validator_address` for a timelock. Called by the timelock validator transaction
    /// handler after the Rust share check passed. Weight and threshold come from the epoch config.
    public(friend) fun submit_dk_shares(
        timelock_id: u64,
        shares: vector<vector<u8>>,
        validator_address: address,
        share_epoch: u64
    ) acquires TimelockRegistry, TimelockEpochConfig {
        let validator_index = stake::get_validator_index(validator_address);
        submit_dk_shares_internal(timelock_id, shares, validator_index, validator_address, share_epoch);
    }

    fun submit_dk_shares_internal(
        timelock_id: u64,
        shares: vector<vector<u8>>,
        validator_index: u64,
        validator_address: address,
        share_epoch: u64
    ) acquires TimelockRegistry, TimelockEpochConfig {
        assert!(exists<TimelockEpochConfig>(@aptos_framework), E_IBE_NOT_READY);
        let config = &TimelockEpochConfig[@aptos_framework];
        let current_epoch = reconfiguration::current_epoch();
        assert!(config.epoch == current_epoch && share_epoch == current_epoch, E_WRONG_EPOCH);
        assert!(validator_index < config.weights.length(), E_UNKNOWN_VALIDATOR_INDEX);
        let weight = config.weights[validator_index];
        assert!(shares.length() == weight, E_INVALID_THRESHOLD);

        let registry = &mut TimelockRegistry[@aptos_framework];
        assert!(registry.timelocks.contains(timelock_id), E_TIMELOCK_NOT_FOUND);
        let info = registry.timelocks.borrow_mut(timelock_id);
        assert!(info.status is TimelockStatus::Pending, E_NOT_PENDING);
        assert!(info.epoch == current_epoch, E_WRONG_EPOCH);

        let now = timestamp::now_microseconds();
        assert!(now >= info.deadline_us, E_DEADLINE_NOT_PASSED);
        assert!(now < info.reveal_expiry_us, E_REVEAL_WINDOW_ELAPSED);
        assert!(!info.submitters.contains(&validator_address), E_SHARE_ALREADY_SUBMITTED);

        info.validator_indices.push_back(validator_index);
        info.submitted_shares.push_back(shares);
        info.validator_weights.push_back(weight);
        info.submitters.push_back(validator_address);
        info.share_count += weight;
    }

    /// Everything the handler needs to reconstruct the key:
    /// `(validator_indices, dk_shares, weights, threshold, total_weight, identity)`.
    /// Aborts unless the timelock is `Pending` in the current epoch and verified weight reached the threshold.
    public fun get_reconstruction_inputs(timelock_id: u64): (vector<u64>, vector<vector<vector<u8>>>, vector<u64>, u64, u64, vector<u8>)
    acquires TimelockRegistry, TimelockEpochConfig {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let registry = &TimelockRegistry[@aptos_framework];
        assert!(registry.timelocks.contains(timelock_id), E_TIMELOCK_NOT_FOUND);
        let info = registry.timelocks.borrow(timelock_id);
        assert!(effective_status(info) is TimelockStatus::Pending, E_NOT_PENDING);
        assert!(info.share_count >= info.reveal_threshold, E_BELOW_THRESHOLD);
        let total_weight = TimelockEpochConfig[@aptos_framework].total_weight;
        (
            info.validator_indices,
            info.submitted_shares,
            info.validator_weights,
            info.reveal_threshold,
            total_weight,
            info.identity
        )
    }

    /// Stores the reconstructed key after checking `e(dk, g2) = e(H1(id), mpk)` again.
    ///
    /// Returns true when the timelock is now `Revealed`. When the check fails the timelock becomes
    /// `Expired(key-check-failed)`, its shares are dropped, and the function returns false. It does not abort in
    /// that case, so that the status change persists. There is no retry after either outcome.
    public(friend) fun finalize_timelock_reveal(timelock_id: u64, dk: vector<u8>): bool
    acquires TimelockRegistry, TimelockEpochConfig {
        let (_, _, _, _, _, identity) = get_reconstruction_inputs(timelock_id);
        let mpk = TimelockEpochConfig[@aptos_framework].mpk;
        let key_ok = dk.length() == G1_LENGTH && ibe::verify_decryption_key(dk, identity, mpk);
        finalize_internal(timelock_id, dk, key_ok)
    }

    /// Called by the handler when the key it rebuilt in Rust failed the pairing check. Sets
    /// `Expired(key-check-failed)` and drops the stored shares. There is no retry.
    public(friend) fun fail_timelock_key_check(timelock_id: u64) acquires TimelockRegistry, TimelockEpochConfig {
        get_reconstruction_inputs(timelock_id);
        finalize_internal(timelock_id, vector::empty(), false);
    }

    /// True when the timelock is `Pending` in the current epoch and verified weight reached the threshold.
    /// The handler uses it to decide whether to rebuild the key after storing a share.
    public fun is_reconstructable(timelock_id: u64): bool acquires TimelockRegistry {
        if (!exists<TimelockRegistry>(@aptos_framework)) {
            return false
        };
        let registry = &TimelockRegistry[@aptos_framework];
        if (!registry.timelocks.contains(timelock_id)) {
            return false
        };
        let info = registry.timelocks.borrow(timelock_id);
        (effective_status(info) is TimelockStatus::Pending) && info.share_count >= info.reveal_threshold
    }

    fun finalize_internal(timelock_id: u64, dk: vector<u8>, key_ok: bool): bool acquires TimelockRegistry {
        let registry = &mut TimelockRegistry[@aptos_framework];
        assert!(registry.timelocks.contains(timelock_id), E_TIMELOCK_NOT_FOUND);
        let info = registry.timelocks.borrow_mut(timelock_id);
        assert!(info.status is TimelockStatus::Pending, E_NOT_PENDING);
        assert!(info.epoch == reconfiguration::current_epoch(), E_WRONG_EPOCH);
        assert!(info.share_count >= info.reveal_threshold, E_BELOW_THRESHOLD);

        let now = timestamp::now_microseconds();
        assert!(now < info.reveal_expiry_us, E_REVEAL_WINDOW_ELAPSED);

        let key = PendingKey { reveal_expiry_us: info.reveal_expiry_us, timelock_id };
        if (key_ok) {
            info.decryption_key = dk;
            info.status = TimelockStatus::Revealed;
            registry.pending.remove(&key);
            event::emit(TimelockRevealEvent { timelock_id, timestamp_us: now });
            true
        } else {
            expire(info, REASON_KEY_CHECK_FAILED);
            registry.pending.remove(&key);
            event::emit(
                TimelockExpiredEvent { timelock_id, reason: REASON_KEY_CHECK_FAILED, timestamp_us: now }
            );
            false
        }
    }

    /// Sets `Expired(reason)` and drops the stored shares.
    fun expire(info: &mut TimelockInfo, reason: u8) {
        info.status = TimelockStatus::Expired { reason };
        info.validator_indices = vector::empty();
        info.submitted_shares = vector::empty();
        info.validator_weights = vector::empty();
        info.submitters = vector::empty();
        info.share_count = 0;
    }

    /// Marks due timelocks `Expired` and emits one event each. Takes at most `MAX_EXPIRE_PER_BLOCK` per
    /// block, in order of `(reveal_expiry_us, timelock_id)`.
    ///
    /// Timelocks of an earlier epoch always have `reveal_expiry_us` at or before that epoch's end, so they sit at
    /// the front of the queue and are drained here with reason `epoch-ended`.
    public(friend) fun on_new_block(vm: &signer) acquires TimelockRegistry {
        system_addresses::assert_vm(vm);
        on_new_block_internal();
    }

    fun on_new_block_internal() acquires TimelockRegistry {
        if (!exists<TimelockRegistry>(@aptos_framework)) {
            return
        };
        let now = timestamp::now_microseconds();
        let current_epoch = reconfiguration::current_epoch();
        let registry = &mut TimelockRegistry[@aptos_framework];

        let processed = 0;
        while (processed < MAX_EXPIRE_PER_BLOCK && !registry.pending.is_empty()) {
            let (front, _) = registry.pending.borrow_front();
            if (front.reveal_expiry_us > now && registry.timelocks.borrow(front.timelock_id).epoch == current_epoch) {
                break
            };
            let (key, _) = registry.pending.pop_front();
            let info = registry.timelocks.borrow_mut(key.timelock_id);
            let reason =
                if (info.epoch != current_epoch) { REASON_EPOCH_ENDED }
                else { REASON_REVEAL_WINDOW_ELAPSED };
            expire(info, reason);
            event::emit(TimelockExpiredEvent { timelock_id: key.timelock_id, reason, timestamp_us: now });
            processed += 1;
        };
    }

    // ================================
    // Views
    // ================================

    fun borrow_info(registry: &TimelockRegistry, timelock_id: u64): &TimelockInfo {
        assert!(registry.timelocks.contains(timelock_id), E_TIMELOCK_NOT_FOUND);
        registry.timelocks.borrow(timelock_id)
    }

    /// The stored status, except that a `Pending` timelock of an earlier epoch reads as `Expired(epoch-ended)`.
    fun effective_status(info: &TimelockInfo): TimelockStatus {
        if ((info.status is TimelockStatus::Pending) && info.epoch != reconfiguration::current_epoch()) {
            TimelockStatus::Expired { reason: REASON_EPOCH_ENDED }
        } else {
            info.status
        }
    }

    #[view]
    /// `(deadline_us, identity, is_revealed, share_count)`.
    public fun get_timelock(timelock_id: u64): (u64, vector<u8>, bool, u64) acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let info = borrow_info(&TimelockRegistry[@aptos_framework], timelock_id);
        (info.deadline_us, info.identity, info.status is TimelockStatus::Revealed, info.share_count)
    }

    #[view]
    public fun get_deadline(timelock_id: u64): u64 acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        borrow_info(&TimelockRegistry[@aptos_framework], timelock_id).deadline_us
    }

    #[view]
    public fun get_identity(timelock_id: u64): vector<u8> acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        borrow_info(&TimelockRegistry[@aptos_framework], timelock_id).identity
    }

    #[view]
    /// Epoch the timelock was registered in.
    public fun get_timelock_epoch(timelock_id: u64): u64 acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        borrow_info(&TimelockRegistry[@aptos_framework], timelock_id).epoch
    }

    #[view]
    public fun get_reveal_expiry(timelock_id: u64): u64 acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        borrow_info(&TimelockRegistry[@aptos_framework], timelock_id).reveal_expiry_us
    }

    #[view]
    public fun get_decryption_key(timelock_id: u64): vector<u8> acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let info = borrow_info(&TimelockRegistry[@aptos_framework], timelock_id);
        assert!(info.status is TimelockStatus::Revealed, E_DECRYPTION_KEY_NOT_REVEALED);
        info.decryption_key
    }

    #[view]
    /// `0` pending, `1` revealed, `2` expired. Aborts for an unknown timelock.
    public fun get_status(timelock_id: u64): u8 acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let info = borrow_info(&TimelockRegistry[@aptos_framework], timelock_id);
        match (effective_status(info)) {
            TimelockStatus::Pending => STATUS_PENDING,
            TimelockStatus::Revealed => STATUS_REVEALED,
            TimelockStatus::Expired { reason: _ } => STATUS_EXPIRED
        }
    }

    #[view]
    /// `1` epoch-ended, `2` reveal-window-elapsed, `3` key-check-failed, `0` when the timelock is not expired.
    /// The auction contract uses this for the refund path.
    public fun get_expiry_reason(timelock_id: u64): u8 acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let info = borrow_info(&TimelockRegistry[@aptos_framework], timelock_id);
        match (effective_status(info)) {
            TimelockStatus::Expired { reason } => reason,
            _ => REASON_NONE
        }
    }

    #[view]
    public fun is_revealed(timelock_id: u64): bool acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let registry = &TimelockRegistry[@aptos_framework];
        if (!registry.timelocks.contains(timelock_id)) {
            return false
        };
        registry.timelocks.borrow(timelock_id).status is TimelockStatus::Revealed
    }

    #[view]
    /// True once the deadline has passed. This is not the `Expired` status. See `is_expired`.
    public fun is_deadline_passed(timelock_id: u64): bool acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let registry = &TimelockRegistry[@aptos_framework];
        if (!registry.timelocks.contains(timelock_id)) {
            return false
        };
        timestamp::now_microseconds() >= registry.timelocks.borrow(timelock_id).deadline_us
    }

    #[view]
    /// True when the status is `Expired`, for any reason.
    public fun is_expired(timelock_id: u64): bool acquires TimelockRegistry {
        assert!(exists<TimelockRegistry>(@aptos_framework), E_TIMELOCK_NOT_FOUND);
        let registry = &TimelockRegistry[@aptos_framework];
        if (!registry.timelocks.contains(timelock_id)) {
            return false
        };
        effective_status(registry.timelocks.borrow(timelock_id)) is TimelockStatus::Expired
    }

    #[view]
    public fun get_next_timelock_id(): u64 acquires TimelockRegistry {
        TimelockRegistry[@aptos_framework].next_timelock_id
    }

    /// Identity of a timelock. See the module doc for the byte layout.
    public fun compute_identity(chain: u8, epoch: u64, timelock_id: u64, deadline_us: u64): vector<u8> {
        let input = IDENTITY_TAG;
        input.append(bcs::to_bytes(&chain));
        input.append(bcs::to_bytes(&epoch));
        input.append(bcs::to_bytes(&timelock_id));
        input.append(bcs::to_bytes(&deadline_us));
        sha3_256(input)
    }

    // ================================
    // Test-only helpers
    // ================================

    #[test_only]
    public fun initialize_for_testing(aptos_framework: &signer) {
        aptos_framework::account::create_account_for_test(@aptos_framework);
        timestamp::set_time_has_started_for_testing(aptos_framework);
        chain_id::initialize_for_test(aptos_framework, 4);
        reconfiguration::initialize_for_test(aptos_framework);
        initialize(aptos_framework);
    }

    #[test_only]
    public fun set_epoch_config_for_testing(
        aptos_framework: &signer,
        epoch: u64,
        mpk: vector<u8>,
        threshold: u64,
        weights: vector<u64>,
        epoch_interval_us: u64
    ) acquires IBEPublicParams, TimelockEpochConfig {
        set_epoch_config_internal(aptos_framework, epoch, mpk, threshold, weights, epoch_interval_us, vector[], vector[]);
    }

    #[test_only]
    public fun submit_dk_shares_for_testing(
        timelock_id: u64,
        shares: vector<vector<u8>>,
        validator_index: u64,
        validator_address: address,
        share_epoch: u64
    ) acquires TimelockRegistry, TimelockEpochConfig {
        submit_dk_shares_internal(timelock_id, shares, validator_index, validator_address, share_epoch);
    }

    #[test_only]
    /// Runs the finalize logic with a chosen key-check result, because the pairing native is not available to
    /// Move unit tests.
    public fun finalize_for_testing(timelock_id: u64, dk: vector<u8>, key_ok: bool): bool
    acquires TimelockRegistry {
        finalize_internal(timelock_id, dk, key_ok)
    }

    #[test_only]
    public fun fail_key_check_for_testing(timelock_id: u64) acquires TimelockRegistry, TimelockEpochConfig {
        fail_timelock_key_check(timelock_id);
    }

    #[test_only]
    public fun on_new_block_for_testing() acquires TimelockRegistry {
        on_new_block_internal();
    }
}
