#[test_only]
/// State rules of `timelock_dkg`: the feature switch, the session lifecycle, and the rule that the
/// `TimelockEpochConfig` is written only after the epoch change, only for the target epoch, and never
/// aborts the epoch change. A failed or missing result leaves `ibe_config::is_ready()` false, and
/// `register_timelock` aborts with `E_IBE_NOT_READY` (code 2).
module aptos_framework::timelock_dkg_tests {
    use std::vector;
    use aptos_framework::account;
    use aptos_framework::chunky_dkg_config;
    use aptos_framework::ibe_config;
    use aptos_framework::ibe_golden_vector_fixtures as fixtures;
    use aptos_framework::reconfiguration;
    use aptos_framework::timelock_dkg;
    use aptos_framework::timestamp;

    const EPOCH_INTERVAL: u64 = 7_200_000_000;

    fun setup(): signer {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);
        timelock_dkg::initialize(&framework);
        framework
    }

    fun vks(n: u64): vector<vector<u8>> {
        let out = vector[];
        let i = 0;
        while (i < n) {
            out.push_back(fixtures::timelock_basic_mpk());
            i += 1;
        };
        out
    }

    fun start_session(epoch: u64) {
        timelock_dkg::start_for_testing(epoch, chunky_dkg_config::new_off(), vector[], vector[]);
    }

    fun finish_ok(weights: vector<u64>, threshold: u64, total: u64) {
        timelock_dkg::finish_for_testing(
            x"01", fixtures::timelock_basic_mpk(), threshold, weights, EPOCH_INTERVAL, vks(total)
        );
    }

    fun next_epoch() {
        timestamp::update_global_time_for_test(timestamp::now_microseconds() + 1);
        reconfiguration::reconfigure_for_test_custom();
    }

    #[test]
    fun disabled_by_default_and_start_is_a_noop() {
        setup();
        assert!(!timelock_dkg::is_enabled(), 1);
        start_session(0);
        assert!(timelock_dkg::incomplete_session().is_none(), 2);
    }

    #[test]
    fun enabled_start_is_idempotent_per_epoch() {
        let framework = setup();
        timelock_dkg::enable_for_testing(&framework);
        start_session(0);
        assert!(timelock_dkg::incomplete_session().is_some(), 1);
        assert!(timelock_dkg::is_session_started(0), 2);
        assert!(!timelock_dkg::is_session_started(1), 3);
        let t0 = timelock_dkg::session_start_time(timelock_dkg::incomplete_session().borrow());
        timestamp::update_global_time_for_test(t0 + 50);
        start_session(0);
        let t1 = timelock_dkg::session_start_time(timelock_dkg::incomplete_session().borrow());
        assert!(t0 == t1, 4);
    }

    #[test]
    fun result_writes_epoch_config_only_after_the_epoch_change() {
        let framework = setup();
        timelock_dkg::enable_for_testing(&framework);
        start_session(0);
        finish_ok(vector[2, 1, 2], 3, 5);
        assert!(timelock_dkg::incomplete_session().is_none(), 1);
        assert!(timelock_dkg::last_completed_session().is_some(), 2);
        // Epoch 0 still runs: nothing is written yet.
        timelock_dkg::on_new_epoch_for_testing(&framework);
        assert!(!ibe_config::is_ready(), 3);
        next_epoch();
        timelock_dkg::on_new_epoch_for_testing(&framework);
        assert!(reconfiguration::current_epoch() == 1, 4);
        assert!(ibe_config::is_ready(), 5);
        let (epoch, threshold, total) = ibe_config::get_epoch_config();
        assert!(epoch == 1 && threshold == 3 && total == 5, 6);
        assert!(ibe_config::get_mpk() == fixtures::timelock_basic_mpk(), 7);
    }

    #[test]
    fun missing_result_leaves_the_new_epoch_without_a_key() {
        let framework = setup();
        timelock_dkg::enable_for_testing(&framework);
        start_session(0);
        timelock_dkg::try_clear_incomplete_session(&framework);
        next_epoch();
        timelock_dkg::on_new_epoch_for_testing(&framework);
        assert!(!ibe_config::is_ready(), 1);
    }

    #[test]
    fun stale_result_is_dropped_and_old_config_is_not_ready() {
        let framework = setup();
        timelock_dkg::enable_for_testing(&framework);
        start_session(0);
        finish_ok(vector[2, 1, 2], 3, 5);
        next_epoch();
        timelock_dkg::on_new_epoch_for_testing(&framework);
        assert!(ibe_config::is_ready(), 1);
        // Epoch 1 has no timelock result: the epoch 1 config is not ready in epoch 2.
        next_epoch();
        timelock_dkg::on_new_epoch_for_testing(&framework);
        assert!(!ibe_config::is_ready(), 2);
    }

    #[test]
    #[expected_failure(abort_code = 2, location = aptos_framework::ibe_config)]
    fun register_timelock_aborts_when_the_session_failed() {
        let framework = setup();
        timelock_dkg::enable_for_testing(&framework);
        start_session(0);
        timelock_dkg::try_clear_incomplete_session(&framework);
        next_epoch();
        timelock_dkg::on_new_epoch_for_testing(&framework);
        let user = account::create_signer_for_test(@0xa11ce);
        ibe_config::register_timelock(&user, timestamp::now_microseconds() + 1_000_000);
    }

    #[test]
    #[expected_failure(abort_code = 0x10003, location = aptos_framework::timelock_dkg)]
    fun finish_rejects_an_invalid_result() {
        let framework = setup();
        timelock_dkg::enable_for_testing(&framework);
        start_session(0);
        // 5 weights but only 4 verification keys.
        timelock_dkg::finish_for_testing(
            x"01", fixtures::timelock_basic_mpk(), 3, vector[2, 1, 2], EPOCH_INTERVAL, vks(4)
        );
    }

    #[test]
    #[expected_failure(abort_code = 0x30001, location = aptos_framework::timelock_dkg)]
    fun finish_without_a_session_aborts() {
        let framework = setup();
        timelock_dkg::enable_for_testing(&framework);
        finish_ok(vector[2, 1, 2], 3, 5);
    }

    #[test]
    fun disable_turns_the_feature_off() {
        let framework = setup();
        timelock_dkg::enable_for_testing(&framework);
        timelock_dkg::disable(&framework);
        start_session(0);
        assert!(timelock_dkg::incomplete_session().is_none(), 1);
        let _ = vector::length(&vks(0));
    }
}
