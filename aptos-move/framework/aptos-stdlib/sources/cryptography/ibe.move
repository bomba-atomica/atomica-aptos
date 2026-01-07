/// This module provides Identity-Based Encryption (IBE) decryption capabilities.
/// It uses the `crypto_algebra` module for underlying algebraic structures (G1, G2, Gt).
module aptos_std::ibe {
    use aptos_std::crypto_algebra::{Self, Element};

    /// Decrypts a message using Identity-Based Encryption (IBE) logic.
    /// Performs Pairing(u, sig) -> Gt, Serializes Gt, Hashes (Keccak256), and XORs with ciphertext.
    /// 
    /// generic types G1, G2, Gt must match the curves used (e.g. BLS12-381).
    public fun decrypt<G1, G2, Gt>(u: &Element<G1>, sig: &Element<G2>, ciphertext: vector<u8>): vector<u8> {
        // Use native IBE decryption which is gas-optimized
        // Calls crypto_algebra::handle explicitly to avoid dot-call resolution issues
        decrypt_internal<G1, G2, Gt>(
            crypto_algebra::handle(u), 
            crypto_algebra::handle(sig), 
            ciphertext
        )
    }

    // Native function definition
    native fun decrypt_internal<G1, G2, Gt>(u_handle: u64, sig_handle: u64, ciphertext: vector<u8>): vector<u8>;
}
