use std::path::PathBuf;

use anyhow::Result;
use candid::Principal;
use clap::{Args, Parser, Subcommand};
use did::H160;
use eth_signer::{Signer, Wallet};
use ethers_core::k256::ecdsa::SigningKey;

use super::registration::RegistrationService;
use crate::constant::{DEFAULT_CHAIN_ID, NETWORK_LOCAL};
use crate::error::Error;

/// CLI tool for generating wallet & registering minter principal to the evmc
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct RegisterMinterCli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate an ETH Wallet
    GenerateWallet,

    /// Register a minter principal to the evmc
    Register(RegisterArgs),
}

#[derive(Args)]
pub struct RegisterArgs {
    /// chain id
    #[arg(short = 'c', long = "chain-id", default_value_t = DEFAULT_CHAIN_ID)]
    pub chain_id: u64,

    /// Path to your identity pem file
    pub identity: PathBuf,

    /// Evmc canister principal
    pub evmc: Principal,

    /// Principal of the canister to register
    pub register_canister_id: Principal,

    /// wallet signing key
    #[arg(short = 'k', long = "key")]
    pub signing_key: Option<String>,

    /// IC Network (ic, local or custom url)
    #[arg(short, long, default_value_t = String::from(NETWORK_LOCAL))]
    pub network: String,
}

impl RegisterArgs {
    pub async fn exec(&self) -> Result<()> {
        let wallet = get_wallet(self.signing_key.as_deref())?;
        let address = wallet.address();

        match RegistrationService::new(
            self.chain_id,
            self.evmc,
            self.register_canister_id,
            wallet,
            &self.identity,
            self.network.clone(),
        )
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .register()
        .await
        {
            Ok(()) => {
                println!(
                    "Registration succeeded:\n  Wallet Address = {}\n  Principal = {}",
                    H160::from(address).to_hex_str(),
                    self.register_canister_id
                );

                Ok(())
            }
            Err(Error::AlreadyRegistered(principal)) => {
                println!(
                    "Already registered:\n\tWallet Address = {}\n\tPrincipal = {}",
                    H160::from(address).to_hex_str(),
                    principal
                );
                Ok(())
            }
            Err(err) => anyhow::bail!("{err}"),
        }
    }
}

/// Generate a new wallet or parse an existing one.
fn get_wallet<'a>(signing_key: Option<&str>) -> Result<Wallet<'a, SigningKey>> {
    match signing_key {
        Some(key_hex) => {
            let key_bytes = hex::decode(key_hex)?;
            let wallet = Wallet::from_bytes(&key_bytes)?;
            Ok(wallet)
        }
        None => generate_wallet(),
    }
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
