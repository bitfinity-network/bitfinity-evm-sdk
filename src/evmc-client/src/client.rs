use candid::utils::ArgumentEncoder;
use candid::{CandidType, Principal};
use did::block::BlockResult;
use did::{
    BasicAccount, BlockNumber, Bytes, Transaction, TransactionParams, TransactionReceipt, H160,
    H256, U256,
};
use ic_exports::icrc_types::icrc1::account::Subaccount;
use serde::Deserialize;

use crate::{CanisterClientError, CanisterClientResult, EvmResult};

/// Generic client for interacting with a canister.
/// This is used to abstract away the differences between the IC Agent and the
/// IC Canister.
/// The IC Agent is used for interaction through the dfx tool, while the IC
/// Canister is used for interacting with the EVM canister in wasm environments.
#[async_trait::async_trait]
pub trait CanisterClient {
    /// Call an update method on the canister.
    ///
    /// # Arguments
    ///
    /// * `method` - The method name.
    /// * `args` - The arguments to the method.
    ///
    /// # Returns
    ///
    /// The result of the method call.
    async fn update<T, R>(&self, method: &str, args: T) -> CanisterClientResult<R>
    where
        T: ArgumentEncoder + Send,
        R: for<'de> Deserialize<'de> + CandidType;

    /// Call a query method on the canister.
    ///
    /// # Arguments
    ///
    /// * `method` - The method name.
    /// * `args` - The arguments to the method.
    ///
    /// # Returns
    ///
    /// The result of the method call.
    async fn query<T, R>(&self, method: &str, args: T) -> CanisterClientResult<R>
    where
        T: ArgumentEncoder + Send,
        R: for<'de> Deserialize<'de> + CandidType;
}

/// An EVMC client.
#[derive(Debug)]
pub struct EvmcClient<C>
where
    C: CanisterClient,
{
    /// The canister client.
    client: C,
}

impl<C: CanisterClient> EvmcClient<C> {
    /// Create a new EVMC client.
    ///
    /// # Arguments
    /// * `client` - The canister client.
    pub fn new(client: C) -> Self {
        Self { client }
    }

    /// Returns the receipt of a transaction by transaction hash.
    /// See [eth_getTransactionReceipt](https://eth.wiki/json-rpc/API#eth_gettransactionreceipt).
    /// # Arguments
    ///
    /// * `hash` - The transaction hash.
    ///
    /// # Returns
    ///
    /// The transaction receipt.
    pub async fn eth_get_transaction_receipt(
        &self,
        hash: H256,
    ) -> Result<Option<TransactionReceipt>, CanisterClientError> {
        self.client
            .query("eth_get_transaction_receipt", (hash,))
            .await
    }

    /// Sends a raw transaction to the EVM canister
    /// See [eth_sendRawTransaction](https://eth.wiki/json-rpc/API#eth_sendrawtransaction)
    ///
    /// # Arguments
    /// * `transaction` - The transaction to send
    ///
    /// # Returns
    /// The hash of the transaction
    pub async fn send_raw_transaction(
        &self,
        transaction: Transaction,
    ) -> CanisterClientResult<EvmResult<H256>> {
        self.client
            .update("send_raw_transaction", (transaction,))
            .await
    }

    /// Calls a message on the EVM canister
    /// # Arguments
    /// * `tx_params` - The transaction parameters
    /// * `to` - The address of an account or of the contract to call
    /// * `data` - The data to send
    ///
    /// # Returns
    /// The hash of the transaction
    pub async fn call_message(
        &self,
        tx_params: TransactionParams,
        to: H160,
        data: String,
    ) -> CanisterClientResult<EvmResult<H256>> {
        self.client
            .update("call_message", (tx_params, to, data))
            .await
    }

    /// Creates a new contract on the EVM canister
    ///
    ///  # Arguments
    /// * `tx_params` - The transaction parameters
    /// * `data` - The data to send
    ///
    /// # Returns
    ///
    /// The hash of the transaction
    pub async fn create_contract(
        &self,
        tx_params: TransactionParams,
        data: String,
    ) -> CanisterClientResult<EvmResult<H256>> {
        self.client
            .update("create_contract", (tx_params, data))
            .await
    }

    /// Registers an IC agent on the EVM canister
    ///
    /// # Arguments
    /// * `transaction` - The transaction to send
    /// * `principal` - The principal to register
    ///
    /// # Returns
    /// Ok if the registration was successful
    ///
    /// # Fails
    /// * If the ic agent is already registered
    /// * If the agent does not have enough balance
    pub async fn register_ic_agent(
        &self,
        transaction: Transaction,
        principal: Principal,
    ) -> CanisterClientResult<EvmResult<()>> {
        self.client
            .update("register_ic_agent", (transaction, principal))
            .await
    }

    /// Get the Account information of an address
    ///
    /// # Arguments
    /// * `address` - The address of the account
    ///
    /// # Returns
    /// The account information
    ///  - nonce
    ///  - balance
    pub async fn account_basic(&self, address: H160) -> Result<BasicAccount, CanisterClientError> {
        self.client.query("account_basic", (address,)).await
    }

    /// Get the code of a contract
    /// See [eth_getCode](https://eth.wiki/json-rpc/API#eth_getcode)
    ///
    /// # Arguments
    /// * `address` - The address of the contract
    /// * `block_number` - The block number or tag
    ///
    /// # Returns
    ///
    /// The code of the contract
    pub async fn get_contract_code(
        &self,
        address: H160,
        block_number: BlockNumber,
    ) -> CanisterClientResult<EvmResult<String>> {
        self.client
            .query("eth_get_code", (address, block_number))
            .await
    }

    /// Deposit native tokens to the EVM canister
    ///
    /// # Arguments
    /// * `to` - The address of the recipient
    /// * `amount` - The amount to deposit
    ///
    /// # Returns
    /// The amount of tokens deposited
    pub async fn deposit(&self, to: H160, amount: U256) -> CanisterClientResult<EvmResult<U256>> {
        self.client.update("deposit_tokens", (to, amount)).await
    }

    /// Withdraw native tokens from the EVM canister
    ///
    /// # Arguments
    /// * `from` - The address of the sender
    /// * `to` - The address of the recipient
    /// * `amount` - The amount to withdraw
    ///
    /// # Returns
    ///
    /// The amount withdrawn
    pub async fn withdraw(
        &self,
        from: H160,
        to: Option<Subaccount>,
        amount: U256,
    ) -> CanisterClientResult<EvmResult<U256>> {
        self.client
            .update("withdraw_tokens", (from, to, amount))
            .await
    }

    /// Get the storage of a contract
    /// See [eth_getStorageAt](https://eth.wiki/json-rpc/API#eth_getstorageat)
    ///
    /// # Arguments
    /// * `address` - The address of the contract
    /// * `index` - The index of the storage
    /// * `block_number` - The block number or tag
    ///
    /// # Returns
    ///
    /// The storage of the contract
    pub async fn eth_get_storage_at(
        &self,
        address: H160,
        index: H256,
        block_number: BlockNumber,
    ) -> CanisterClientResult<EvmResult<H256>> {
        self.client
            .query("eth_get_storage_at", (address, index, block_number))
            .await
    }

    /// Verify if the signature is valid and the caller is registered
    ///
    /// # Arguments
    /// * `signing_key` - The signing key of the caller
    ///
    /// # Returns
    /// Ok if the signature is valid and the caller is registered
    ///
    /// # Fails
    /// * If the signature is invalid
    /// * If the caller is not registered
    pub async fn verify_registration(
        &self,
        signing_key: &[u8],
        principal: Principal,
    ) -> CanisterClientResult<EvmResult<()>> {
        self.client
            .update("verify_registration", (signing_key, principal))
            .await
    }

    /// Check if an address is registered
    pub async fn is_address_registered(
        &self,
        address: H160,
        principal: Principal,
    ) -> std::result::Result<bool, CanisterClientError> {
        self.client
            .query("is_address_registered", (address, principal))
            .await
    }

    /// Get the the transaction by hash
    /// See [eth_getTransactionByHash](https://eth.wiki/json-rpc/API#eth_gettransactionbyhash)
    ///
    /// # Arguments
    /// * `hash` - The hash of the transaction
    ///
    /// # Returns
    ///
    /// Result of the transaction or None if the transaction does not exist
    pub async fn eth_get_transaction_by_hash(
        &self,
        hash: H256,
    ) -> CanisterClientResult<Option<Transaction>> {
        self.client
            .query("eth_get_transaction_by_hash", (hash,))
            .await
    }

    /// Gets the transaction by block hash and transaction index position
    /// See [eth_getTransactionByBlockHashAndIndex](https://eth.wiki/json-rpc/API#eth_gettransactionbyblockhashandindex)
    ///
    /// # Arguments
    /// * `hash` - The hash of the block
    /// * `index` - The index of the transaction
    ///
    /// # Returns
    /// Result of the transaction or None if the transaction does not exist
    pub async fn eth_get_transaction_by_block_hash_and_index(
        &self,
        hash: H256,
        index: U256,
    ) -> CanisterClientResult<Option<Transaction>> {
        self.client
            .query("eth_get_transaction_by_block_hash_and_index", (hash, index))
            .await
    }

    /// Gets the transaction by block number and transaction index position
    /// See [eth_getTransactionByBlockNumberAndIndex](https://eth.wiki/json-rpc/API#eth_gettransactionbyblocknumberandindex)
    ///
    /// # Arguments
    /// * `block_number` - The block number or tag
    /// * `index` - The index of the transaction
    ///
    /// # Returns
    /// Result of the transaction or None if the transaction does not exist
    pub async fn eth_get_transaction_by_block_number_and_index(
        &self,
        block_number: BlockNumber,
        index: U256,
    ) -> CanisterClientResult<Option<Transaction>> {
        self.client
            .query(
                "eth_get_transaction_by_block_number_and_index",
                (block_number, index),
            )
            .await
    }

    /// Get the balance of an address
    /// See [eth_getBalance](https://eth.wiki/json-rpc/API#eth_getbalance)
    ///
    /// # Arguments
    /// * `address` - The address of the account
    /// * `block_number` - The block number or tag
    ///
    /// # Returns
    ///
    /// The balance of the account
    pub async fn eth_get_balance(
        &self,
        address: H160,
        block_number: BlockNumber,
    ) -> CanisterClientResult<EvmResult<U256>> {
        self.client
            .query("eth_get_balance", (address, block_number))
            .await
    }

    /// Mint Native to an address
    /// Note: This works on the testnet only
    pub async fn mint(&self, address: H160, amount: U256) -> CanisterClientResult<EvmResult<U256>> {
        self.client
            .update("mint_native_tokens", (address, amount))
            .await
    }

    /// Get the latest block number
    /// See [eth_blockNumber](https://eth.wiki/json-rpc/API#eth_blocknumber)
    ///
    /// # Returns
    /// The latest block number
    pub async fn eth_block_number(&self) -> Result<usize, CanisterClientError> {
        self.client.query("eth_block_number", ()).await
    }

    /// Get the block by hash
    /// See [eth_getBlockByHash](https://eth.wiki/json-rpc/API#eth_getblockbyhash)
    ///
    /// # Arguments
    /// * `hash` - The hash of the block
    /// * `include_transactions` - Whether to include the transactions in the
    /// block
    /// # Returns
    /// The block at the given hash
    pub async fn eth_get_block_by_hash(
        &self,
        hash: H256,
        include_transactions: bool,
    ) -> CanisterClientResult<EvmResult<BlockResult>> {
        self.client
            .query("eth_get_block_by_hash", (hash, include_transactions))
            .await
    }

    /// Get the block by number
    /// See [eth_getBlockByNumber](https://eth.wiki/json-rpc/API#eth_getblockbynumber)
    ///
    /// # Arguments
    /// * `block_number` - The block number or tag
    /// * `include_transactions` - Whether to include the transactions in the
    /// block
    ///
    /// # Returns
    ///
    /// The block at the given block number or tag
    pub async fn eth_get_block_by_number(
        &self,
        block_number: BlockNumber,
        include_transactions: bool,
    ) -> CanisterClientResult<EvmResult<BlockResult>> {
        self.client
            .query(
                "eth_get_block_by_number",
                (block_number, include_transactions),
            )
            .await
    }

    /// Get the transaction count of an address at a given block number
    /// See [eth_getTransactionCount](https://eth.wiki/json-rpc/API#eth_gettransactioncount)
    ///
    /// # Arguments
    ///
    /// * `address` - The address to get the transaction count for
    /// * `block_number` - The block number to get the transaction count at
    ///
    /// # Returns
    ///
    /// The transaction count of the address at the given block number
    pub async fn eth_get_transaction_count(
        &self,
        address: H160,
        block_number: BlockNumber,
    ) -> CanisterClientResult<EvmResult<U256>> {
        self.client
            .query("eth_get_transaction_count", (address, block_number))
            .await
    }

    /// Execute a call on the EVM without modifying the state
    /// See [eth_call](https://eth.wiki/json-rpc/API#eth_call)
    ///
    /// # Arguments
    /// * `from` - The address of the caller
    /// * `to` - The address of the contract to call
    /// * `value` - The value to send to the contract
    /// * `gas_limit` - The gas limit for the call
    /// * `gas_price` - The gas price for the call
    /// * `data` - The data to send to the contract
    ///
    /// # Returns
    ///
    /// The result of the call
    pub async fn eth_call(
        &self,
        from: Option<H160>,
        to: Option<H160>,
        value: Option<U256>,
        gas_limit: u64,
        gas_price: Option<U256>,
        data: Option<Bytes>,
    ) -> CanisterClientResult<EvmResult<String>> {
        self.client
            .query("eth_call", (from, to, value, gas_limit, gas_price, data))
            .await
    }
}
