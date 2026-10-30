// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[cfg(any(test, feature = "fuzzing"))]
use crate::dkg::DKGTranscriptMetadata;
use crate::{
    dkg::{chunky_dkg::CertifiedChunkyDKGOutput, DKGTranscript},
    jwks,
    validator_verifier::ValidatorVerifier,
};
use anyhow::Context;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub enum ValidatorTransaction {
    DKGResult(DKGTranscript),
    ObservedJWKUpdate(jwks::QuorumCertifiedUpdate),
    ChunkyDKGResult(CertifiedChunkyDKGOutput),
    /// Appended at tag 3. The fork's pre-1.49.6 `TimelockShare` used tag 2, which upstream now gives
    /// to `ChunkyDKGResult`. Never reorder these variants: the tag is part of the BCS wire format.
    TimelockShare(TimelockShare),
}

/// Upper bound on the number of shares in one `TimelockShare` (one per virtual player of the author).
/// Cheap structural bound only. The exact count is checked against the epoch config on chain.
pub const MAX_TIMELOCK_SHARES: usize = 1 << 16;

/// Length of a compressed BLS12-381 G1 point.
pub const TIMELOCK_SHARE_LEN: usize = 48;

/// The decryption key shares of one validator for one timelock (spec `aptos-upgrade-ibe-chunky-spec.md`
/// sections 5 and 8). Submitted after the timelock deadline passed in a committed block.
///
/// The transaction carries no signature. Its authenticity rests on the share check: every share must
/// satisfy `e(H1(id), vk_p) = e(share, g2)` for the verification key of the author's virtual player, so a
/// forged or replayed share for another author is either valid (harmless) or rejected without using the
/// author's slot.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct TimelockShare {
    /// Epoch the share was derived in. Must equal the current epoch and the epoch of the timelock.
    pub epoch: u64,
    /// Registry id of the timelock.
    pub timelock_id: u64,
    /// The validator that submits the shares.
    pub author: AccountAddress,
    /// One compressed G1 share (48 bytes) per virtual player of the author, in player order.
    pub shares: Vec<Vec<u8>>,
}

impl TimelockShare {
    /// Structural checks that need no chain state.
    pub fn verify_structure(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.shares.is_empty(), "TimelockShare has no shares");
        anyhow::ensure!(
            self.shares.len() <= MAX_TIMELOCK_SHARES,
            "TimelockShare has too many shares"
        );
        anyhow::ensure!(
            self.shares.iter().all(|s| s.len() == TIMELOCK_SHARE_LEN),
            "TimelockShare share has the wrong length"
        );
        Ok(())
    }
}

impl ValidatorTransaction {
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn dummy(payload: Vec<u8>) -> Self {
        Self::DKGResult(DKGTranscript {
            metadata: DKGTranscriptMetadata {
                epoch: 999,
                author: AccountAddress::ZERO,
            },
            transcript_bytes: payload,
        })
    }

    pub fn size_in_bytes(&self) -> usize {
        bcs::serialized_size(self).unwrap()
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            ValidatorTransaction::DKGResult(_) => "validator_transaction__dkg_result",
            ValidatorTransaction::ObservedJWKUpdate(_) => {
                "validator_transaction__observed_jwk_update"
            },
            ValidatorTransaction::ChunkyDKGResult(_) => "validator_transaction__chunky_dkg_result",
            ValidatorTransaction::TimelockShare(_) => "validator_transaction__timelock_share",
        }
    }

    pub fn verify(&self, verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        match self {
            ValidatorTransaction::DKGResult(dkg_result) => dkg_result
                .verify(verifier)
                .context("DKGResult verification failed"),
            ValidatorTransaction::ObservedJWKUpdate(_) => Ok(()),
            ValidatorTransaction::ChunkyDKGResult(_) => {
                // TODO: Implement verification for ChunkyDKGResult
                Ok(())
            },
            ValidatorTransaction::TimelockShare(share) => {
                share
                    .verify_structure()
                    .context("TimelockShare structure check failed")?;
                verifier
                    .get_public_key(&share.author)
                    .ok_or_else(|| anyhow::anyhow!("TimelockShare author is not a validator"))?;
                Ok(())
            },
        }
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Topic {
    DKG,
    JWK_CONSENSUS(jwks::Issuer),
    JWK_CONSENSUS_PER_KEY_MODE {
        issuer: jwks::Issuer,
        kid: jwks::KID,
    },
    ChunkyDKG,
    /// One pooled share transaction per timelock and epoch.
    TimelockShare {
        epoch: u64,
        timelock_id: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn share() -> TimelockShare {
        TimelockShare {
            epoch: 7,
            timelock_id: 9,
            author: AccountAddress::from_hex_literal("0xa11ce").unwrap(),
            shares: vec![vec![1u8; TIMELOCK_SHARE_LEN]; 3],
        }
    }

    #[test]
    fn timelock_share_tag_is_stable() {
        // The ULEB128 variant tag is the first BCS byte. DKGResult 0, ObservedJWKUpdate 1,
        // ChunkyDKGResult 2 (upstream 1.49.6), TimelockShare 3.
        let bytes = bcs::to_bytes(&ValidatorTransaction::TimelockShare(share())).unwrap();
        assert_eq!(bytes[0], 3);
        let dummy = bcs::to_bytes(&ValidatorTransaction::dummy(vec![1])).unwrap();
        assert_eq!(dummy[0], 0);
    }

    #[test]
    fn timelock_share_bcs_roundtrip_and_golden_bytes() {
        let txn = ValidatorTransaction::TimelockShare(share());
        let bytes = bcs::to_bytes(&txn).unwrap();
        assert_eq!(bcs::from_bytes::<ValidatorTransaction>(&bytes).unwrap(), txn);
        // tag, epoch u64 LE, timelock_id u64 LE, author (32 bytes), share count, first share length.
        assert_eq!(&bytes[1..9], &7u64.to_le_bytes());
        assert_eq!(&bytes[9..17], &9u64.to_le_bytes());
        assert_eq!(&bytes[17..49], &AccountAddress::from_hex_literal("0xa11ce").unwrap().to_vec()[..]);
        assert_eq!(bytes[49], 3);
        assert_eq!(bytes[50], TIMELOCK_SHARE_LEN as u8);
        assert_eq!(bytes.len(), 50 + 3 * (1 + TIMELOCK_SHARE_LEN));
        assert_eq!(txn.size_in_bytes(), bytes.len());
        assert_eq!(txn.type_name(), "validator_transaction__timelock_share");
    }

    #[test]
    fn timelock_share_structure_checks() {
        assert!(share().verify_structure().is_ok());
        let mut empty = share();
        empty.shares.clear();
        assert!(empty.verify_structure().is_err());
        let mut short = share();
        short.shares[1] = vec![0u8; TIMELOCK_SHARE_LEN - 1];
        assert!(short.verify_structure().is_err());
        let mut long = share();
        long.shares[2] = vec![0u8; TIMELOCK_SHARE_LEN + 1];
        assert!(long.verify_structure().is_err());
    }

    #[test]
    fn timelock_topic_is_per_timelock_and_epoch() {
        let a = Topic::TimelockShare { epoch: 1, timelock_id: 1 };
        assert_eq!(a, Topic::TimelockShare { epoch: 1, timelock_id: 1 });
        assert_ne!(a, Topic::TimelockShare { epoch: 2, timelock_id: 1 });
        assert_ne!(a, Topic::TimelockShare { epoch: 1, timelock_id: 2 });
        assert_ne!(a, Topic::ChunkyDKG);
    }
}
