/// Timelock chunky DKG on-chain state and helpers (fork only).
///
/// Design: `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md` section 4. This module mirrors
/// `chunky_dkg.move` for a second chunky DKG session per epoch. The session has the same dealers and the
/// same target validator set as the mempool session, but its own state, its own start event, its own
/// session id (`TimelockDKGSessionId` in Rust) and its own validator transaction (`TimelockDKGResult`).
/// `chunky_dkg.move` is not edited.
///
/// The result of the session is turned into a `TimelockEpochConfig` for the target epoch. The config is
/// written by `on_new_epoch`, after the epoch change, because `ibe_config` derives the epoch end from
/// `reconfiguration::last_reconfiguration_time`.
///
/// ## Failure never blocks the epoch change
///
/// An incomplete timelock session delays the epoch change by at most `grace_period_secs`. After that the
/// epoch changes without a timelock key. `ibe_config::is_ready()` is then false and `register_timelock`
/// aborts with `E_IBE_NOT_READY`.
///
/// The feature is off until governance calls `enable`. While it is off, no session starts and no code
/// path of the epoch change reads this module.
module aptos_framework::timelock_dkg {
    use std::error;
    use std::option::{Self, Option};
    use aptos_framework::chain_id;
    use aptos_framework::chunky_dkg_config::ChunkyDKGConfig;
    use aptos_framework::event::emit;
    use aptos_framework::ibe_config;
    use aptos_framework::reconfiguration;
    use aptos_framework::system_addresses;
    use aptos_framework::timestamp;
    use aptos_framework::validator_consensus_info::ValidatorConsensusInfo;
    friend aptos_framework::reconfiguration_with_dkg;

    const ETIMELOCK_DKG_NOT_IN_PROGRESS: u64 = 1;
    const ETIMELOCK_DKG_NOT_ENABLED: u64 = 2;
    const EINVALID_RESULT: u64 = 3;
    const EINVALID_PARAMETERS_HASH: u64 = 4;
    /// `clear_expected_public_parameters_hash` runs only on the test chain (chain id 4).
    const ECLEAR_HASH_ONLY_ON_TEST_CHAIN: u64 = 5;

    /// Length of a SHA3-256 hash of the timelock `PublicParameters` blob.
    const PARAMETERS_HASH_LENGTH: u64 = 32;

    /// Default wait for the timelock session before the epoch changes without it.
    const DEFAULT_GRACE_PERIOD_SECS: u64 = 120;

    /// Same layout as `chunky_dkg::ChunkyDKGSessionMetadata`, so Rust reads it as the same type.
    struct TimelockDKGSessionMetadata has copy, drop, store {
        dealer_epoch: u64,
        chunky_dkg_config: ChunkyDKGConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>
    }

    #[event]
    struct TimelockDKGStartEvent has drop, store {
        session_metadata: TimelockDKGSessionMetadata,
        start_time_us: u64
    }

    /// Emitted in the reconfiguration transaction by `on_new_epoch` when the feature is on and no
    /// `TimelockEpochConfig` was written for the new epoch. `epoch` is the new epoch, which has no
    /// timelock key: `ibe_config::is_ready()` is false and `register_timelock` aborts in it.
    #[event]
    struct TimelockKeyMissingEvent has drop, store {
        epoch: u64
    }

    struct TimelockDKGSessionState has copy, store, drop {
        metadata: TimelockDKGSessionMetadata,
        start_time_us: u64,
        aggregated_subtranscript: vector<u8>
    }

    /// The epoch config derived from an accepted result. Written to `ibe_config` after the epoch change.
    struct PendingTimelockConfig has copy, store, drop {
        /// Epoch the config is for: `dealer_epoch + 1`.
        target_epoch: u64,
        mpk: vector<u8>,
        reconstruct_threshold_weight: u64,
        weights: vector<u64>,
        epoch_interval_us: u64,
        public_parameters_hash: vector<u8>,
        subtranscript_hash: vector<u8>,
        verification_keys: vector<vector<u8>>
    }

    struct TimelockDKGState has key {
        enabled: bool,
        grace_period_secs: u64,
        last_completed: Option<TimelockDKGSessionState>,
        in_progress: Option<TimelockDKGSessionState>,
        pending_config: Option<PendingTimelockConfig>
    }

    /// The hash of the timelock `PublicParameters` blob that a `TimelockDKGResult` must carry. Set at
    /// genesis (layout field `timelock_public_parameters_hash`) or by governance. The handler reads it from
    /// chain state, never from a node-local file, so every node executes the result the same way. When it
    /// is absent the handler rejects every result, except on the test chain (chain id 4), which uses the
    /// built-in test parameters and stays unpinned. Each node also refuses to serve the timelock when its
    /// blob differs from `TimelockEpochConfig`.
    struct ExpectedPublicParametersHash has key, drop {
        hash: vector<u8>
    }

    /// Requires every later `TimelockDKGResult` to carry `hash`. Replaces an earlier value.
    public fun set_expected_public_parameters_hash(
        aptos_framework: &signer, hash: vector<u8>
    ) acquires ExpectedPublicParametersHash {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            hash.length() == PARAMETERS_HASH_LENGTH,
            error::invalid_argument(EINVALID_PARAMETERS_HASH)
        );
        if (exists<ExpectedPublicParametersHash>(@aptos_framework)) {
            ExpectedPublicParametersHash[@aptos_framework].hash = hash;
        } else {
            move_to(aptos_framework, ExpectedPublicParametersHash { hash });
        }
    }

    /// Removes the pinned hash on the test chain (chain id 4) only. Idempotent there. The test chain
    /// accepts a result with no pinned hash. On any other chain an absent hash rejects every
    /// `TimelockDKGResult`, so this aborts with `ECLEAR_HASH_ONLY_ON_TEST_CHAIN`.
    public fun clear_expected_public_parameters_hash(
        aptos_framework: &signer
    ) acquires ExpectedPublicParametersHash {
        system_addresses::assert_aptos_framework(aptos_framework);
        assert!(
            chain_id::get() == 4,
            error::invalid_state(ECLEAR_HASH_ONLY_ON_TEST_CHAIN)
        );
        if (exists<ExpectedPublicParametersHash>(@aptos_framework)) {
            move_from<ExpectedPublicParametersHash>(@aptos_framework);
        }
    }

    #[view]
    public fun expected_public_parameters_hash(): Option<vector<u8>> acquires ExpectedPublicParametersHash {
        if (exists<ExpectedPublicParametersHash>(@aptos_framework)) {
            option::some(ExpectedPublicParametersHash[@aptos_framework].hash)
        } else {
            option::none()
        }
    }

    /// Creates the state, disabled. Idempotent.
    public fun initialize(aptos_framework: &signer) {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<TimelockDKGState>(@aptos_framework)) {
            move_to(
                aptos_framework,
                TimelockDKGState {
                    enabled: false,
                    grace_period_secs: DEFAULT_GRACE_PERIOD_SECS,
                    last_completed: option::none(),
                    in_progress: option::none(),
                    pending_config: option::none()
                }
            );
        }
    }

    /// Turns the timelock DKG on. Takes effect for the next reconfiguration.
    public fun enable(aptos_framework: &signer, grace_period_secs: u64) acquires TimelockDKGState {
        initialize(aptos_framework);
        let state = &mut TimelockDKGState[@aptos_framework];
        state.enabled = true;
        state.grace_period_secs = grace_period_secs;
    }

    /// Turns the timelock DKG off. Takes effect for the next reconfiguration.
    public fun disable(aptos_framework: &signer) acquires TimelockDKGState {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (exists<TimelockDKGState>(@aptos_framework)) {
            TimelockDKGState[@aptos_framework].enabled = false;
        }
    }

    #[view]
    public fun is_enabled(): bool acquires TimelockDKGState {
        exists<TimelockDKGState>(@aptos_framework)
            && TimelockDKGState[@aptos_framework].enabled
    }

    /// Wait for the session, in microseconds, before the epoch changes without it.
    public fun grace_period_us(): u64 acquires TimelockDKGState {
        if (exists<TimelockDKGState>(@aptos_framework)) {
            TimelockDKGState[@aptos_framework].grace_period_secs * 1_000_000
        } else {
            DEFAULT_GRACE_PERIOD_SECS * 1_000_000
        }
    }

    /// Marks the session in progress and notifies the validators. Idempotent for `dealer_epoch`.
    /// Does nothing while the feature is off. Only `reconfiguration_with_dkg::try_start_with_chunky_dkg`
    /// calls this. A reconfiguration through `try_start` starts no session, so its new epoch has no key,
    /// and `on_new_epoch` reports that epoch with `TimelockKeyMissingEvent`.
    public(friend) fun start(
        dealer_epoch: u64,
        chunky_dkg_config: ChunkyDKGConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>
    ) acquires TimelockDKGState {
        if (!is_enabled()) { return };
        if (is_session_started(dealer_epoch)) { return };
        let state = &mut TimelockDKGState[@aptos_framework];
        let metadata = TimelockDKGSessionMetadata {
            dealer_epoch,
            chunky_dkg_config,
            dealer_validator_set,
            target_validator_set
        };
        let start_time_us = timestamp::now_microseconds();
        state.in_progress = option::some(
            TimelockDKGSessionState {
                metadata,
                start_time_us,
                aggregated_subtranscript: vector[]
            }
        );
        emit(TimelockDKGStartEvent { session_metadata: metadata, start_time_us });
    }

    /// Stores the aggregated subtranscript and the config derived from it. Marks the session completed.
    /// Aborts when no session is in progress.
    public(friend) fun finish(
        aggregated_subtranscript: vector<u8>,
        mpk: vector<u8>,
        reconstruct_threshold_weight: u64,
        weights: vector<u64>,
        epoch_interval_us: u64,
        public_parameters_hash: vector<u8>,
        subtranscript_hash: vector<u8>,
        verification_keys: vector<vector<u8>>
    ) acquires TimelockDKGState {
        assert!(exists<TimelockDKGState>(@aptos_framework), error::invalid_state(ETIMELOCK_DKG_NOT_ENABLED));
        let state = &mut TimelockDKGState[@aptos_framework];
        assert!(
            state.in_progress.is_some(),
            error::invalid_state(ETIMELOCK_DKG_NOT_IN_PROGRESS)
        );
        assert!(
            ibe_config::is_valid_epoch_config(
                &mpk, reconstruct_threshold_weight, &weights, epoch_interval_us, &verification_keys
            ),
            error::invalid_argument(EINVALID_RESULT)
        );
        let session = state.in_progress.extract();
        let target_epoch = session.metadata.dealer_epoch + 1;
        session.aggregated_subtranscript = aggregated_subtranscript;
        state.last_completed = option::some(session);
        state.in_progress = option::none();
        state.pending_config = option::some(
            PendingTimelockConfig {
                target_epoch,
                mpk,
                reconstruct_threshold_weight,
                weights,
                epoch_interval_us,
                public_parameters_hash,
                subtranscript_hash,
                verification_keys
            }
        );
    }

    /// Delete the incomplete session, if it exists.
    public fun try_clear_incomplete_session(fx: &signer) acquires TimelockDKGState {
        system_addresses::assert_aptos_framework(fx);
        if (exists<TimelockDKGState>(@aptos_framework)) {
            TimelockDKGState[@aptos_framework].in_progress = option::none();
        }
    }

    /// Runs after the epoch change. Writes the `TimelockEpochConfig` of the new epoch when the session of
    /// the last epoch completed. Drops a pending config that is not for the new epoch. When nothing is
    /// written, the old config stays and `ibe_config::is_ready()` is false for the new epoch. Then, while
    /// the feature is on, it emits `TimelockKeyMissingEvent` for the new epoch. Never aborts the epoch
    /// change.
    public(friend) fun on_new_epoch(aptos_framework: &signer) acquires TimelockDKGState {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<TimelockDKGState>(@aptos_framework)) { return };
        let state = &mut TimelockDKGState[@aptos_framework];
        let epoch = reconfiguration::current_epoch();
        let written = try_write_pending_config(aptos_framework, &mut state.pending_config, epoch);
        if (!written && state.enabled) {
            emit(TimelockKeyMissingEvent { epoch });
        }
    }

    /// Writes the pending config when it is for `current_epoch` and `ibe_config` accepts it. Returns
    /// whether a config was written. A config for a later epoch waits for that epoch. A config for an
    /// earlier epoch is stale and is dropped. A refused config is dropped.
    fun try_write_pending_config(
        aptos_framework: &signer,
        pending_config: &mut Option<PendingTimelockConfig>,
        current_epoch: u64
    ): bool {
        if (pending_config.is_none()) { return false };
        if (pending_config.borrow().target_epoch > current_epoch) { return false };
        let pending = pending_config.extract();
        if (pending.target_epoch != current_epoch) { return false };
        // Never abort the epoch change: skip a config that `ibe_config` would refuse.
        if (!ibe_config::is_initialized()
            || !ibe_config::is_valid_epoch_config(
                &pending.mpk,
                pending.reconstruct_threshold_weight,
                &pending.weights,
                pending.epoch_interval_us,
                &pending.verification_keys
            )) { return false };
        let PendingTimelockConfig {
            target_epoch,
            mpk,
            reconstruct_threshold_weight,
            weights,
            epoch_interval_us,
            public_parameters_hash,
            subtranscript_hash,
            verification_keys
        } = pending;
        ibe_config::set_epoch_config(
            aptos_framework,
            target_epoch,
            mpk,
            reconstruct_threshold_weight,
            weights,
            epoch_interval_us,
            public_parameters_hash,
            subtranscript_hash,
            verification_keys
        );
        true
    }

    public fun incomplete_session(): Option<TimelockDKGSessionState> acquires TimelockDKGState {
        if (exists<TimelockDKGState>(@aptos_framework)) {
            TimelockDKGState[@aptos_framework].in_progress
        } else {
            option::none()
        }
    }

    public fun last_completed_session(): Option<TimelockDKGSessionState> acquires TimelockDKGState {
        if (exists<TimelockDKGState>(@aptos_framework)) {
            TimelockDKGState[@aptos_framework].last_completed
        } else {
            option::none()
        }
    }

    public fun session_dealer_epoch(session: &TimelockDKGSessionState): u64 {
        session.metadata.dealer_epoch
    }

    public fun session_start_time(session: &TimelockDKGSessionState): u64 {
        session.start_time_us
    }

    /// True iff a timelock session was started for `epoch` (in progress or completed).
    public fun is_session_started(epoch: u64): bool acquires TimelockDKGState {
        let in_prog = incomplete_session();
        if (in_prog.is_some() && session_dealer_epoch(in_prog.borrow()) == epoch) {
            return true
        };
        let last = last_completed_session();
        last.is_some() && session_dealer_epoch(last.borrow()) == epoch
    }

    #[test_only]
    public fun enable_for_testing(aptos_framework: &signer) acquires TimelockDKGState {
        enable(aptos_framework, 5);
    }

    #[test_only]
    public fun start_for_testing(
        dealer_epoch: u64,
        chunky_dkg_config: ChunkyDKGConfig,
        dealer_validator_set: vector<ValidatorConsensusInfo>,
        target_validator_set: vector<ValidatorConsensusInfo>
    ) acquires TimelockDKGState {
        start(dealer_epoch, chunky_dkg_config, dealer_validator_set, target_validator_set);
    }

    #[test_only]
    public fun finish_for_testing(
        aggregated_subtranscript: vector<u8>,
        mpk: vector<u8>,
        reconstruct_threshold_weight: u64,
        weights: vector<u64>,
        epoch_interval_us: u64,
        verification_keys: vector<vector<u8>>
    ) acquires TimelockDKGState {
        finish(
            aggregated_subtranscript, mpk, reconstruct_threshold_weight, weights, epoch_interval_us,
            vector[], vector[], verification_keys
        );
    }

    #[test_only]
    public fun on_new_epoch_for_testing(aptos_framework: &signer) acquires TimelockDKGState {
        on_new_epoch(aptos_framework);
    }

    #[test_only]
    /// The `epoch` of an emitted `TimelockKeyMissingEvent`.
    public fun key_missing_event_epoch_for_testing(e: &TimelockKeyMissingEvent): u64 {
        e.epoch
    }

    #[test_only]
    /// Sets a pending config directly, so tests can reach the paths `finish` refuses.
    public fun set_pending_config_for_testing(
        target_epoch: u64,
        mpk: vector<u8>,
        reconstruct_threshold_weight: u64,
        weights: vector<u64>,
        epoch_interval_us: u64,
        verification_keys: vector<vector<u8>>
    ) acquires TimelockDKGState {
        TimelockDKGState[@aptos_framework].pending_config = option::some(
            PendingTimelockConfig {
                target_epoch,
                mpk,
                reconstruct_threshold_weight,
                weights,
                epoch_interval_us,
                public_parameters_hash: vector[],
                subtranscript_hash: vector[],
                verification_keys
            }
        );
    }
}
