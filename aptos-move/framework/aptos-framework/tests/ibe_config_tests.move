#[test_only]
/// Registry rules of `ibe_config` that do not depend on the Rust natives.
///
/// The pairing check inside `finalize_timelock_reveal` is a native. These tests drive the same
/// state machine through `finalize_for_testing(.., key_ok)`.
module aptos_framework::ibe_config_tests {
    use std::vector;
    use aptos_framework::account;
    use aptos_framework::event;
    use aptos_framework::ibe_config;
    use aptos_framework::ibe_golden_vector_fixtures as fixtures;
    use aptos_framework::reconfiguration;
    use aptos_framework::timestamp;

    const MIN: u64 = 60_000_000;
    const EPOCH_INTERVAL: u64 = 7_200_000_000; // 2 hours

    fun setup(): signer {
        let framework = account::create_signer_for_test(@0x1);
        ibe_config::initialize_for_testing(&framework);
        framework
    }

    /// Epoch 0 config with weights [2, 1, 2], threshold 3, and time 0 as the epoch start.
    fun setup_ready(): signer {
        let framework = setup();
        ibe_config::set_epoch_config_for_testing(
            &framework, 0, fixtures::timelock_basic_mpk(), 3, vector[2, 1, 2], EPOCH_INTERVAL
        );
        framework
    }

    fun shares(n: u64): vector<vector<u8>> {
        let out = vector[];
        let i = 0;
        while (i < n) {
            out.push_back(x"aa");
            i += 1;
        };
        out
    }

    fun register(user: &signer, deadline_us: u64): u64 {
        let id = ibe_config::get_next_timelock_id();
        ibe_config::register_timelock(user, deadline_us);
        id
    }

    // ---- readiness and registration ----

    #[test]
    fun not_ready_before_config() {
        setup();
        assert!(!ibe_config::is_ready(), 1);
        assert!(ibe_config::get_epoch() == 0, 2);
    }

    #[test]
    fun ready_after_config() {
        setup_ready();
        assert!(ibe_config::is_ready(), 1);
        assert!(ibe_config::get_mpk() == fixtures::timelock_basic_mpk(), 2);
        let (epoch, threshold, total) = ibe_config::get_epoch_config();
        assert!(epoch == 0 && threshold == 3 && total == 5, 3);
    }

    #[test(framework = @0x1)]
    #[expected_failure(abort_code = 1, location = aptos_framework::ibe_config)]
    fun config_rejects_short_mpk(framework: &signer) {
        setup();
        ibe_config::set_epoch_config_for_testing(framework, 0, x"00", 3, vector[2, 1, 2], EPOCH_INTERVAL);
    }

    #[test(framework = @0x1)]
    #[expected_failure(abort_code = 16, location = aptos_framework::ibe_config)]
    fun config_rejects_threshold_above_total(framework: &signer) {
        setup();
        ibe_config::set_epoch_config_for_testing(
            framework, 0, fixtures::timelock_basic_mpk(), 6, vector[2, 1, 2], EPOCH_INTERVAL
        );
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 2, location = aptos_framework::ibe_config)]
    fun register_aborts_when_not_ready(user: &signer) {
        setup();
        register(user, 60 * MIN);
    }

    #[test(user = @0xA)]
    fun register_records_epoch_deadline_identity_and_pending(user: &signer) {
        setup_ready();
        let deadline = 60 * MIN;
        let id = register(user, deadline);
        assert!(id == 0, 1);
        assert!(ibe_config::get_next_timelock_id() == 1, 2);
        assert!(ibe_config::get_deadline(id) == deadline, 3);
        assert!(ibe_config::get_timelock_epoch(id) == 0, 4);
        assert!(ibe_config::get_reveal_expiry(id) == deadline + 10 * MIN, 5);
        assert!(ibe_config::get_status(id) == 0, 6);
        assert!(ibe_config::get_expiry_reason(id) == 0, 7);
        let identity = ibe_config::get_identity(id);
        assert!(identity.length() == 32, 8);
        assert!(identity == ibe_config::compute_identity(4, 0, id, deadline), 9);
        let (d, i, revealed, count) = ibe_config::get_timelock(id);
        assert!(d == deadline && i == identity && !revealed && count == 0, 10);
    }

    #[test(user = @0xA)]
    fun duplicate_registration_gets_distinct_id_and_identity(user: &signer) {
        setup_ready();
        let a = register(user, 60 * MIN);
        let b = register(user, 60 * MIN);
        assert!(a == 0 && b == 1, 1);
        assert!(ibe_config::get_identity(a) != ibe_config::get_identity(b), 2);
    }

    #[test]
    fun identity_binds_chain_epoch_id_and_deadline() {
        let base = ibe_config::compute_identity(4, 1, 2, 3);
        assert!(base != ibe_config::compute_identity(5, 1, 2, 3), 1);
        assert!(base != ibe_config::compute_identity(4, 2, 2, 3), 2);
        assert!(base != ibe_config::compute_identity(4, 1, 3, 3), 3);
        assert!(base != ibe_config::compute_identity(4, 1, 2, 4), 4);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 8, location = aptos_framework::ibe_config)]
    fun register_aborts_for_deadline_not_in_future(user: &signer) {
        setup_ready();
        timestamp::update_global_time_for_test(10 * MIN);
        register(user, 10 * MIN);
    }

    #[test(user = @0xA)]
    fun register_accepts_deadline_exactly_at_epoch_end_minus_guard(user: &signer) {
        setup_ready();
        register(user, EPOCH_INTERVAL - 15 * MIN);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 9, location = aptos_framework::ibe_config)]
    fun register_aborts_after_epoch_end_guard(user: &signer) {
        setup_ready();
        register(user, EPOCH_INTERVAL - 15 * MIN + 1);
    }

    // ---- share submission ----

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 3, location = aptos_framework::ibe_config)]
    fun submit_before_deadline_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
    }

    #[test(user = @0xA)]
    fun submit_accumulates_weight_from_config(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 2, 1);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 3, 2);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 7, location = aptos_framework::ibe_config)]
    fun duplicate_submitter_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 6, location = aptos_framework::ibe_config)]
    fun share_count_must_equal_weight(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 0, @0xB0, 0);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 14, location = aptos_framework::ibe_config)]
    fun unknown_validator_index_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 3, @0xB0, 0);
    }

    // ---- epoch binding ----

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 10, location = aptos_framework::ibe_config)]
    fun share_for_other_epoch_is_rejected(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 1);
    }

    // ---- one reveal, one-way status ----

    #[test(user = @0xA)]
    fun reveal_is_one_way_and_stores_key(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        let dk = fixtures::timelock_basic_reconstructed_dk();
        assert!(ibe_config::finalize_for_testing(id, dk, true), 1);
        assert!(ibe_config::get_status(id) == 1, 2);
        assert!(ibe_config::is_revealed(id), 3);
        assert!(!ibe_config::is_expired(id), 4);
        assert!(ibe_config::get_decryption_key(id) == dk, 5);
        assert!(ibe_config::get_expiry_reason(id) == 0, 6);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 11, location = aptos_framework::ibe_config)]
    fun second_reveal_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        let dk = fixtures::timelock_basic_reconstructed_dk();
        ibe_config::finalize_for_testing(id, dk, true);
        ibe_config::finalize_for_testing(id, dk, true);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 11, location = aptos_framework::ibe_config)]
    fun submit_after_reveal_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        ibe_config::finalize_for_testing(id, fixtures::timelock_basic_reconstructed_dk(), true);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 2, @0xB2, 0);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 12, location = aptos_framework::ibe_config)]
    fun finalize_below_threshold_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::finalize_for_testing(id, fixtures::timelock_basic_reconstructed_dk(), true);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 5, location = aptos_framework::ibe_config)]
    fun get_decryption_key_aborts_before_reveal(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        ibe_config::get_decryption_key(id);
    }

    #[test(user = @0xA)]
    fun failed_key_check_expires_and_never_reopens(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        assert!(!ibe_config::finalize_for_testing(id, x"00", false), 1);
        assert!(ibe_config::get_status(id) == 2, 2);
        assert!(ibe_config::get_expiry_reason(id) == 3, 3);
        assert!(ibe_config::is_expired(id), 4);
        assert!(!ibe_config::is_revealed(id), 5);
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 0, 6);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 11, location = aptos_framework::ibe_config)]
    fun no_retry_after_key_check_failure(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        ibe_config::finalize_for_testing(id, x"00", false);
        ibe_config::finalize_for_testing(id, fixtures::timelock_basic_reconstructed_dk(), true);
    }

    // ---- expiry ----

    #[test(user = @0xA)]
    fun deadline_passed_differs_from_expired(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        assert!(!ibe_config::is_deadline_passed(id), 1);
        timestamp::update_global_time_for_test(60 * MIN);
        assert!(ibe_config::is_deadline_passed(id), 2);
        assert!(!ibe_config::is_expired(id), 3);
        assert!(ibe_config::get_status(id) == 0, 4);
    }

    #[test(user = @0xA)]
    fun reveal_window_expiry_marks_expired_and_drops_shares(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);

        // Just inside the window: nothing expires.
        timestamp::update_global_time_for_test(70 * MIN - 1);
        ibe_config::on_new_block_for_testing();
        assert!(ibe_config::get_status(id) == 0, 1);

        // At the window end: expired.
        timestamp::update_global_time_for_test(70 * MIN);
        ibe_config::on_new_block_for_testing();
        assert!(ibe_config::get_status(id) == 2, 2);
        assert!(ibe_config::get_expiry_reason(id) == 2, 3);
        assert!(ibe_config::is_expired(id), 4);
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 0, 5);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 15, location = aptos_framework::ibe_config)]
    fun submit_after_reveal_window_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(70 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
    }

    #[test(user = @0xA)]
    fun revealed_timelock_is_not_expired_later(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        ibe_config::finalize_for_testing(id, fixtures::timelock_basic_reconstructed_dk(), true);
        timestamp::update_global_time_for_test(100 * MIN);
        ibe_config::on_new_block_for_testing();
        assert!(ibe_config::get_status(id) == 1, 1);
    }

    #[test(user = @0xA)]
    fun on_new_block_expires_in_bounded_batches(user: &signer) {
        setup_ready();
        let n = 60;
        let i = 0;
        while (i < n) {
            register(user, 60 * MIN);
            i += 1;
        };
        timestamp::update_global_time_for_test(70 * MIN);
        ibe_config::on_new_block_for_testing();
        // 50 per block: ids 0..49 expired, 50..59 pending.
        assert!(ibe_config::get_status(49) == 2, 1);
        assert!(ibe_config::get_status(50) == 0, 2);
        ibe_config::on_new_block_for_testing();
        assert!(ibe_config::get_status(59) == 2, 3);
    }

    // ---- epoch change ----

    #[test(user = @0xA)]
    fun epoch_change_expires_pending_with_epoch_ended(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        reconfiguration::reconfigure_for_test_custom();
        assert!(reconfiguration::current_epoch() == 1, 1);

        // Reads as expired at once. The stored shares go on the next block.
        assert!(!ibe_config::is_ready(), 2);
        assert!(ibe_config::get_status(id) == 2, 3);
        assert!(ibe_config::get_expiry_reason(id) == 1, 4);
        ibe_config::on_new_block_for_testing();
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 0, 5);
    }

    #[test(user = @0xA, framework = @0x1)]
    #[expected_failure(abort_code = 10, location = aptos_framework::ibe_config)]
    fun old_epoch_timelock_cannot_take_shares_in_new_epoch(user: &signer, framework: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        reconfiguration::reconfigure_for_test_custom();
        timestamp::update_global_time_for_test(61 * MIN);
        ibe_config::set_epoch_config_for_testing(
            framework, 1, fixtures::timelock_basic_mpk(), 3, vector[2, 1, 2], EPOCH_INTERVAL
        );
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 1);
    }

    // ---- handler support ----

    #[test(user = @0xA)]
    fun is_reconstructable_tracks_threshold_and_status(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        assert!(!ibe_config::is_reconstructable(id), 1);
        assert!(!ibe_config::is_reconstructable(id + 100), 2);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        assert!(!ibe_config::is_reconstructable(id), 3);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        assert!(ibe_config::is_reconstructable(id), 4);
        ibe_config::finalize_for_testing(id, fixtures::timelock_basic_reconstructed_dk(), true);
        assert!(!ibe_config::is_reconstructable(id), 5);
    }

    #[test(user = @0xA)]
    fun fail_key_check_expires_with_reason_and_never_reopens(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        ibe_config::fail_key_check_for_testing(id);
        assert!(ibe_config::get_status(id) == 2, 1);
        assert!(ibe_config::get_expiry_reason(id) == 3, 2);
        assert!(!ibe_config::is_reconstructable(id), 3);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 12, location = aptos_framework::ibe_config)]
    fun fail_key_check_below_threshold_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::fail_key_check_for_testing(id);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 11, location = aptos_framework::ibe_config)]
    fun fail_key_check_after_reveal_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        ibe_config::finalize_for_testing(id, fixtures::timelock_basic_reconstructed_dk(), true);
        ibe_config::fail_key_check_for_testing(id);
    }

    /// Refund gate for the auction contract: every expiry reason reads as status Expired with its own reason,
    /// and a revealed timelock never does.
    #[test(user = @0xA)]
    fun every_expiry_reason_is_visible_to_the_refund_path(user: &signer) {
        setup_ready();
        let key_failed = register(user, 60 * MIN);
        let window = register(user, 60 * MIN);
        let revealed = register(user, 60 * MIN);
        let epoch_ended = register(user, 100 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(key_failed, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(key_failed, shares(1), 1, @0xB1, 0);
        ibe_config::fail_key_check_for_testing(key_failed);
        ibe_config::submit_dk_shares_for_testing(revealed, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(revealed, shares(1), 1, @0xB1, 0);
        ibe_config::finalize_for_testing(revealed, fixtures::timelock_basic_reconstructed_dk(), true);
        timestamp::update_global_time_for_test(71 * MIN);
        ibe_config::on_new_block_for_testing();
        assert!(ibe_config::get_expiry_reason(window) == 2, 1);
        assert!(ibe_config::get_status(window) == 2, 2);
        assert!(ibe_config::get_expiry_reason(key_failed) == 3, 3);
        assert!(ibe_config::get_status(revealed) == 1, 4);
        assert!(ibe_config::get_expiry_reason(revealed) == 0, 5);
        reconfiguration::reconfigure_for_test_custom();
        assert!(ibe_config::get_expiry_reason(epoch_ended) == 1, 6);
        assert!(ibe_config::get_status(epoch_ended) == 2, 7);
    }

    // ---- verification keys and rejected shares ----

    fun keys(n: u64): vector<vector<u8>> {
        let out = vector[];
        let i = 0;
        while (i < n) {
            let vk = vector[];
            let b = 0;
            while (b < 96) {
                vk.push_back((i as u8));
                b += 1;
            };
            out.push_back(vk);
            i += 1;
        };
        out
    }

    #[test]
    fun player_keys_are_sliced_by_weight() {
        let framework = setup();
        ibe_config::set_epoch_config_with_keys_for_testing(
            &framework, 0, fixtures::timelock_basic_mpk(), 3, vector[2, 1, 2], EPOCH_INTERVAL, keys(5)
        );
        let k0 = ibe_config::get_player_keys_for_testing(0);
        let k1 = ibe_config::get_player_keys_for_testing(1);
        let k2 = ibe_config::get_player_keys_for_testing(2);
        assert!(k0.length() == 2 && k1.length() == 1 && k2.length() == 2, 1);
        assert!(k0[0][0] == 0 && k0[1][0] == 1, 2);
        assert!(k1[0][0] == 2, 3);
        assert!(k2[0][0] == 3 && k2[1][0] == 4, 4);
    }

    #[test]
    #[expected_failure(abort_code = 14, location = aptos_framework::ibe_config)]
    fun player_keys_of_unknown_validator_abort() {
        let framework = setup();
        ibe_config::set_epoch_config_with_keys_for_testing(
            &framework, 0, fixtures::timelock_basic_mpk(), 3, vector[2, 1, 2], EPOCH_INTERVAL, keys(5)
        );
        ibe_config::get_player_keys_for_testing(3);
    }

    #[test]
    #[expected_failure(abort_code = 16, location = aptos_framework::ibe_config)]
    fun epoch_config_with_wrong_key_count_aborts() {
        let framework = setup();
        ibe_config::set_epoch_config_with_keys_for_testing(
            &framework, 0, fixtures::timelock_basic_mpk(), 3, vector[2, 1, 2], EPOCH_INTERVAL, keys(4)
        );
    }

    #[test]
    #[expected_failure(abort_code = 16, location = aptos_framework::ibe_config)]
    fun epoch_config_with_short_key_aborts() {
        let framework = setup();
        let bad = keys(5);
        vector::borrow_mut(&mut bad, 2).pop_back();
        ibe_config::set_epoch_config_with_keys_for_testing(
            &framework, 0, fixtures::timelock_basic_mpk(), 3, vector[2, 1, 2], EPOCH_INTERVAL, bad
        );
    }

    #[test(user = @0xA)]
    fun rejected_share_emits_event_and_keeps_the_slot_free(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::report_rejected_share_for_testing(id, 0, @0xB0, 0);
        let events = event::emitted_events<ibe_config::TimelockShareRejectedEvent>();
        assert!(events.length() == 1, 1);
        // Nothing was stored.
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 0, 2);
        // The same validator can still submit.
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 2, 3);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 3, location = aptos_framework::ibe_config)]
    fun rejected_share_before_deadline_emits_nothing(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        ibe_config::report_rejected_share_for_testing(id, 0, @0xB0, 0);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 7, location = aptos_framework::ibe_config)]
    fun rejected_share_after_accepted_submission_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::report_rejected_share_for_testing(id, 0, @0xB0, 0);
    }
}
