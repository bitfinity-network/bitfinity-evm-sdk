use candid::Principal;
use did::H160;
use eth_signer::{Signer, Wallet};
use ethers_core::k256::ecdsa::SigningKey;
use evmc_client::{EvmcClient, IcAgentClient};
use ic_agent::Agent;

use crate::agent::user_principal;
use crate::error::{Error, Result};

type EvmcAgentClient = EvmcClient<IcAgentClient>;

pub struct ReservationService<'a> {
    client: EvmcAgentClient,
    amount_to_mint: Option<u64>,
    reserve_canister_id: Principal,
    agent_principal: Principal,
    wallet: Wallet<'a, SigningKey>,
}

impl<'a> ReservationService<'a> {
    pub async fn new(
        agent: Agent,
        amount_to_mint: Option<u64>,
        evmc_canister_id: Principal,
        reserve_canister_id: Principal,
        wallet: Wallet<'a, SigningKey>,
    ) -> Result<ReservationService<'a>> {
        let agent_principal = user_principal(&agent)?;

        let client = EvmcClient::new(IcAgentClient::with_agent(evmc_canister_id, agent));

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

    async fn reserve_ic_agent(&self) -> Result<()> {
        info!("reserving ic-agent {}", self.agent_principal);

        let is_reserved = self.is_address_reserved().await?;
        if is_reserved {
            info!("agent is already reserved");
            return Err(Error::AlreadyReserved(self.agent_principal));
        }

        let address: did::H160 = self.wallet.address().into();

        // mint tokens to be able to pay reservation fee (only on testnets)
        if let Some(amount_to_mint) = self.amount_to_mint {
            info!("minting native tokens for address");
            self.mint_native_tokens_to_address(amount_to_mint).await?;
        }

        self.client
            .reserve_address(self.reserve_canister_id, address)
            .await??;

        info!("result is OK");

        Ok(())
    }

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

    async fn mint_native_tokens_to_address(&self, amount_to_mint: u64) -> Result<()> {
        let address = H160::from(self.wallet.address());

        info!("minting EVM tokens to {address}");

        self.client
            .mint(address, did::U256::from(amount_to_mint))
            .await??;

        info!("tokens minted");

        Ok(())
    }
}
