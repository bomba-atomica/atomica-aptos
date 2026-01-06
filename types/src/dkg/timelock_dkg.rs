// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{
    ValidCryptoMaterial, CryptoMaterialError, ValidCryptoMaterialStringExt
};
use aptos_crypto_derive::{DeserializeKey, SerializeKey, SilentDebug, SilentDisplay};
use serde::{Deserialize, Serialize};
use aptos_dkg::{
    pvss::{
        traits::{Reconstructable, Convert},
        Player, WeightedConfig,
        das::{self, PublicParameters as DasPP},
        input_secret::InputSecret,
    },
};

// Access types from blstrs crate
use blstrs::{G1Projective, Scalar};
use group::Group; // For G1Projective::generator()
use ff::PrimeField; // For Scalar::from_repr()

//
// Custom Share Structs for Timelock (Scalar Dealing)
//

/// A Timelock share holds the actual scalar value required for IBE.
/// Note: derive(Serialize, Deserialize) removed to avoid conflict with SerializeKey/DeserializeKey
#[derive(DeserializeKey, SerializeKey, SilentDisplay, SilentDebug, PartialEq, Clone)]
pub struct TimelockShare {
    pub(crate) scalar: Scalar,
    /// Cached public commitment (g^scalar) for verification against group-based PVSS.
    pub(crate) comm: G1Projective, 
}

impl TimelockShare {
    pub fn new(scalar: Scalar) -> Self {
        let comm = G1Projective::generator() * scalar;
        TimelockShare { scalar, comm }
    }
    
    pub fn as_scalar(&self) -> &Scalar {
        &self.scalar
    }
    
    pub fn as_group_element(&self) -> &G1Projective {
        &self.comm
    }
}

impl ValidCryptoMaterial for TimelockShare {
    const AIP_80_PREFIX: &'static str = "";
    fn to_bytes(&self) -> Vec<u8> {
        self.scalar.to_bytes_le().to_vec()
    }
}

impl TryFrom<&[u8]> for TimelockShare {
    type Error = CryptoMaterialError;
    fn try_from(bytes: &[u8]) -> std::result::Result<TimelockShare, Self::Error> {
         // blstrs::Scalar implements ff::PrimeField
         // from_repr takes an array of bytes. Scalar size is 32.
         let bytes_array: [u8; 32] = bytes.try_into().map_err(|_| CryptoMaterialError::DeserializationError)?;
         let s = Option::<Scalar>::from(Scalar::from_repr(bytes_array))
            .ok_or(CryptoMaterialError::DeserializationError)?;
         Ok(TimelockShare::new(s))
    }
}

// Implement Reconstructable<WeightedConfig> as required by DKGTrait::DealtSecretKeyShare (Vec<T>)
impl Reconstructable<WeightedConfig> for TimelockShare {
    type Share = TimelockShare;
    fn reconstruct(_sc: &WeightedConfig, _shares: &Vec<(Player, Self::Share)>) -> Self {
         panic!("TimelockShare reconstruction not implemented")
    }
}

/// A Timelock secret (Scalar wrapper).
/// This is the "DealtSecret" type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelockSecret(pub Scalar);

impl Convert<TimelockSecret, DasPP> for InputSecret {
    fn to(&self, _pp: &DasPP) -> TimelockSecret {
        TimelockSecret(*self.get_secret_a())
    }
}

impl Reconstructable<WeightedConfig> for TimelockSecret {
    type Share = Vec<TimelockShare>; 
    fn reconstruct(_sc: &WeightedConfig, _shares: &Vec<(Player, Self::Share)>) -> Self {
       panic!("TimelockSecret reconstruction not implemented");
    }
}
