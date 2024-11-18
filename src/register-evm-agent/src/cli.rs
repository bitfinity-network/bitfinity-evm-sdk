use std::path::PathBuf;

use anyhow::Result;
use candid::Principal;
use clap::{Args, Parser, Subcommand};
use did::H160;
use eth_signer::{Signer, Wallet};
use ethers_core::k256::ecdsa::SigningKey;
use evm_canister_client::agent::identity::init_agent;
use evm_canister_client::{EvmCanisterClient, IcAgentClient};
use register_evm_agent_core::error::Error;
use register_evm_agent_core::reservation::ReservationService;
use register_evm_agent_core::tokio_waiter::TokioTimeWaiter;

use crate::constant::{DEFAULT_CHAIN_ID, NETWORK_IC, NETWORK_LOCAL};
use crate::transaction::SignTransactionArgs;

/// CLI tool for generating wallet & reserving EVM addresses to IC principals
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct ReserveMinterCli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate an ETH Wallet
    GenerateWallet,

    /// Reserve an EVM address to the IC principal
    Reserve(ReserveArgs),

    /// Sign a transaction
    SignTransaction(SignTransactionArgs),
}

#[derive(Args)]
pub struct ReserveArgs {
    /// amount of native tokens to mint on testnets for this wallet
    #[arg(short = 'a', long = "amount-to-mint")]
    pub amount_to_mint: Option<u64>,

    /// Gas price for sending the reserve transaction
    #[arg(short = 'g', long = "gas-price")]
    pub gas_price: u128,

    /// Path to your identity pem file
    #[arg(short = 'i', long = "identity")]
    pub identity: PathBuf,

    /// Evm canister principal
    #[arg(short = 'e', long = "evm")]
    pub evm: Principal,

    /// IC Network (ic, local or custom url)
    #[arg(short, long, default_value_t = String::from(NETWORK_LOCAL))]
    pub network: String,

    /// Principal associated to the reserved address
    #[arg(short = 'c', long = "canister-id")]
    pub reserve_canister_id: Principal,

    /// wallet signing key
    #[arg(short = 'k', long = "key")]
    pub signing_key: String,
}

impl ReserveArgs {
    pub async fn exec(&self) -> Result<()> {
        let wallet = get_wallet(self.signing_key.as_str())?;
        let address = wallet.address();

        info!("initializing agent...");
        let network = network_url(&self.network);
        let agent = init_agent(&self.identity, network, None).await?;

        let client = EvmCanisterClient::new(IcAgentClient::with_agent(self.evm, agent));

        match ReservationService::new(
            client,
            self.amount_to_mint,
            self.gas_price.into(),
            self.reserve_canister_id,
            wallet,
            DEFAULT_CHAIN_ID,
            TokioTimeWaiter,
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .reserve()
        .await
        {
            Ok(()) => {
                println!(
                    "Reservation succeeded:\n  Wallet Address = {}\n  Principal = {}",
                    H160::from(address).to_hex_str(),
                    self.reserve_canister_id
                );

                Ok(())
            }
            Err(Error::AlreadyReserved) => {
                println!(
                    "Already reserved:\n\tWallet Address = {}\n\t",
                    H160::from(address).to_hex_str(),
                );
                Ok(())
            }
            Err(err) => anyhow::bail!("{err}"),
        }
    }
}

/// Parse an existing wallet
pub fn get_wallet<'a>(signing_key: &str) -> Result<Wallet<'a, SigningKey>> {
    let key_bytes = hex::decode(signing_key)?;
    let wallet = Wallet::from_bytes(&key_bytes)?;
    Ok(wallet)
}

/// generate a brand new wallet
pub fn generate_wallet<'a>() -> Result<Wallet<'a, SigningKey>> {
    let mut rng = rand::thread_rng();
    let wallet = Wallet::new(&mut rng);
    let signer = wallet.signer();
    let signer_hex = hex::encode(signer.to_bytes());
    let public_key = wallet.signer().verifying_key();
    let public_key_hex = hex::encode(public_key.to_sec1_bytes());
    let address: H160 = wallet.address().into();
    println!(
        "Wallet:\n  Private Key = {}\n  Public Key = {}\n  Address = {}",
        signer_hex,
        public_key_hex,
        address.to_hex_str(),
    );
    Ok(wallet)
}

/// make network url from network name
pub(crate) fn network_url(network: &str) -> &str {
    match network {
        NETWORK_LOCAL => "http://localhost:8000",
        NETWORK_IC => "https://ic0.app",
        url => url,
    }
}
