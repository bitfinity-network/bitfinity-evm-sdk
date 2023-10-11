use std::path::PathBuf;
use std::str::FromStr;

use candid::Principal;
use clap::Args;
use eth_signer::transaction::TransactionBuilder;
use eth_signer::{Signer, Wallet};
use ethers_core::k256::ecdsa::SigningKey;
use ethers_core::types::{H160, U256};
use evm_canister_client::{EvmCanisterClient, IcAgentClient};
use evm_canister_client::agent::identity::init_agent;

use crate::cli::{get_wallet, network_url, DEFAULT_CHAIN_ID, NETWORK_LOCAL};
use crate::constant::DEFAULT_GAS_LIMIT;
use crate::error::Result;

type EvmCanisterAgentClient = EvmCanisterClient<IcAgentClient>;

#[derive(Args)]
pub struct SignTransactionArgs {
    /// Path to your identity pem file
    #[arg(short = 'i', long = "identity")]
    pub identity: PathBuf,

    /// IC Network (ic, local or custom url)
    #[arg(short, long, default_value_t = String::from(NETWORK_LOCAL))]
    pub network: String,

    /// wallet signing key
    #[arg(short = 'k', long = "key")]
    pub signing_key: String,

    /// Evm canister principal
    #[arg(short = 'e', long = "evm")]
    pub evm: Principal,

    /// Address of the recipient
    #[arg(short = 't', long = "transaction")]
    pub to: Option<String>,

    #[arg(short = 'v', long = "value")]
    pub value: Option<u128>,

    #[arg(short = 'g', long = "gas")]
    pub gas: Option<u128>,

    #[arg(short = 'l', long = "gas-price")]
    pub gas_price: Option<u128>,

    #[arg(short = 'n', long = "nonce")]
    pub nonce: Option<u128>,

    #[arg(short = 'd', long = "data")]
    pub data: Option<String>,
}

impl SignTransactionArgs {
    pub async fn exec(&self) -> anyhow::Result<()> {
        info!("initializing agent...");
        let network = network_url(&self.network);
        let agent = init_agent(&self.identity, network).await?;
        let client = EvmCanisterClient::new(IcAgentClient::with_agent(self.evm, agent));
        let wallet = get_wallet(self.signing_key.as_str())?;

        let tx = self.transaction_builder(wallet, &client).await?;

        let tx_bytes = ethers_core::types::Transaction::from(tx.clone()).rlp();

        println!("Transaction: {:#?}", tx);
        println!("Transaction Bytes: {:#?}", tx_bytes);

        Ok(())
    }
    async fn transaction_builder(
        &self,
        wallet: Wallet<'_, SigningKey>,
        client: &EvmCanisterAgentClient,
    ) -> Result<did::Transaction> {
        let address = wallet.address();

        let nonce = match self.nonce {
            Some(n) => did::U256::from(n),
            None => client.account_basic(address.into()).await?.nonce,
        };

        let tx = TransactionBuilder {
            from: &address.into(),
            to: self
                .to
                .clone()
                .map(|address| H160::from_str(&address).expect("address invalid").into()),
            nonce,
            value: self.value.map(U256::from).unwrap_or_default().into(),
            gas: self
                .gas
                .map(U256::from)
                .unwrap_or(DEFAULT_GAS_LIMIT.into())
                .into(),
            gas_price: self.gas_price.map(did::U256::from),
            input: self
                .data
                .clone()
                .map(|v| hex::decode(v).expect("data invalid"))
                .unwrap_or_default(),
            signature: eth_signer::transaction::SigningMethod::SigningKey(wallet.signer()),
            chain_id: DEFAULT_CHAIN_ID,
        }
        .calculate_hash_and_build()?;

        Ok(tx)
    }
}
