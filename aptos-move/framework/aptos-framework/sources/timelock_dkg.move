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
    /// Does nothing while the feature is off.
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
    /// written, the old config stays and `ibe_config::is_ready()` is false for the new epoch.
    public(friend) fun on_new_epoch(aptos_framework: &signer) acquires TimelockDKGState {
        system_addresses::assert_aptos_framework(aptos_framework);
        if (!exists<TimelockDKGState>(@aptos_framework)) { return };
        let state = &mut TimelockDKGState[@aptos_framework];
        if (state.pending_config.is_none()) { return };
        let current_epoch = reconfiguration::current_epoch();
        // A config for a later epoch waits for that epoch. A config for an earlier epoch is stale.
        if (state.pending_config.borrow().target_epoch > current_epoch) { return };
        let pending = state.pending_config.extract();
        if (pending.target_epoch != current_epoch) { return };
        // Never abort the epoch change: skip a config that `ibe_config` would refuse.
        if (!ibe_config::is_initialized()
            || !ibe_config::is_valid_epoch_config(
                &pending.mpk,
                pending.reconstruct_threshold_weight,
                &pending.weights,
                pending.epoch_interval_us,
                &pending.verification_keys
            )) { return };
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
}
