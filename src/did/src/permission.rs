use std::collections::HashSet;

use candid::{CandidType, Deserialize};
use ic_stable_structures::Storable;

use crate::codec;

/// Principal specific permission
#[derive(
    Debug, Clone, CandidType, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord, serde::Serialize,
)]
pub enum Permission {
    /// Gives administrator permissions
    Admin,
    /// Allows calling the endpoints to read the logs and get runtime statistics
    ReadLogs,
    /// Allows calling the endpoints to set the logs configuration
    UpdateLogsConfiguration,
    /// Allows caller to reset the EVM state
    ResetEvmState,
    /// Allows the signature verification canister to send transaction to
    /// the EVM Canister
    PrivilegedSendTransaction,
}

#[derive(Debug, Clone, Default, CandidType, Deserialize, PartialEq, Eq, serde::Serialize)]
pub struct PermissionList {
    pub permissions: HashSet<Permission>,
}

impl Storable for PermissionList {
    fn to_bytes(&self) -> std::borrow::Cow<[u8]> {
        codec::encode(self).into()
    }

    fn from_bytes(bytes: std::borrow::Cow<[u8]>) -> Self {
        codec::decode(&bytes)
    }

    const BOUND: ic_stable_structures::Bound = ic_stable_structures::Bound::Unbounded;
}

#[cfg(test)]
mod test {

    use candid::{Decode, Encode};

    use super::*;

    #[test]
    fn test_candid_permission_list() {
        let permission_list = PermissionList {
            permissions: HashSet::from_iter(vec![Permission::Admin, Permission::ReadLogs]),
        };

        let serialized = Encode!(&permission_list).unwrap();
        let deserialized = Decode!(serialized.as_slice(), PermissionList).unwrap();

        assert_eq!(permission_list, deserialized);
    }

    #[test]
    fn test_storable_permission_list() {
        let permission_list = PermissionList {
            permissions: HashSet::from_iter(vec![Permission::Admin, Permission::ReadLogs]),
        };

        let serialized = permission_list.to_bytes();
        let deserialized = PermissionList::from_bytes(serialized);

        assert_eq!(permission_list, deserialized);
    }
}
