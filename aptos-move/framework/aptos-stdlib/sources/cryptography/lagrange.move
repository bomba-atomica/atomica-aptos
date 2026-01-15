/// This module provides a native implementation for computing Lagrange coefficients
/// for Shamir's Secret Sharing over a prime field.
module aptos_std::lagrange {
    use std::features;
    use aptos_std::crypto_algebra;

    /// Compute Lagrange coefficients for a set of participants.
    /// Returns vector of scalar elements in field `F`.
    public fun coefficients<F>(participants: &vector<u64>): vector<crypto_algebra::Element<F>> {
        if (!features::cryptography_algebra_enabled()) {
            abort(std::error::not_implemented(0))
        };
        
        let handles = lagrange_coefficients_internal<F>(*participants);
        let elements = std::vector::empty();
        let i = 0;
        let n = std::vector::length(&handles);
        while (i < n) {
            let handle = *std::vector::borrow(&handles, i);
            std::vector::push_back(&mut elements, crypto_algebra::wrap_element<F>(handle));
            i = i + 1;
        };
        elements
    }

    native fun lagrange_coefficients_internal<F>(participants: vector<u64>): vector<u64>;
}
