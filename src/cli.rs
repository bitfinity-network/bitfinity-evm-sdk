use crate::agent::{init_agent, user_principal};
use anyhow::{bail, Result};
use candid::{Decode, Encode, Principal};
use clap::{Args, Parser, Subcommand};
use did::error::EvmError;
use did::{Transaction, H160};
use eth_signer::{Signer, Wallet};
use ethers_core::k256::ecdsa::SigningKey;
use ethers_core::types::transaction::eip2718::TypedTransaction;
use ethers_core::types::TransactionRequest;
use evm_adapter::constant::{CHAIN_ID, MINTER_ADDRESS, REGISTRATION_FEE};
use ic_agent::Agent;

const AMOUNT_TO_MINT: u128 = 10_u128.pow(18);

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
    /// Path to your identity pem file
    pub identity: String,

    /// Evmc canister principal
    pub evmc: Principal,

    /// Minter canister principal
    pub minter: Principal,

    /// wallet signing key
    #[arg(short = 'k', long = "key")]
    pub signing_key: Option<String>,

    /// IC Network (ic, local or custom url)
    #[arg(short, long, default_value_t = String::from("local"))]
    pub network: String,
}

impl RegisterArgs {
    pub async fn exec(&self) -> Result<()> {
        let evmc_canister_id = self.evmc;
        let minter_canister_id = self.minter;
        let signing_key = self.signing_key.as_deref();
        let agent = init_agent(&self.identity, &self.network).await?;
        let wallet = get_wallet(signing_key)?;
        register_wallet(
            &agent,
            &self.network,
            &evmc_canister_id,
            &minter_canister_id,
            &wallet,
        )
        .await?;

        Ok(())
    }
}

/// Generate a new wallet or parse an existing one.
pub fn get_wallet<'a>(signing_key: Option<&str>) -> Result<Wallet<'a, SigningKey>> {
    match signing_key {
        Some(key_hex) => {
            let key_bytes = hex::decode(key_hex)?;
            let wallet = Wallet::from_bytes(&key_bytes)?;
            Ok(wallet)
        }
        None => {
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
    }
}

/// Register the given `wallet` to the caller `principal`.
pub async fn register_wallet(
    agent: &Agent,
    network: &str,
    evmc_canister_id: &Principal,
    minter_canister_id: &Principal,
    wallet: &Wallet<'static, SigningKey>,
) -> Result<()> {
    let address: H160 = wallet.address().into();
    let principal = user_principal(agent)?;
    let is_registered = is_address_registered(agent, evmc_canister_id, &address).await?;
    if is_registered {
        println!(
            "Already registered:\n\tWallet Address = {}\n\tPrincipal = {}",
            address.to_hex_str(),
            principal
        );
        return Ok(());
    }

    let tx = registration_transaction(wallet).await?;
    let args = Encode!(&Transaction::from(tx), &wallet.signer().to_bytes().to_vec())?;

    // mint tokens to be able to pay registration fee (only on testnets)
    match network {
        n if n != "ic" => mint_evm_tokens_to_address(agent, evmc_canister_id, &address).await?,
        _ => (),
    }

    agent
        .update(minter_canister_id, "register")
        .with_arg(args)
        .call_and_wait()
        .await?;

    println!(
        "Registration is successful:\n  Wallet Address = {}\n  Principal = {}",
        address.to_hex_str(),
        minter_canister_id
    );

    Ok(())
}

/// check if address is registered to principal
pub async fn is_address_registered(
    agent: &Agent,
    evmc_canister_id: &Principal,
    address: &H160,
) -> Result<bool> {
    let args = Encode!(address)?;
    let res = agent
        .query(evmc_canister_id, "is_address_registered")
        .with_arg(args)
        .call()
        .await?;
    let principal = user_principal(agent)?;
    match Decode!(res.as_slice(), bool) {
        Ok(res) => Ok(res),
        Err(_) => bail!(
            "Failed to check registration status:\n  Wallet Address = {}\n  Principal = {}",
            address.to_hex_str(),
            principal
        ),
    }
}

/// Returns a new registration transaction for the given `wallet`.
pub async fn registration_transaction(
    wallet: &Wallet<'_, SigningKey>,
) -> Result<ethers_core::types::Transaction> {
    let address = H160::from(wallet.address());

    // Create and sign the registration transaction
    let to = ethers_core::types::H160::from(MINTER_ADDRESS.clone());
    let tx: TypedTransaction = TransactionRequest::new()
        .from(address.clone())
        .to(to)
        .value(REGISTRATION_FEE)
        .chain_id(CHAIN_ID)
        .nonce(0)
        .gas_price(0)
        .gas(53000)
        .into();
    let signature = wallet.sign_transaction(&tx).await?;

    // register principal
    let bytes = tx.rlp_signed(&signature);
    let mut tx: ethers_core::types::Transaction = rlp::decode(&bytes)?;
    tx.from = address.into();
    Ok(tx)
}

/// mint native tokens to wallet address as registration fee.
pub async fn mint_evm_tokens_to_address(
    agent: &Agent,
    evmc_canister_id: &Principal,
    to: &H160,
) -> Result<()> {
    let payload = Encode!(to, &did::U256::from(AMOUNT_TO_MINT))?;

    let res = agent
        .update(evmc_canister_id, "mint_evm_tokens")
        .with_arg(payload)
        .call_and_wait()
        .await?;

    Decode!(res.as_slice(), std::result::Result<did::U256, EvmError>)??;

    Ok(())
}
