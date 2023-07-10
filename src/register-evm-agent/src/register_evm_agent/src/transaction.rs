use crate::agent::init_agent;
use crate::cli::{get_wallet, network_url};
use crate::cli::{DEFAULT_CHAIN_ID, NETWORK_LOCAL};
use crate::constant::{DEFAULT_GAS_LIMIT, METHOD_ACCOUNT_BASIC, METHOD_MIN_GAS_PRICE};
use crate::error::Result;
use candid::{Decode, Encode, Principal};
use clap::Args;
use did::transaction::TransactionBuilder;
use did::BasicAccount;
use eth_signer::{Signer, Wallet};
use ethers_core::k256::ecdsa::SigningKey;
use ethers_core::types::{H160, U256};
use ic_agent::Agent;
use std::path::PathBuf;
use std::str::FromStr;

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

    /// Evmc canister principal
    #[arg(short = 'e', long = "evmc")]
    pub evmc: Principal,

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
        let wallet = get_wallet(self.signing_key.as_str())?;

        let tx = self.transaction_builder(wallet, &agent).await?;

        let tx_bytes = ethers_core::types::Transaction::from(tx.clone()).rlp();

        //    Pretty print the transaction
        println!("Transaction: {:#?}", tx);
        println!("Transaction Bytes: {:#?}", tx_bytes);

        Ok(())
    }
    async fn transaction_builder(
        &self,
        wallet: Wallet<'_, SigningKey>,
        agent: &Agent,
    ) -> Result<did::Transaction> {
        let address = wallet.address();

        let nonce = match self.nonce {
            Some(n) => did::U256::from(n),
            None => self.basic_account(agent, &address.into()).await?.nonce,
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
            signature: did::transaction::SigningMethod::SigningKey(wallet.signer()),
            chain_id: DEFAULT_CHAIN_ID,
        }
        .calculate_hash_and_build()?;

        Ok(tx)
    }
    async fn basic_account(&self, agent: &Agent, address: &did::H160) -> Result<BasicAccount> {
        let args = Encode!(&address)?;

        let res = agent
            .query(&self.evmc, METHOD_ACCOUNT_BASIC)
            .with_arg(args)
            .call()
            .await?;

        let res = Decode!(res.as_slice(), BasicAccount)?;

        Ok(res)
    }

    async fn min_gas_price(&self, agent: &Agent) -> Result<did::U256> {
        let res = agent.query(&self.evmc, METHOD_MIN_GAS_PRICE).call().await?;

        let res = Decode!(res.as_slice(), did::U256)?;

        Ok(res)
    }
}
