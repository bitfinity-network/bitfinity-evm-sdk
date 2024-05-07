use std::borrow::Cow;

use candid::{CandidType, Principal};
use did::H160;
use ic_stable_structures::{Bound, Storable};
use serde::Deserialize;

use crate::error::Error;

/// 32-bytes entity identifier.
/// Uniquely identifies:
/// - an EVM address,
/// - an IC principal,
///
/// # Encoding
/// - first byte is the token type identifier,
///
/// ## EVM addresses encoding
/// [1..5] - big endian chain id integer,
/// [5..25] - EVM address data.
///
/// ## IC principals encoding
/// [1] - principal data length,
/// [2..] - principal data.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    CandidType,
    Deserialize,
    serde::Serialize,
)]
pub struct Id256(pub [u8; ID_256_BYTE_SIZE]);

const ID_256_BYTE_SIZE: usize = 32;

impl Id256 {
    pub const BYTE_SIZE: usize = ID_256_BYTE_SIZE;
    pub const PRINCIPAL_MARK: u8 = 0;
    pub const EVM_ADDRESS_MARK: u8 = 1;
    pub const BTC_TX_MARK: u8 = 2;

    /// Creates unique identifier for contract.
    /// Chain id required to make identifiers unique across all chains.
    pub fn from_evm_address(address: &H160, chain_id: u32) -> Self {
        let mut buf = [0u8; Self::BYTE_SIZE];

        buf[0] = Self::EVM_ADDRESS_MARK;

        let chain_id_data = chain_id.to_be_bytes();
        let address_data = address.0.as_bytes();
        buf[1..][..chain_id_data.len()].copy_from_slice(&chain_id_data);
        buf[1 + chain_id_data.len()..][..address_data.len()].copy_from_slice(address_data);

        Self(buf)
    }

    pub fn to_evm_address(&self) -> Result<(u32, H160), Error> {
        if self.0[0] != Self::EVM_ADDRESS_MARK {
            return Err(Error::Internal("wrong evm address mark in Id256".into()));
        }

        let chain_id_bytes = self.0[1..5]
            .try_into()
            .expect("we have exactly 4 bytes, as expected for u32");
        let chain_id = u32::from_be_bytes(chain_id_bytes);

        let address = H160::from_slice(&self.0[5..25]);
        Ok((chain_id, address))
    }

    /// Creates Self from bytes.
    /// The `bytes` must contain exactly 32 bytes.
    pub fn from_slice(bytes: &[u8]) -> Option<Self> {
        Self::try_from(bytes).ok()
    }

    pub fn chain_id(&self) -> u32 {
        if self.0[0] == Self::PRINCIPAL_MARK {
            return 0;
        }

        u32::from_be_bytes(self.0[1..5].try_into().expect("exactly 4 bytes"))
    }

    pub fn native_address() -> H160 {
        let mut bytes = [0u8; H160::BYTE_SIZE];
        const NO_TO_TOKEN_MARK: u8 = 2;
        bytes[19] = NO_TO_TOKEN_MARK;
        H160::from_slice(&bytes)
    }

    pub fn no_to_address() -> H160 {
        let mut bytes = [0u8; H160::BYTE_SIZE];
        const NO_TO_TOKEN_MARK: u8 = 3;
        bytes[19] = NO_TO_TOKEN_MARK;
        H160::from_slice(&bytes)
    }

    /// Convert BTC transaction index into id256 type.
    ///
    /// Transactions in BTC network are indexed by the block id the transaction belongs to and the
    /// index of the transaction in the block.
    pub fn from_btc_tx_index(block_id: u64, tx_index: u32) -> Self {
        let mut buf = [0u8; Self::BYTE_SIZE];

        buf[0] = Self::BTC_TX_MARK;

        let block_id_bytes = block_id.to_be_bytes();
        let tx_id_bytes = tx_index.to_be_bytes();
        buf[1..][..block_id_bytes.len()].copy_from_slice(&block_id_bytes);
        buf[1 + block_id_bytes.len()..][..tx_id_bytes.len()].copy_from_slice(&tx_id_bytes);

        Self(buf)
    }

    /// Converts Id256 into `(block_id, tx_index)` transaction index if the ID represents the rune id,
    /// or returns an error otherwise.
    pub fn to_btc_tx_index(&self) -> Result<(u64, u32), Error> {
        if self.0[0] != Self::BTC_TX_MARK {
            return Err(Error::Internal("wrong rune id mark in Id256".into()));
        }

        let block_id_bytes = self.0[1..9]
            .try_into()
            .expect("we have exactly 8 bytes, as expected for u64");
        let block_id = u64::from_be_bytes(block_id_bytes);

        let tx_index_bytes = self.0[9..13]
            .try_into()
            .expect("we have exactly 8 bytes, as expected for u32");
        let tx_index = u32::from_be_bytes(tx_index_bytes);
        Ok((block_id, tx_index))
    }
}

impl TryFrom<&[u8]> for Id256 {
    type Error = Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let inner: [u8; Self::BYTE_SIZE as _] = value
            .try_into()
            .map_err(|_| Error::Internal("data of Id256 should contain exactly 32 bytes".into()))?;

        match inner[0] {
            Self::PRINCIPAL_MARK | Self::EVM_ADDRESS_MARK => Ok(Self(inner)),
            _ => Err(Error::Internal("wrong Id256 mark in first byte".into())),
        }
    }
}

impl From<&Principal> for Id256 {
    fn from(principal: &Principal) -> Self {
        let mut buf = [0u8; 32];

        buf[0] = Self::PRINCIPAL_MARK;

        let principal_data = principal.as_slice();
        buf[1] = principal_data.len() as u8;
        buf[2..][..principal_data.len()].copy_from_slice(principal_data);

        Self(buf)
    }
}

impl TryFrom<Id256> for Principal {
    type Error = Error;

    fn try_from(id: Id256) -> std::result::Result<Self, Self::Error> {
        if id.0[0] != Id256::PRINCIPAL_MARK {
            return Err(Error::Internal("wrong principal mark in Id256".into()));
        }

        let principal_len = id.0[1] as usize;
        if principal_len > 29 {
            return Err(Error::Internal("wrong principal data len in Id256".into()));
        }

        Ok(Principal::from_slice(&id.0[2..][..principal_len]))
    }
}

impl TryFrom<Id256> for H160 {
    type Error = Error;

    fn try_from(id: Id256) -> std::result::Result<Self, Self::Error> {
        if id.0[0] != Id256::EVM_ADDRESS_MARK {
            return Err(Error::Internal("wrong address mark in Id256".into()));
        }

        Ok(H160::from_slice(&id.0[1..][..20]))
    }
}

impl Storable for Id256 {
    const BOUND: Bound = Bound::Bounded {
        max_size: Self::BYTE_SIZE as _,
        is_fixed_size: true,
    };

    fn to_bytes(&self) -> Cow<'_, [u8]> {
        (&self.0[..]).into()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self::try_from(bytes.as_ref()).expect("failed to deserialize Id256")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id256_from_slice() {
        let res = Id256::try_from(&[1u8; 32][..]);
        assert!(res.is_ok());

        let res = Id256::try_from(&[1u8; 33][..]);
        assert_eq!(
            res,
            Err(Error::Internal(
                "data of Id256 should contain exactly 32 bytes".into()
            ))
        );
    }

    #[test]
    fn id256_to_address_roundtrip() {
        let chain_id = 31156;
        let address = H160::from_slice(&[42; 20]);
        let id = Id256::from_evm_address(&address, chain_id);
        let (restored_chain_id, restored_address) = id.to_evm_address().unwrap();

        assert_eq!(restored_chain_id, chain_id);
        assert_eq!(restored_address, address);
    }

    #[test]
    fn id256_to_address_invalid_type() {
        let principal = Principal::from_slice(&[20; 29]);
        let id = Id256::from(&principal);

        assert_eq!(
            id.to_evm_address(),
            Err(Error::Internal("wrong evm address mark in Id256".into()))
        );
    }

    #[test]
    fn id256_to_principal_roundtrip() {
        let principal = Principal::from_slice(&[20; 29]);
        let id = Id256::from(&principal);
        let restored_principal = Principal::try_from(id).unwrap();

        assert_eq!(restored_principal, principal);
    }

    #[test]
    fn id256_to_principal_invalid_type() {
        let chain_id = 31156;
        let address = H160::from_slice(&[42; 20]);
        let id = Id256::from_evm_address(&address, chain_id);

        assert_eq!(
            Principal::try_from(id),
            Err(Error::Internal("wrong principal mark in Id256".into()))
        );
    }

    #[test]
    fn storable_id256_roundtrip() {
        let chain_id = 31156;
        let address = H160::from_slice(&[42; 20]);
        let id = Id256::from_evm_address(&address, chain_id);

        let decoded = Id256::from_bytes(id.to_bytes());
        assert_eq!(id, decoded);
    }

    #[test]
    fn btc_tx_index_roundtrip() {
        let block_id = 100500;
        let tx_id = 42;

        let id = Id256::from_btc_tx_index(block_id, tx_id);
        assert_eq!(id.to_btc_tx_index(), Ok((block_id, tx_id)));
    }

    #[test]
    fn btc_tx_index_decode_error() {
        let id = Id256::from_evm_address(&H160::from_slice(&[42; 20]), 31156);
        assert!(matches!(id.to_btc_tx_index(), Err(Error::Internal(_))), "unexpected to_rune_id result: {:?}", id.to_btc_tx_index())
    }
}
