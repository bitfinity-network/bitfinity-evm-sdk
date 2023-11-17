use candid::Principal;
use did::error::SignatureVerificationError;
use did::{Transaction, H160};
use ic_canister_client::{CanisterClient, CanisterClientResult};

/// This is the result type for all SignatureVerification canister calls.
pub type SignatureVerificationResult<T> = Result<T, SignatureVerificationError>;

/// A Signature Verification canister client.
#[derive(Debug)]
pub struct SignatureVerificationCanisterClient<C>
where
    C: CanisterClient,
{
    /// The canister client.
    client: C,
}

impl<C: CanisterClient> SignatureVerificationCanisterClient<C> {
    /// Create a new canister client.
    ///
    /// # Arguments
    /// * `client` - The canister client.
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Verifies a transaction signature and returns the signing address
    pub async fn verify_signature(
        &self,
        transaction: Transaction,
    ) -> CanisterClientResult<SignatureVerificationResult<H160>> {
        self.client.query("verify_signature", (transaction,)).await
    }

    /// Add principal to the access control list
    pub async fn add_principal_to_access_list(
        &self,
        principal: Principal,
    ) -> CanisterClientResult<SignatureVerificationResult<()>> {
        self.client.update("add_access", (principal,)).await
    }

    /// Remove principal from the access control list
    pub async fn remove_principal_from_access_list(
        &self,
        principal: Principal,
    ) -> CanisterClientResult<SignatureVerificationResult<()>> {
        self.client.update("remove_access", (principal,)).await
    }

    /// Get the owner of the canister
    pub async fn get_owner(&self) -> CanisterClientResult<Principal> {
        self.client.query("get_owner", ()).await
    }

    /// Set the owner of the canister
    pub async fn set_owner(
        &self,
        principal: Principal,
    ) -> CanisterClientResult<SignatureVerificationResult<()>> {
        self.client.update("set_owner", (principal,)).await
    }

    /// Get the access control list
    pub async fn get_access_list(&self) -> CanisterClientResult<Vec<Principal>> {
        self.client.query("get_access_list", ()).await
    }
}
