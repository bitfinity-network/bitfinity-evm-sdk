use candid::Principal;
use did::transaction::{SigningMethod, TransactionBuilder};
use did::{H160, H256, U256};
use eth_signer::{Signer, Wallet};
use ethers_core::k256::ecdsa::SigningKey;
use evm_canister_client::{EvmCanisterClient, IcAgentClient, ic_agent::Agent};

use crate::agent::user_principal;
use crate::cli::DEFAULT_CHAIN_ID;
use crate::error::{Error, Result};

type EvmCanisterAgentClient = EvmCanisterClient<IcAgentClient>;

pub struct ReservationService<'a> {
    client: EvmCanisterAgentClient,
    amount_to_mint: Option<u64>,
    reserve_canister_id: Principal,
    agent_principal: Principal,
    wallet: Wallet<'a, SigningKey>,
}

impl<'a> ReservationService<'a> {
    pub async fn new(
        agent: Agent,
        amount_to_mint: Option<u64>,
        evm_canister_id: Principal,
        reserve_canister_id: Principal,
        wallet: Wallet<'a, SigningKey>,
    ) -> Result<ReservationService<'a>> {
        let agent_principal = user_principal(&agent)?;

        let client = EvmCanisterClient::new(IcAgentClient::with_agent(evm_canister_id, agent));

        Ok(Self {
            client,
            amount_to_mint,
            reserve_canister_id,
            agent_principal,
            wallet,
        })
    }

    pub async fn reserve(&self) -> Result<()> {
        self.reserve_ic_agent().await?;

        Ok(())
    }

    /// Reserve the address on the ic-agent canister
    ///
    /// Two steps:
    /// 1. Send a transaction to the ic-agent canister to reserve the address
    /// with the ic-agent attached to the transaction as an input
    /// 2. Call the reserve_address method on the ic-agent canister with the
    /// transaction hash in the step 1
    ///
    async fn reserve_ic_agent(&self) -> Result<()> {
        info!("reserving ic-agent {}", self.reserve_canister_id);

        let is_reserved = self.is_address_reserved().await?;
        if is_reserved {
            info!("address is already reserved");
            return Err(Error::AlreadyReserved(self.agent_principal));
        }

        let address: did::H160 = self.wallet.address().into();

        // mint tokens to be able to pay reservation fee (only on testnets)
        if let Some(amount_to_mint) = self.amount_to_mint {
            info!("minting native tokens for address");
            self.mint_native_tokens_to_address(amount_to_mint).await?;
        }

        let nonce = self.client.account_basic(address.clone()).await?.nonce;

        let tx = TransactionBuilder {
            from: &address.clone(),
            to: Some(address),
            nonce,
            value: U256::zero(),
            gas: 23_000_u64.into(),
            gas_price: None,
            input: self.reserve_canister_id.as_slice().to_vec(),
            signature: SigningMethod::SigningKey(self.wallet.signer()),
            chain_id: DEFAULT_CHAIN_ID,
        }
        .calculate_hash_and_build()?;

        info!("sending transaction to reserve address...");
        let tx_hash = self.client.send_raw_transaction(tx).await??;

        self.wait_for_transaction(tx_hash.clone()).await?;

        self.client
            .reserve_address(self.reserve_canister_id, tx_hash)
            .await??;

        info!("Address reserved successfully");

        Ok(())
    }

    /// Check if the address is already reserved
    async fn is_address_reserved(&self) -> Result<bool> {
        let address: H160 = self.wallet.address().into();

        info!("checking if {address} is already reserved...");

        let reserved = self
            .client
            .is_address_reserved(self.reserve_canister_id, address.clone())
            .await?;

        if reserved {
            info!("{address} is already reserved");
        } else {
            info!("{address} is not reserved yet");
        }

        Ok(reserved)
    }

    /// Mint native tokens to the address
    async fn mint_native_tokens_to_address(&self, amount_to_mint: u64) -> Result<()> {
        let address = H160::from(self.wallet.address());

        info!("minting EVM native tokens to {address}");

        self.client
            .mint_native_tokens(address, did::U256::from(amount_to_mint))
            .await??;

        info!("tokens minted successfully");

        Ok(())
    }

    /// Wait for the transaction to be finalized
    async fn wait_for_transaction(&self, tx_hash: H256) -> Result<()> {
        const MAX_RETRIES: u32 = 10;

        for _ in 0..MAX_RETRIES {
            info!("waiting for transaction to be finalized...");
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let tx_receipt = self
                .client
                .eth_get_transaction_receipt(tx_hash.clone())
                .await??;

            if let Some(receipt) = tx_receipt {
                if receipt.status == Some(1_u64.into()) {
                    return Ok(());
                } else {
                    return Err(Error::TransactionFailed);
                }
            }
        }

        Err(Error::TransactionNotFinalized(tx_hash))
    }
}
