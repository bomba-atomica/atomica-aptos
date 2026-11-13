// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_framework::{extended_checks, path_in_crate, BuildOptions};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::on_chain_config::{
    aptos_test_feature_flags_genesis, Features, TimedFeaturesBuilder,
};
use aptos_vm::natives;
use move_model::model::GlobalEnv;
use move_package::CompilerConfig;
use move_unit_test::{
    package_test::{run_move_unit_tests, UnitTestResult},
    test_validation, UnitTestingConfig,
};
use move_vm_runtime::native_functions::NativeFunctionTable;
use tempfile::tempdir;

/// Configures the unit test validation hook to run the extended checker.
fn configure_extended_checks_for_unit_test() {
    fn validate(env: &GlobalEnv) {
        extended_checks::run_extended_checks(env);
    }
    test_validation::set_validation_hook(Box::new(validate));
}

fn run_tests_for_pkg(path_to_pkg: impl Into<String>, use_latest_language: bool) {
    run_tests_for_pkg_with_filter(
        path_to_pkg,
        use_latest_language,
        std::env::var("TEST_FILTER").ok(),
        &mut std::io::stdout(),
    );
}

fn run_tests_for_pkg_with_filter(
    path_to_pkg: impl Into<String>,
    use_latest_language: bool,
    filter: Option<String>,
    output: &mut (impl std::io::Write + Send),
) {
    let pkg_path = path_in_crate(path_to_pkg);
    let compiler_config = CompilerConfig {
        known_attributes: extended_checks::get_all_attribute_names().clone(),
        print_errors: Some(true),
        ..Default::default()
    };
    let mut build_config = move_package::BuildConfig {
        test_mode: true,
        install_dir: Some(tempdir().unwrap().path().to_path_buf()),
        compiler_config: compiler_config.clone(),
        full_model_generation: true, // Run extended checks also on test code
        ..Default::default()
    };

    if use_latest_language {
        let latest_build_options = BuildOptions::default().set_latest_language();
        build_config.compiler_config.bytecode_version = latest_build_options.bytecode_version;
        build_config.compiler_config.language_version = latest_build_options.language_version;
    }

    let utc = UnitTestingConfig {
        filter,
        report_statistics: matches!(std::env::var("REPORT_STATS"), Ok(s) if s.as_str() == "1"),
        ..Default::default()
    };
    let ok = run_move_unit_tests(
        &pkg_path,
        build_config.clone(),
        // TODO(Gas): double check if this is correct
        utc,
        aptos_test_natives(),
        aptos_test_feature_flags_genesis(),
        /* gas limit */ Some(100_000),
        /* cost_table */ None,
        /* compute_coverage */ false,
        output,
        true,
    );
    match ok {
        Err(e) => panic!("move unit tests failed:\n{:#}", e),
        Ok(UnitTestResult::Failure) => panic!("move unit tests failed"),
        Ok(UnitTestResult::Success) => {},
    }
}

/// TODO: per @vgao1996:
/// - There should be only one ground truth of `aptos_test_natives`.
///   But rn it's defined here, in `move-examples` and in `framework-experimental`.
/// - This function updates a global config (in `configure_extended_checks_for_unit_test`)
///   then returns a list natives. This pattern is confusing.
/// More discussion: https://github.com/aptos-labs/aptos-core/pull/15997#discussion_r1994469668
pub fn aptos_test_natives() -> NativeFunctionTable {
    // By side effect, configure for unit tests
    natives::configure_for_unit_test();
    configure_extended_checks_for_unit_test();
    // move_stdlib has the testing feature enabled to include debug native functions
    natives::aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    )
}

#[test]
fn move_framework_unit_tests() {
    run_tests_for_pkg("aptos-framework", false);
}

#[test]
fn move_aptos_stdlib_unit_tests() {
    run_tests_for_pkg("aptos-stdlib", false);
}

#[test]
fn move_stdlib_unit_tests() {
    run_tests_for_pkg("move-stdlib", false);
}

#[test]
fn move_token_unit_tests() {
    run_tests_for_pkg("aptos-token", false);
}

#[test]
fn move_token_objects_unit_tests() {
    run_tests_for_pkg("aptos-token-objects", false);
}

#[test]
fn move_trading_unit_tests() {
    run_tests_for_pkg("aptos-trading", false);
}

#[test]
fn move_experimental_unit_tests() {
    run_tests_for_pkg("aptos-experimental", true);
}

// SCOUT(#227) TEST: a filtered Rust test for the Move tests of `ibe_native_test` and `ibe_golden_vector_tests`. One filter string cannot select both without `ibe_config_tests`, and the cargo filter `ibe` would also run ibe_config_move_unit_tests, so pick the names with care.
/// Runs the aptos-framework Move tests that match `filter` and returns how many passed. The report is
/// copied to stdout as it is written, so a failure still shows it.
fn run_filtered_framework_tests(filter: &str) -> u64 {
    struct Tee(Vec<u8>);
    impl std::io::Write for Tee {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            print!("{}", String::from_utf8_lossy(buf));
            self.0.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut tee = Tee(Vec::new());
    run_tests_for_pkg_with_filter("aptos-framework", false, Some(filter.to_string()), &mut tee);
    let out = String::from_utf8(tee.0).unwrap();
    out.split("passed: ")
        .nth(1)
        .and_then(|rest| rest.split(';').next())
        .and_then(|n| n.trim().parse().ok())
        .expect("Move test summary line")
}

/// The Move unit tests of `ethereum_headers` and `ethereum_header_config` (fork only). Runs
/// with `cargo test -p aptos-framework --test move_unit_test -- ethereum_headers` and fails when the filter
/// matches no Move test, so the Rust test never passes with zero Move tests.
#[test]
fn ethereum_headers_move_unit_tests() {
    let passed = run_filtered_framework_tests("ethereum_headers_tests");
    assert!(passed >= 8, "only {passed} ethereum_headers Move tests ran");
}

/// The Move unit tests of `ibe_config_tests` (fork only). Runs with
/// `cargo test -p aptos-framework --test move_unit_test -- ibe_config` and fails when fewer Move tests
/// pass than the file holds.
#[test]
fn ibe_config_move_unit_tests() {
    let passed = run_filtered_framework_tests("ibe_config_tests");
    assert!(passed >= 60, "only {passed} ibe_config Move tests ran");
}

/// The Move unit tests of `timelock_dkg_tests` (fork only), including the `TimelockKeyMissingEvent`
/// tests (atomica issue 231). Runs with `cargo test -p aptos-framework --test move_unit_test --
/// timelock_dkg` and fails when fewer Move tests pass than the file holds.
#[test]
fn timelock_dkg_move_unit_tests() {
    let passed = run_filtered_framework_tests("timelock_dkg_tests");
    assert!(passed >= 16, "only {passed} timelock_dkg Move tests ran");
}
