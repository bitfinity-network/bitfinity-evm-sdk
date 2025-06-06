use candid::Principal;
use did::build::BuildData;
use did::error::SignatureVerificationError;
use did::{H160, Transaction};
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
        transaction: &Transaction,
    ) -> CanisterClientResult<SignatureVerificationResult<H160>> {
        self.client.query("verify_signature", (transaction,)).await
    }

    /// Add principal to the access control list
    pub async fn admin_add_principal_to_access_list(
        &self,
        principal: Principal,
    ) -> CanisterClientResult<SignatureVerificationResult<()>> {
        self.client
            .update("admin_add_principal_to_access_list", (principal,))
            .await
    }

    /// Remove principal from the access control list
    pub async fn admin_remove_principal_from_access_list(
        &self,
        principal: Principal,
    ) -> CanisterClientResult<SignatureVerificationResult<()>> {
        self.client
            .update("admin_remove_principal_from_access_list", (principal,))
            .await
    }

    /// Get the owner of the canister
    pub async fn get_owner(&self) -> CanisterClientResult<Principal> {
        self.client.query("get_owner", ()).await
    }

    /// Set the owner of the canister
    pub async fn admin_set_owner(
        &self,
        principal: Principal,
    ) -> CanisterClientResult<SignatureVerificationResult<()>> {
        self.client.update("admin_set_owner", (principal,)).await
    }

    /// Get the access control list
    pub async fn get_access_list(&self) -> CanisterClientResult<Vec<Principal>> {
        self.client.query("get_access_list", ()).await
    }

    /// Returns the build data of the canister.
    pub async fn get_canister_build_data(&self) -> CanisterClientResult<BuildData> {
        self.client.query("get_canister_build_data", ()).await
    }

    /// Get the evm canister for the transaction forwarding
    pub async fn get_evm_canister(
        &self,
    ) -> CanisterClientResult<SignatureVerificationResult<Principal>> {
        self.client.query("get_evm_canister", ()).await
    }

    /// Sets the evm canister for the transaction forwarding
    pub async fn admin_set_evm_canister(
        &self,
        principal: Principal,
    ) -> CanisterClientResult<SignatureVerificationResult<()>> {
        self.client
            .update("admin_set_evm_canister", (principal,))
            .await
    }
}
