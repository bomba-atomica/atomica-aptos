// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Node-side rule for deriving timelock key shares.
//!
//! Design: `atomica/docs/decisions/aptos-upgrade-ibe-chunky-spec.md` sections 5 and 7.
//!
//! A node may derive a share (`s_p * H1(id)`) only for a timelock it read from the on-chain registry that
//! is `Pending`, belongs to the current epoch, and whose deadline has passed in the **committed** block
//! timestamp. There is no function in this module that derives a share from a bare identity. The only way to
//! reach [`DerivableTimelock::derive_shares`] is [`DerivableTimelock::authorize`], which takes the registry
//! row and the committed state. The node's wall clock is never an input.

use aptos_dkg::ibe::{compute_identity, derive_decryption_key, encode_g1, IbeError, MasterScalar};

/// Status of a registry row, as read from chain state. `Revealed` and `Expired` are final.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegistryTimelockStatus {
    Pending,
    Revealed,
    Expired,
}

/// One timelock as read from `ibe_config` in committed chain state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegistryTimelock {
    pub timelock_id: u64,
    /// Epoch the timelock was registered in.
    pub epoch: u64,
    pub deadline_us: u64,
    pub reveal_expiry_us: u64,
    /// The registry's identity bytes.
    pub identity: Vec<u8>,
    pub status: RegistryTimelockStatus,
}

/// Why the node refuses to derive shares for a timelock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DerivationRefusal {
    NotPending,
    EpochNotCurrent {
        timelock_epoch: u64,
        current_epoch: u64,
    },
    DeadlineNotPassed {
        deadline_us: u64,
        committed_timestamp_us: u64,
    },
    RevealWindowElapsed {
        reveal_expiry_us: u64,
        committed_timestamp_us: u64,
    },
    /// The registry identity is not the identity of `(chain, epoch, id, deadline)`.
    IdentityMismatch,
}

/// A timelock the node may derive shares for. Fields are private: the value exists only through
/// [`DerivableTimelock::authorize`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DerivableTimelock {
    timelock_id: u64,
    epoch: u64,
    identity: [u8; 32],
}

impl DerivableTimelock {
    /// Checks the node rule. `current_epoch` and `committed_timestamp_us` are the epoch and the timestamp of
    /// the latest committed block, read from chain state.
    pub fn authorize(
        timelock: &RegistryTimelock,
        chain_id: u8,
        current_epoch: u64,
        committed_timestamp_us: u64,
    ) -> Result<Self, DerivationRefusal> {
        if timelock.status != RegistryTimelockStatus::Pending {
            return Err(DerivationRefusal::NotPending);
        }
        if timelock.epoch != current_epoch {
            return Err(DerivationRefusal::EpochNotCurrent {
                timelock_epoch: timelock.epoch,
                current_epoch,
            });
        }
        if committed_timestamp_us < timelock.deadline_us {
            return Err(DerivationRefusal::DeadlineNotPassed {
                deadline_us: timelock.deadline_us,
                committed_timestamp_us,
            });
        }
        if committed_timestamp_us >= timelock.reveal_expiry_us {
            return Err(DerivationRefusal::RevealWindowElapsed {
                reveal_expiry_us: timelock.reveal_expiry_us,
                committed_timestamp_us,
            });
        }
        let expected = compute_identity(
            chain_id,
            timelock.epoch,
            timelock.timelock_id,
            timelock.deadline_us,
        );
        if timelock.identity.as_slice() != expected.as_slice() {
            return Err(DerivationRefusal::IdentityMismatch);
        }
        Ok(Self {
            timelock_id: timelock.timelock_id,
            epoch: timelock.epoch,
            identity: expected,
        })
    }

    pub fn timelock_id(&self) -> u64 {
        self.timelock_id
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    /// The compressed G1 shares `s_p * H1(id)` for the node's secret shares, in player order.
    pub fn derive_shares(&self, secret_shares: &[MasterScalar]) -> Result<Vec<Vec<u8>>, IbeError> {
        secret_shares
            .iter()
            .map(|s| derive_decryption_key(s, &self.identity).map(|p| encode_g1(&p).to_vec()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_dkg::ibe::{decode_g1, master_public_key, random_scalar, verify_dk_share};
    use rand::{rngs::StdRng, SeedableRng};

    const CHAIN: u8 = 4;

    fn row(epoch: u64, id: u64, deadline: u64) -> RegistryTimelock {
        RegistryTimelock {
            timelock_id: id,
            epoch,
            deadline_us: deadline,
            reveal_expiry_us: deadline + 600_000_000,
            identity: compute_identity(CHAIN, epoch, id, deadline).to_vec(),
            status: RegistryTimelockStatus::Pending,
        }
    }

    #[test]
    fn derives_only_at_or_after_the_deadline_in_the_current_epoch() {
        let t = row(3, 7, 1_000);
        // Deadline not yet passed in the committed block timestamp.
        assert_eq!(
            DerivableTimelock::authorize(&t, CHAIN, 3, 999),
            Err(DerivationRefusal::DeadlineNotPassed {
                deadline_us: 1_000,
                committed_timestamp_us: 999
            })
        );
        // Exactly at the deadline is allowed.
        let ok = DerivableTimelock::authorize(&t, CHAIN, 3, 1_000).unwrap();
        assert_eq!((ok.timelock_id(), ok.epoch()), (7, 3));
        // After the reveal window nothing is derived.
        assert!(matches!(
            DerivableTimelock::authorize(&t, CHAIN, 3, 1_000 + 600_000_000),
            Err(DerivationRefusal::RevealWindowElapsed { .. })
        ));
    }

    #[test]
    fn refuses_other_epochs_final_status_and_forged_identity() {
        let t = row(3, 7, 1_000);
        assert!(matches!(
            DerivableTimelock::authorize(&t, CHAIN, 4, 2_000),
            Err(DerivationRefusal::EpochNotCurrent { .. })
        ));
        for status in [
            RegistryTimelockStatus::Revealed,
            RegistryTimelockStatus::Expired,
        ] {
            let mut done = t.clone();
            done.status = status;
            assert_eq!(
                DerivableTimelock::authorize(&done, CHAIN, 3, 2_000),
                Err(DerivationRefusal::NotPending)
            );
        }
        // An identity that is not the one of (chain, epoch, id, deadline) is not derived.
        let mut forged = t.clone();
        forged.identity = vec![9u8; 32];
        assert_eq!(
            DerivableTimelock::authorize(&forged, CHAIN, 3, 2_000),
            Err(DerivationRefusal::IdentityMismatch)
        );
        // Another chain id changes the identity.
        assert_eq!(
            DerivableTimelock::authorize(&t, CHAIN + 1, 3, 2_000),
            Err(DerivationRefusal::IdentityMismatch)
        );
    }

    #[test]
    fn derived_shares_pass_the_share_check() {
        let mut rng = StdRng::seed_from_u64(5);
        let secrets: Vec<_> = (0..3).map(|_| random_scalar(&mut rng)).collect();
        let t = row(3, 7, 1_000);
        let d = DerivableTimelock::authorize(&t, CHAIN, 3, 1_500).unwrap();
        let shares = d.derive_shares(&secrets).unwrap();
        assert_eq!(shares.len(), 3);
        for (s, share) in secrets.iter().zip(&shares) {
            assert!(verify_dk_share(
                &decode_g1(share).unwrap(),
                &t.identity,
                &master_public_key(s)
            ));
        }
    }
}
