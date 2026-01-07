// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::blstrs::{multi_pairing, random_scalar};
use aptos_dkg::{
    algebra::polynomials::{
        poly_eval, poly_mul_fft, poly_mul_less_slow, poly_mul_slow, poly_xnmul,
    },
    utils::{
        g1_multi_exp, g2_multi_exp, parallel_multi_pairing,
        random::{random_g1_point, random_g2_point, random_scalars},
    },
    weighted_vuf::pinkas::MIN_MULTIPAIR_NUM_JOBS,
};
use aptos_runtimes::spawn_rayon_thread_pool;
use blstrs::{G1Projective, G2Projective, Scalar};
use ff::Field;
use group::Group;
use rand::thread_rng;
use std::ops::Mul;

/// Tests that g1_multi_exp wrapper correctly handles edge cases that trigger blstrs bugs.
///
/// The wrapper function should:
/// 1. Panic when bases.len() != scalars.len() (prevents blstrs heisenbugs)
/// 2. Return identity for empty arrays
/// 3. Use scalar multiplication for single elements (prevents generator bugs)
/// 4. Delegate to blstrs for multiple elements
#[test]
fn test_g1_multi_exp_wrapper_safety() {
    let mut rng = thread_rng();

    // Test 1: Mismatched sizes should panic
    let result = std::panic::catch_unwind(|| {
        g1_multi_exp(
            &[G1Projective::identity(), G1Projective::identity()],
            &[Scalar::ONE],
        );
    });
    assert!(
        result.is_err(),
        "Should panic when bases.len() != scalars.len()"
    );

    let result = std::panic::catch_unwind(|| {
        g1_multi_exp(&[G1Projective::identity()], &[Scalar::ONE, Scalar::ONE]);
    });
    assert!(
        result.is_err(),
        "Should panic when bases.len() != scalars.len()"
    );

    // Test 2: Empty arrays should return identity
    let result = g1_multi_exp(&[], &[]);
    assert_eq!(
        result,
        G1Projective::identity(),
        "Empty arrays should return identity"
    );

    // Test 3: Single element with identity base
    let result = g1_multi_exp(&[G1Projective::identity()], &[Scalar::ONE]);
    assert_eq!(
        result,
        G1Projective::identity(),
        "Identity * 1 should equal identity"
    );

    // Test 4: Single element with random base
    let base = random_g1_point(&mut rng);
    let scalar = Scalar::ONE;
    let result = g1_multi_exp(&[base], &[scalar]);
    assert_eq!(result, base, "Random point * 1 should equal the point");

    // Test 5: Single element with random scalar (this is the bug case for generator)
    let result = g1_multi_exp(&[G1Projective::generator()], &[Scalar::ONE]);
    assert_eq!(
        result,
        G1Projective::generator(),
        "Generator * 1 should equal generator (wrapper should avoid blstrs bug)"
    );

    // Test 6: Multiple elements (should delegate to blstrs safely)
    let bases = vec![
        random_g1_point(&mut rng),
        random_g1_point(&mut rng),
        random_g1_point(&mut rng),
    ];
    let scalars = vec![Scalar::ONE, Scalar::ONE, Scalar::ONE];
    let result = g1_multi_exp(&bases, &scalars);
    let expected = bases[0] + bases[1] + bases[2];
    assert_eq!(result, expected, "Multi-element should compute correctly");
}

/// Tests that g2_multi_exp wrapper correctly handles edge cases that trigger blstrs bugs.
///
/// The wrapper function should:
/// 1. Panic when bases.len() != scalars.len() (prevents blstrs heisenbugs)
/// 2. Return identity for empty arrays
/// 3. Use scalar multiplication for single elements (prevents generator bugs)
/// 4. Delegate to blstrs for multiple elements
#[test]
fn test_g2_multi_exp_wrapper_safety() {
    let mut rng = thread_rng();

    // Test 1: Mismatched sizes should panic
    let result = std::panic::catch_unwind(|| {
        g2_multi_exp(
            &[G2Projective::identity(), G2Projective::identity()],
            &[Scalar::ONE],
        );
    });
    assert!(
        result.is_err(),
        "Should panic when bases.len() != scalars.len()"
    );

    let result = std::panic::catch_unwind(|| {
        g2_multi_exp(&[G2Projective::identity()], &[Scalar::ONE, Scalar::ONE]);
    });
    assert!(
        result.is_err(),
        "Should panic when bases.len() != scalars.len()"
    );

    // Test 2: Empty arrays should return identity
    let result = g2_multi_exp(&[], &[]);
    assert_eq!(
        result,
        G2Projective::identity(),
        "Empty arrays should return identity"
    );

    // Test 3: Single element with identity base
    let result = g2_multi_exp(&[G2Projective::identity()], &[Scalar::ONE]);
    assert_eq!(
        result,
        G2Projective::identity(),
        "Identity * 1 should equal identity"
    );

    // Test 4: Single element with random base
    let base = random_g2_point(&mut rng);
    let scalar = Scalar::ONE;
    let result = g2_multi_exp(&[base], &[scalar]);
    assert_eq!(result, base, "Random point * 1 should equal the point");

    // Test 5: Single element with generator (this is the documented bug case)
    let result = g2_multi_exp(&[G2Projective::generator()], &[Scalar::ONE]);
    assert_eq!(
        result,
        G2Projective::generator(),
        "Generator * 1 should equal generator (wrapper should avoid blstrs bug)"
    );

    // Test 6: Multiple elements (should delegate to blstrs safely)
    let bases = vec![
        random_g2_point(&mut rng),
        random_g2_point(&mut rng),
        random_g2_point(&mut rng),
    ];
    let scalars = vec![Scalar::ONE, Scalar::ONE, Scalar::ONE];
    let result = g2_multi_exp(&bases, &scalars);
    let expected = bases[0] + bases[1] + bases[2];
    assert_eq!(result, expected, "Multi-element should compute correctly");
}

/// At some point I suspected that size-1 multiexps where the scalar is set to 1 had a bug in them.
/// But they seem fine.
#[test]
fn test_crypto_size_1_multiexp_random_base() {
    let mut rng = thread_rng();

    let bases = vec![random_g2_point(&mut rng)];
    let scalars = vec![Scalar::ONE];

    let result = G2Projective::multi_exp(&bases, &scalars);

    assert_eq!(result, bases[0]);
}

/// Size-1 G1 multiexps on the generator where the scalar is set to one do NOT seem to be buggy.
#[test]
fn test_crypto_g_1_to_zero_multiexp() {
    let generator = G1Projective::generator();
    let result = G1Projective::multi_exp([generator].as_slice(), [Scalar::ONE].as_slice());

    assert_eq!(result, generator);
}

#[test]
fn test_crypto_poly_multiply() {
    let mut rng = thread_rng();
    for num_coeffs_f in [1, 2, 3, 4, 5, 6, 7, 8] {
        for num_coeffs_g in [1, 2, 3, 4, 5, 6, 7, 8] {
            let f = random_scalars(num_coeffs_f, &mut rng);
            let g = random_scalars(num_coeffs_g, &mut rng);

            // FFT-based multiplication
            let fft_fg = poly_mul_fft(&f, &g);

            // Naive multiplication
            let naive_fg = poly_mul_slow(&f, &g);

            // We test correctness of $h(X) = f(X) \cdot g(X)$ by picking a random point $r$ and
            // comparing $h(r)$ with $f(r) \cdot g(r)$.
            let r = random_scalar(&mut rng);

            let fg_rand = poly_eval(&f, &r).mul(poly_eval(&g, &r));
            let fft_fg_rand = poly_eval(&fft_fg, &r);
            assert_eq!(fft_fg_rand, fg_rand);

            // We also test correctness of the naive multiplication algorithm
            let naive_fg_rand = poly_eval(&naive_fg, &r);
            assert_eq!(naive_fg_rand, fg_rand);

            // Lastly, of course the naive result should be the same as the FFT result (since they are both correct)
            assert_eq!(naive_fg, fft_fg);
        }
    }
}

#[test]
fn test_crypto_poly_multiply_divide_and_conquer() {
    let mut rng = thread_rng();
    for log_n in [1, 2, 3, 4, 5, 6, 7, 8] {
        let n = 1 << log_n;
        let f = random_scalars(n, &mut rng);
        let g = random_scalars(n, &mut rng);

        let fg = poly_mul_less_slow(&f, &g);

        // FFT-based multiplication
        let fft_fg = poly_mul_fft(&f, &g);
        assert_eq!(fg, fft_fg);

        // Schwartz-Zippel test
        let r = random_scalar(&mut rng);
        let fg_rand = poly_eval(&f, &r).mul(poly_eval(&g, &r));
        let our_fg_rand = poly_eval(&fg, &r);
        assert_eq!(our_fg_rand, fg_rand);
    }
}

#[test]
#[allow(non_snake_case)]
fn test_crypto_poly_shift() {
    let mut rng = thread_rng();
    for num_coeffs_f in [1, 2, 3, 4, 5, 6, 7, 8] {
        for n in 0..16 {
            // compute the coefficients of X^n
            let mut Xn = Vec::with_capacity(n + 1);
            Xn.resize(n + 1, Scalar::ZERO);
            Xn[n] = Scalar::ONE;

            // pick a random f
            let f = random_scalars(num_coeffs_f, &mut rng);

            // f(X) * X^n via shift
            let shifted1 = poly_xnmul(&f, n);
            // f(X) * X^n via multiplication
            let shifted2 = poly_mul_fft(&f, &Xn);

            assert_eq!(shifted1, shifted2);
        }
    }
}

#[test]
fn test_parallel_multi_pairing() {
    let mut rng = thread_rng();

    let r1 = [random_g1_point(&mut rng), random_g1_point(&mut rng)];
    let r2 = [random_g2_point(&mut rng), random_g2_point(&mut rng)];

    let pool1 = spawn_rayon_thread_pool("testmultpair".to_string(), Some(1));
    let pool32 = spawn_rayon_thread_pool("testmultpair".to_string(), Some(32));

    for (g1, g2) in vec![
        ([G1Projective::identity(), r1[0]], r2),
        (r1, r2),
        (r1, [G2Projective::identity(), r2[0]]),
    ] {
        let res1 = multi_pairing(g1.iter(), g2.iter());
        let res2 = parallel_multi_pairing(g1.iter(), g2.iter(), &pool1, MIN_MULTIPAIR_NUM_JOBS);
        let res3 = parallel_multi_pairing(g1.iter(), g2.iter(), &pool32, MIN_MULTIPAIR_NUM_JOBS);

        assert_eq!(res1, res2);
        assert_eq!(res1, res3);
    }
}
