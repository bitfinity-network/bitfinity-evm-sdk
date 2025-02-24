use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::H160;

/// The `to` field of a transaction. Either a target address, or empty for a contract creation
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize, Default)]
pub enum TxKind {
    #[default]
    Create,
    Call(H160),
}

impl From<Option<H160>> for TxKind {
    fn from(value: Option<H160>) -> Self {
        match value {
            Some(address) => TxKind::Call(address),
            None => TxKind::Create,
        }
    }
}

impl TxKind {
    /// Returns the address of the contract that will be called or will receive the transfer
    pub fn to(&self) -> Option<&H160> {
        match self {
            TxKind::Create => None,
            TxKind::Call(to) => Some(to),
        }
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_should_convert_from_h160() {
        let address = H160::from([0u8; 20]);
        let tx_kind: TxKind = Some(address.clone()).into();
        assert_eq!(tx_kind, TxKind::Call(address));

        let tx_kind: TxKind = None.into();
        assert_eq!(tx_kind, TxKind::Create);
    }

    #[test]
    fn test_should_return_to_address() {
        let address = H160::from([0u8; 20]);
        let tx_kind = TxKind::Call(address.clone());
        assert_eq!(tx_kind.to(), Some(&address));

        let tx_kind = TxKind::Create;
        assert_eq!(tx_kind.to(), None);
    }
}
