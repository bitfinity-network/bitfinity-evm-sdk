use std::borrow::Cow;

use candid::Principal;
use ic_stable_structures::{Bound, Storable};

/// Storable principal. May be used as a stable storage key.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct StorablePrincipal(pub Principal);

impl StorablePrincipal {
    pub const MAX_PRINCIPAL_LENGTH_IN_BYTES: usize = 29;
}

impl Storable for StorablePrincipal {
    fn to_bytes(&self) -> Cow<'_, [u8]> {
        self.0.as_slice().into()
    }

    fn from_bytes(bytes: Cow<'_, [u8]>) -> Self {
        Self(Principal::from_slice(&bytes))
    }

    const BOUND: Bound = Bound::Bounded {
        max_size: Self::MAX_PRINCIPAL_LENGTH_IN_BYTES as u32,
        is_fixed_size: false,
    };
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_storable_principal_roundtrip() {
        let principal_01 = Principal::from_slice(&[1; 29]);
        let principal_02 = Principal::from_slice(&[3; 24]);
        let principal_03 =
            Principal::from_text("mfufu-x6j4c-gomzb-geilq").expect("valid principal");

        let principals = vec![principal_01, principal_02, principal_03];

        for principal in principals {
            let source = StorablePrincipal(principal);
            let bytes = source.to_bytes();
            let decoded = StorablePrincipal::from_bytes(bytes);
            assert_eq!(source, decoded);
        }
    }
}
