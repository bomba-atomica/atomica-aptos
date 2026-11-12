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

/// BCS mirror of the Move enum `0x1::ibe_config::TimelockStatus`. The variant order is the wire format.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TimelockStatusResource {
    Pending,
    Revealed,
    Expired { reason: u8 },
}

/// BCS mirror of the Move struct `0x1::ibe_config::TimelockInfo`, as stored in the registry table. The field
/// order is the wire format. The node reads it from committed chain state to find the timelocks it serves.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TimelockInfoResource {
    pub timelock_id: u64,
    pub epoch: u64,
    pub deadline_us: u64,
    pub reveal_expiry_us: u64,
    pub identity: Vec<u8>,
    pub decryption_key: Vec<u8>,
    pub status: TimelockStatusResource,
    pub share_count: u64,
    pub reveal_threshold: u64,
    pub validator_indices: Vec<u64>,
    pub submitted_shares: Vec<Vec<Vec<u8>>>,
    pub validator_weights: Vec<u64>,
    pub submitters: Vec<move_core_types::account_address::AccountAddress>,
}

impl TimelockInfoResource {
    /// The registry row the node rule ([`DerivableTimelock::authorize`]) takes.
    pub fn to_registry_timelock(&self) -> RegistryTimelock {
        RegistryTimelock {
            timelock_id: self.timelock_id,
            epoch: self.epoch,
            deadline_us: self.deadline_us,
            reveal_expiry_us: self.reveal_expiry_us,
            identity: self.identity.clone(),
            status: match self.status {
                TimelockStatusResource::Pending => RegistryTimelockStatus::Pending,
                TimelockStatusResource::Revealed => RegistryTimelockStatus::Revealed,
                TimelockStatusResource::Expired { .. } => RegistryTimelockStatus::Expired,
            },
        }
    }
}

/// BCS mirror of the Move event `0x1::ibe_config::TimelockExpiredEvent`. The field order is the wire format.
/// `submitters` lists the validators whose share sets were accepted, in submission order.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TimelockExpiredEvent {
    pub timelock_id: u64,
    pub reason: u8,
    pub timestamp_us: u64,
    pub submitters: Vec<move_core_types::account_address::AccountAddress>,
}

/// The table handle of `TimelockRegistry.timelocks`. The resource is `{ timelocks: Table, pending:
/// BigOrderedMap, next_timelock_id }` and `Table` is a struct with one `address` field, so the handle is the
/// first 32 bytes of the BCS encoding. The rest of the resource is not read.
pub fn timelock_registry_table_handle(resource_bytes: &[u8]) -> Option<[u8; 32]> {
    resource_bytes.get(..32)?.try_into().ok()
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
    fn registry_row_mirror_matches_the_move_layout() {
        // `0x1::ibe_config::TimelockStatus` is an enum: Pending 0, Revealed 1, Expired { reason } 2.
        assert_eq!(
            bcs::to_bytes(&TimelockStatusResource::Pending).unwrap(),
            vec![0]
        );
        assert_eq!(
            bcs::to_bytes(&TimelockStatusResource::Revealed).unwrap(),
            vec![1]
        );
        assert_eq!(
            bcs::to_bytes(&TimelockStatusResource::Expired { reason: 3 }).unwrap(),
            vec![2, 3]
        );
        let info = TimelockInfoResource {
            timelock_id: 4,
            epoch: 3,
            deadline_us: 1_000,
            reveal_expiry_us: 2_000,
            identity: vec![7; 32],
            decryption_key: vec![],
            status: TimelockStatusResource::Pending,
            share_count: 1,
            reveal_threshold: 3,
            validator_indices: vec![0],
            submitted_shares: vec![vec![vec![9; 48]]],
            validator_weights: vec![1],
            submitters: vec![move_core_types::account_address::AccountAddress::ONE],
        };
        let bytes = bcs::to_bytes(&info).unwrap();
        assert_eq!(
            bcs::from_bytes::<TimelockInfoResource>(&bytes).unwrap(),
            info
        );
        let row = info.to_registry_timelock();
        assert_eq!(row.status, RegistryTimelockStatus::Pending);
        assert_eq!((row.timelock_id, row.epoch, row.deadline_us), (4, 3, 1_000));
    }

    #[test]
    fn registry_table_handle_is_the_first_address_of_the_resource() {
        let mut resource = vec![0xABu8; 32];
        resource.extend_from_slice(&[1, 2, 3]);
        assert_eq!(timelock_registry_table_handle(&resource), Some([0xAB; 32]));
        assert_eq!(timelock_registry_table_handle(&resource[..31]), None);
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
