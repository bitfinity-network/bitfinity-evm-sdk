use std::path::Path;

use candid::Principal;
use ic_agent::identity::{BasicIdentity, Secp256k1Identity};
use ic_agent::Identity;

use crate::error::Error;

pub enum GenericIdentity {
    Secp256k1Identity(Secp256k1Identity),
    BasicIdentity(BasicIdentity),
}

impl TryFrom<&Path> for GenericIdentity {
    type Error = Error;

    fn try_from(path: &Path) -> Result<Self, Self::Error> {
        Secp256k1Identity::from_pem_file(path)
            .map(GenericIdentity::from)
            .or(BasicIdentity::from_pem_file(path).map(GenericIdentity::from))
            .map_err(|e| Error::Pem(path.to_path_buf(), e))
    }
}

impl Identity for GenericIdentity {
    fn sender(&self) -> std::result::Result<Principal, String> {
        match self {
            Self::BasicIdentity(identity) => identity.sender(),
            Self::Secp256k1Identity(identity) => identity.sender(),
        }
    }

    fn sign(&self, blob: &[u8]) -> std::result::Result<ic_agent::Signature, String> {
        match self {
            Self::BasicIdentity(identity) => identity.sign(blob),
            Self::Secp256k1Identity(identity) => identity.sign(blob),
        }
    }
}

impl From<Secp256k1Identity> for GenericIdentity {
    fn from(value: Secp256k1Identity) -> Self {
        Self::Secp256k1Identity(value)
    }
}

impl From<BasicIdentity> for GenericIdentity {
    fn from(value: BasicIdentity) -> Self {
        Self::BasicIdentity(value)
    }
}
