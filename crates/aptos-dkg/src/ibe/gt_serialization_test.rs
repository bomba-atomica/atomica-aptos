//! Test file to explore Gt/Fp12 serialization options

#[cfg(test)]
mod tests {
    use aptos_crypto::blstrs::multi_pairing;
    use blstrs::{Fp12, G1Projective, G2Projective, Gt};
    use group::Group;
    use std::iter;

    #[test]
    fn explore_gt_fp12_serialization() {
        // Create a Gt element via pairing
        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();
        let gt = multi_pairing(iter::once(&g1), iter::once(&g2));

        // Try to convert to Fp12
        // Gt and Fp12 should be convertible
        let fp12: Fp12 = gt.into();

        // Check if we can convert back
        let gt_back: Gt = fp12.into();
        assert_eq!(gt, gt_back, "Gt <-> Fp12 conversion should be bidirectional");

        println!("Gt: {:?}", gt);
        println!("Fp12: {:?}", fp12);

        // Check debug format length (this is what current code uses)
        let debug_str = format!("{:?}", gt);
        println!("Debug string length: {}", debug_str.len());
        println!("Debug string (first 100 chars): {}", &debug_str[..100.min(debug_str.len())]);
    }

    #[test]
    fn test_gt_serialization_methods() {
        let g1 = G1Projective::generator();
        let g2 = G2Projective::generator();
        let gt = multi_pairing(iter::once(&g1), iter::once(&g2));

        // Try various serialization approaches
        // Uncomment these to see what works:

        // Option 1: Direct to_bytes (if it exists)
        // let bytes = gt.to_bytes();

        // Option 2: Via Fp12
        let fp12: Fp12 = gt.into();
        // let bytes = fp12.to_bytes(); // Does this exist?

        // Option 3: Serde (if implemented)
        // let bytes = bincode::serialize(&gt).unwrap();

        // For now, just check the structure
        println!("Testing Gt serialization options...");
    }
}
