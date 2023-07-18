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
}
