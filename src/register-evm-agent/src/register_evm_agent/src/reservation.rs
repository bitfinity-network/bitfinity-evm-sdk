use candid::{Decode, Encode, Principal};
use did::error::EvmError;

use did::H160;
use eth_signer::{Signer, Wallet};
use ethers_core::k256::ecdsa::SigningKey;

use ic_agent::Agent;

use crate::agent::user_principal;
use crate::constant::{METHOD_ADDRESS_RESERVED, METHOD_MINT_NATIVE_TOKENS, METHOD_RESERVE_ADDRESS};
use crate::error::{Error, Result};

pub struct ReservationService<'a> {
    agent: Agent,
    amount_to_mint: Option<u64>,
    evmc_canister_id: Principal,
    register_canister_id: Principal,
    wallet: Wallet<'a, SigningKey>,
}

impl<'a> ReservationService<'a> {
    pub async fn new(
        agent: Agent,
        amount_to_mint: Option<u64>,
        evmc_canister_id: Principal,
        register_canister_id: Principal,
        wallet: Wallet<'a, SigningKey>,
    ) -> Result<ReservationService<'a>> {
        Ok(Self {
            agent,
            amount_to_mint,
            evmc_canister_id,
            register_canister_id,
            wallet,
        })
    }

    pub async fn reserve(&self) -> Result<()> {
        self.reserve_ic_agent().await?;

        Ok(())
    }

    async fn reserve_ic_agent(&self) -> Result<()> {
        let principal = user_principal(&self.agent)?;
        info!("registering ic-agent {principal}");
        let is_registered = self.is_address_reserved().await?;
        if is_registered {
            info!("agent is already registered");
            return Err(Error::AlreadyRegistered(principal));
        }
        let address: did::H160 = self.wallet.address().into();
        let args = Encode!(&self.register_canister_id, &address)?;

        // mint tokens to be able to pay registration fee (only on testnets)
        if let Some(amount_to_mint) = self.amount_to_mint {
            info!("minting native tokens for address");
            self.mint_native_tokens_to_address(amount_to_mint).await?;
        }

        let res = self
            .agent
            .update(&self.evmc_canister_id, METHOD_RESERVE_ADDRESS)
            .with_arg(args)
            .call_and_wait()
            .await?;

        info!("{METHOD_RESERVE_ADDRESS} called, decoding result");
        Decode!(res.as_slice(), std::result::Result<(), EvmError>)??;
        info!("result is OK");

        Ok(())
    }

    async fn is_address_reserved(&self) -> Result<bool> {
        let address: H160 = self.wallet.address().into();
        info!("checking if {address} is already reserved...");
        let args = Encode!(&self.register_canister_id, &address)?;
        let res = self
            .agent
            .query(&self.evmc_canister_id, METHOD_ADDRESS_RESERVED)
            .with_arg(args)
            .call()
            .await?;
        let principal = user_principal(&self.agent)?;
        match Decode!(res.as_slice(), bool) {
            Ok(res) => {
                info!("{address} is not registered yet");
                Ok(res)
            }
            Err(_) => Err(Error::CouldNotCheckRegistrationStatus(
                address.to_hex_str(),
                principal,
            )),
        }
    }

    async fn mint_native_tokens_to_address(&self, amount_to_mint: u64) -> Result<()> {
        let address = H160::from(self.wallet.address());
        info!("minting EVM tokens to {address}");
        let payload = Encode!(&address, &did::U256::from(amount_to_mint))?;

        let res = self
            .agent
            .update(&self.evmc_canister_id, METHOD_MINT_NATIVE_TOKENS)
            .with_arg(payload)
            .call_and_wait()
            .await?;

        Decode!(res.as_slice(), std::result::Result<did::U256, EvmError>)??;

        info!("tokens minted");

        Ok(())
    }
}
