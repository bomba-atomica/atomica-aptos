#[test_only]
/// Registry rules of `ibe_config`.
///
/// `finalize_for_testing` runs the real pairing check of `finalize_timelock_reveal`. `setup_ready` sets
/// the master public key `s * g2` for a known test secret `s`, and `dk_of` derives the real key
/// `s * H1(identity)` of a registered timelock with `crypto_algebra`.
module aptos_framework::ibe_config_tests {
    use std::features;
    use std::option;
    use std::vector;
    use aptos_std::bls12381_algebra::{Fr, G1, G2, FormatG1Compr, FormatG2Compr, HashG1XmdSha256SswuRo};
    use aptos_std::crypto_algebra::{Self, Element};
    use aptos_framework::account;
    use aptos_framework::event;
    use aptos_framework::ibe_config;
    use aptos_framework::ibe_golden_vector_fixtures as fixtures;
    use aptos_framework::reconfiguration;
    use aptos_framework::timestamp;

    const MIN: u64 = 60_000_000;
    const EPOCH_INTERVAL: u64 = 7_200_000_000; // 2 hours
    const H1_DST: vector<u8> = b"ATOMICA_TIMELOCK_IBE_V2_H1";
    /// Master secret of the test key pair. Test only: it is public.
    const TEST_MASTER_SECRET: u64 = 0xA70A;

    /// The registry with no registrar set.
    fun setup_without_registrar(): signer {
        let framework = account::create_signer_for_test(@0x1);
        features::change_feature_flags_for_testing(
            &framework, vector[features::get_bls12_381_strutures_feature()], vector[]
        );
        ibe_config::initialize_for_testing(&framework);
        framework
    }

    /// The registry with `@0xA` as registrar. Every test that registers signs as `@0xA`.
    fun setup(): signer {
        let framework = setup_without_registrar();
        ibe_config::set_timelock_registrar(&framework, @0xA);
        framework
    }

    fun master_secret(): Element<Fr> {
        crypto_algebra::from_u64<Fr>(TEST_MASTER_SECRET)
    }

    /// `s * g2` for the test secret `s`.
    fun test_mpk(): vector<u8> {
        let mpk = crypto_algebra::scalar_mul(&crypto_algebra::one<G2>(), &master_secret());
        crypto_algebra::serialize<G2, FormatG2Compr>(&mpk)
    }

    /// The real decryption key of timelock `id` under `test_mpk`: `s * H1(identity)`.
    fun dk_of(id: u64): vector<u8> {
        let q = crypto_algebra::hash_to<G1, HashG1XmdSha256SswuRo>(&H1_DST, &ibe_config::get_identity(id));
        crypto_algebra::serialize<G1, FormatG1Compr>(&crypto_algebra::scalar_mul(&q, &master_secret()))
    }

    /// Epoch 0 config with `test_mpk`, weights [2, 1, 2], threshold 3, and time 0 as the epoch start.
    fun setup_ready(): signer {
        let framework = setup();
        ibe_config::set_epoch_config_for_testing(
            &framework, 0, test_mpk(), 3, vector[2, 1, 2], EPOCH_INTERVAL
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

    /// The only `TimelockExpiredEvent` emitted so far, as `(timelock_id, reason, submitters)`.
    fun only_expired_event(): (u64, u8, vector<address>) {
        let events = event::emitted_events<ibe_config::TimelockExpiredEvent>();
        assert!(events.length() == 1, 100);
        ibe_config::expired_event_fields_for_testing(&events[0])
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
        assert!(ibe_config::get_mpk() == test_mpk(), 2);
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

    // ---- registrar ----

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 17, location = aptos_framework::ibe_config)]
    fun register_aborts_with_not_registrar_while_no_registrar_is_set(user: &signer) {
        let framework = setup_without_registrar();
        ibe_config::set_epoch_config_for_testing(
            &framework, 0, test_mpk(), 3, vector[2, 1, 2], EPOCH_INTERVAL
        );
        assert!(ibe_config::is_ready(), 1);
        assert!(ibe_config::timelock_registrar().is_none(), 2);
        register(user, 60 * MIN);
    }

    #[test(user = @0xA, other = @0xB)]
    #[expected_failure(abort_code = 17, location = aptos_framework::ibe_config)]
    fun register_aborts_for_another_signer_and_succeeds_for_the_registrar(user: &signer, other: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        assert!(id == 0, 1);
        assert!(ibe_config::get_status(id) == 0, 2);
        register(other, 60 * MIN);
    }

    #[test(user = @0xA)]
    fun register_with_id_returns_the_next_id_and_the_event_carries_it(user: &signer) {
        setup_ready();
        register(user, 60 * MIN);
        let expected = ibe_config::get_next_timelock_id();
        let id = ibe_config::register_timelock_with_id(user, 60 * MIN);
        assert!(id == expected && id == 1, 1);
        assert!(ibe_config::get_next_timelock_id() == id + 1, 2);
        let events = event::emitted_events<ibe_config::TimelockRegistrationEvent>();
        assert!(events.length() == 2, 3);
        let (event_id, sender) = ibe_config::registration_event_fields_for_testing(&events[1]);
        assert!(event_id == id && sender == @0xA, 4);
    }

    #[test(framework = @0x1, other = @0xB)]
    #[expected_failure(abort_code = 0x50003, location = aptos_framework::system_addresses)]
    fun set_registrar_is_framework_only_and_a_second_call_replaces_the_address(framework: &signer, other: &signer) {
        setup_without_registrar();
        assert!(ibe_config::timelock_registrar().is_none(), 1);
        ibe_config::set_timelock_registrar(framework, @0xA);
        assert!(ibe_config::timelock_registrar() == option::some(@0xA), 2);
        ibe_config::set_timelock_registrar(framework, @0xB);
        assert!(ibe_config::timelock_registrar() == option::some(@0xB), 3);
        ibe_config::set_timelock_registrar(other, @0xB);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 2, location = aptos_framework::ibe_config)]
    /// `setup` made `@0xA` the registrar, so the readiness check (2) decides, not the registrar check (17).
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
        let dk = dk_of(id);
        assert!(ibe_config::finalize_for_testing(id, dk), 1);
        assert!(ibe_config::get_status(id) == 1, 2);
        assert!(ibe_config::is_revealed(id), 3);
        assert!(!ibe_config::is_expired(id), 4);
        assert!(ibe_config::get_decryption_key(id) == dk, 5);
        assert!(ibe_config::get_expiry_reason(id) == 0, 6);
    }

    /// Defect found by the e2e run: with fewer submitters than validators the inputs of the Rust
    /// reconstruction listed only the weights of the submitters, so the weight sum differed from the total
    /// weight and every reveal failed with `key-check-failed`. The weights must be those of all validators.
    #[test(user = @0xA)]
    fun reconstruction_inputs_carry_the_weights_of_all_validators(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        // Validators 0 (weight 2) and 2 (weight 2) submit, validator 1 does not. Threshold is 3.
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 2, @0xB2, 0);
        let (indices, dk_shares, weights, threshold, total_weight, _identity) =
            ibe_config::get_reconstruction_inputs(id);
        assert!(indices == vector[0, 2], 1);
        assert!(dk_shares.length() == 2, 2);
        assert!(weights == vector[2, 1, 2], 3);
        assert!(threshold == 3, 4);
        assert!(total_weight == 5, 5);
    }

    /// Submissions out of validator-index order keep their submission order in `indices`, while
    /// `weights` stays the full per-validator list in validator order.
    #[test(user = @0xA)]
    fun reconstruction_inputs_out_of_index_order_carry_all_weights(user: &signer) {
        let framework = setup();
        ibe_config::set_epoch_config_for_testing(
            &framework, 0, fixtures::timelock_basic_mpk(), 3, vector[1, 1, 1, 1], EPOCH_INTERVAL
        );
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 2, @0xB2, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 0, @0xB0, 0);
        assert!(!ibe_config::is_reconstructable(id), 1);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 3, @0xB3, 0);
        assert!(ibe_config::is_reconstructable(id), 2);
        let (indices, dk_shares, weights, threshold, total_weight, _identity) =
            ibe_config::get_reconstruction_inputs(id);
        assert!(indices == vector[2, 0, 3], 3);
        assert!(dk_shares.length() == 3, 4);
        assert!(weights == vector[1, 1, 1, 1], 5);
        assert!(threshold == 3, 6);
        assert!(total_weight == 4, 7);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 11, location = aptos_framework::ibe_config)]
    fun second_reveal_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        let dk = dk_of(id);
        ibe_config::finalize_for_testing(id, dk);
        ibe_config::finalize_for_testing(id, dk);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 11, location = aptos_framework::ibe_config)]
    fun submit_after_reveal_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        ibe_config::finalize_for_testing(id, dk_of(id));
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 2, @0xB2, 0);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 12, location = aptos_framework::ibe_config)]
    fun finalize_below_threshold_aborts(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::finalize_for_testing(id, dk_of(id));
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
        assert!(!ibe_config::finalize_for_testing(id, x"00"), 1);
        assert!(ibe_config::get_status(id) == 2, 2);
        assert!(ibe_config::get_expiry_reason(id) == 3, 3);
        assert!(ibe_config::is_expired(id), 4);
        assert!(!ibe_config::is_revealed(id), 5);
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 0, 6);
        assert!(ibe_config::get_submitters(id) == vector[@0xB0, @0xB1], 7);
        let (event_id, reason, submitters) = only_expired_event();
        assert!(event_id == id && reason == 3 && submitters == vector[@0xB0, @0xB1], 8);
    }

    /// A valid G1 point that is the key of another timelock fails the real pairing check.
    #[test(user = @0xA)]
    fun finalize_runs_the_real_pairing_check(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        let other = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        assert!(!ibe_config::finalize_for_testing(id, dk_of(other)), 1);
        assert!(ibe_config::get_status(id) == 2, 2);
        assert!(ibe_config::get_expiry_reason(id) == 3, 3);
    }

    #[test(user = @0xA)]
    #[expected_failure(abort_code = 11, location = aptos_framework::ibe_config)]
    fun no_retry_after_key_check_failure(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        ibe_config::finalize_for_testing(id, x"00");
        ibe_config::finalize_for_testing(id, dk_of(id));
    }

    // ---- expiry ----

    /// Read rule: an unrevealed timelock reads expired from its reveal expiry on, before the queue runs.
    #[test(user = @0xA)]
    fun status_reads_expired_at_reveal_expiry_without_running_the_queue(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(70 * MIN);
        assert!(ibe_config::get_status(id) == 2, 1);
        assert!(ibe_config::is_expired(id), 2);
        assert!(ibe_config::get_expiry_reason(id) == 2, 3);
        // The queue has not run: no event yet.
        assert!(event::emitted_events<ibe_config::TimelockExpiredEvent>().is_empty(), 4);
    }

    #[test(user = @0xA)]
    fun status_reads_pending_one_microsecond_before_reveal_expiry(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(70 * MIN - 1);
        assert!(ibe_config::get_status(id) == 0, 1);
        assert!(!ibe_config::is_expired(id), 2);
        assert!(ibe_config::get_expiry_reason(id) == 0, 3);
    }

    /// The queue drains 50 per block. The 51st entry is not drained and still reads expired with reason 2.
    #[test(user = @0xA)]
    fun undrained_51st_timelock_reads_expired_with_reason_2(user: &signer) {
        setup_ready();
        let i = 0;
        while (i < 51) {
            register(user, 60 * MIN);
            i += 1;
        };
        timestamp::update_global_time_for_test(70 * MIN);
        ibe_config::on_new_block_for_testing();
        assert!(event::emitted_events<ibe_config::TimelockExpiredEvent>().length() == 50, 1);
        assert!(ibe_config::get_status(50) == 2, 2);
        assert!(ibe_config::get_expiry_reason(50) == 2, 3);
        assert!(ibe_config::is_expired(50), 4);
    }

    #[test(user = @0xA)]
    fun revealed_timelock_never_reads_expired_after_reveal_expiry(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        ibe_config::finalize_for_testing(id, dk_of(id));
        timestamp::update_global_time_for_test(70 * MIN);
        assert!(ibe_config::get_status(id) == 1, 1);
        assert!(!ibe_config::is_expired(id), 2);
        assert!(ibe_config::get_expiry_reason(id) == 0, 3);
    }

    /// The epoch rule keeps priority over the reveal window read rule.
    #[test(user = @0xA)]
    fun earlier_epoch_pending_reads_epoch_ended_even_past_reveal_expiry(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(71 * MIN);
        reconfiguration::reconfigure_for_test_custom();
        assert!(reconfiguration::current_epoch() == 1, 1);
        assert!(ibe_config::get_status(id) == 2, 2);
        assert!(ibe_config::get_expiry_reason(id) == 1, 3);
    }

    #[test(user = @0xA)]
    fun is_reconstructable_is_false_at_reveal_expiry_with_threshold_weight(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        assert!(ibe_config::is_reconstructable(id), 1);
        timestamp::update_global_time_for_test(70 * MIN);
        // Threshold weight is still stored, because the queue has not run.
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 3, 2);
        assert!(!ibe_config::is_reconstructable(id), 3);
    }

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

    /// `get_status` reads 2 at the window end before the queue runs (read rule), so this test checks the
    /// queue through the dropped shares and the single `TimelockExpiredEvent`.
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

        // At the window end: reads expired at once. The shares stay until the queue runs.
        timestamp::update_global_time_for_test(70 * MIN);
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 2, 8);
        assert!(event::emitted_events<ibe_config::TimelockExpiredEvent>().is_empty(), 9);
        ibe_config::on_new_block_for_testing();
        assert!(ibe_config::get_status(id) == 2, 2);
        assert!(ibe_config::get_expiry_reason(id) == 2, 3);
        assert!(ibe_config::is_expired(id), 4);
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 0, 5);
        assert!(ibe_config::get_submitters(id) == vector[@0xB0], 6);
        let (event_id, reason, submitters) = only_expired_event();
        assert!(event_id == id && reason == 2 && submitters == vector[@0xB0], 7);
    }

    /// The list keeps submission order, not validator index order.
    #[test(user = @0xA)]
    fun expired_by_reveal_window_keeps_and_emits_two_submitters_in_submission_order(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 2, @0xB2, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 0, @0xB0, 0);
        timestamp::update_global_time_for_test(70 * MIN);
        ibe_config::on_new_block_for_testing();
        assert!(ibe_config::get_expiry_reason(id) == 2, 1);
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 0, 2);
        let kept = ibe_config::get_submitters(id);
        assert!(kept == vector[@0xB2, @0xB0], 3);
        let (event_id, reason, submitters) = only_expired_event();
        assert!(event_id == id && reason == 2, 4);
        assert!(submitters == kept, 5);
    }

    #[test(user = @0xA)]
    fun expired_by_epoch_end_keeps_and_emits_submitters(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        reconfiguration::reconfigure_for_test_custom();
        ibe_config::on_new_block_for_testing();
        assert!(ibe_config::get_expiry_reason(id) == 1, 1);
        assert!(ibe_config::get_submitters(id) == vector[@0xB1], 2);
        let (event_id, reason, submitters) = only_expired_event();
        assert!(event_id == id && reason == 1 && submitters == vector[@0xB1], 3);
    }

    #[test(user = @0xA)]
    fun expired_by_key_check_keeps_and_emits_submitters(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(60 * MIN);
        ibe_config::submit_dk_shares_for_testing(id, shares(1), 1, @0xB1, 0);
        ibe_config::submit_dk_shares_for_testing(id, shares(2), 2, @0xB2, 0);
        ibe_config::fail_key_check_for_testing(id);
        assert!(ibe_config::get_expiry_reason(id) == 3, 1);
        let (_, _, _, count) = ibe_config::get_timelock(id);
        assert!(count == 0, 2);
        assert!(ibe_config::get_submitters(id) == vector[@0xB1, @0xB2], 3);
        let (event_id, reason, submitters) = only_expired_event();
        assert!(event_id == id && reason == 3 && submitters == vector[@0xB1, @0xB2], 4);
    }

    #[test(user = @0xA)]
    fun expired_with_no_accepted_share_has_an_empty_list_and_an_empty_event_list(user: &signer) {
        setup_ready();
        let id = register(user, 60 * MIN);
        timestamp::update_global_time_for_test(70 * MIN);
        ibe_config::on_new_block_for_testing();
        assert!(ibe_config::get_expiry_reason(id) == 2, 1);
        assert!(ibe_config::get_submitters(id).is_empty(), 2);
        let (event_id, reason, submitters) = only_expired_event();
        assert!(event_id == id && reason == 2 && submitters.is_empty(), 3);
    }

    // Share submission reads the stored status, not the read rule, so a share past the window aborts with
    // E_REVEAL_WINDOW_ELAPSED (15) and not E_NOT_PENDING (11).
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
        ibe_config::finalize_for_testing(id, dk_of(id));
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
        // 50 per block: the first run emits 50 events. Every id reads 2 by the read rule, so the bound is
        // counted through the events, not through get_status.
        assert!(event::emitted_events<ibe_config::TimelockExpiredEvent>().length() == 50, 1);
        ibe_config::on_new_block_for_testing();
        assert!(event::emitted_events<ibe_config::TimelockExpiredEvent>().length() == 60, 2);
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
        assert!(ibe_config::get_submitters(id) == vector[@0xB0], 6);
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
        ibe_config::finalize_for_testing(id, dk_of(id));
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
        ibe_config::finalize_for_testing(id, dk_of(id));
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
        ibe_config::finalize_for_testing(revealed, dk_of(revealed));
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
