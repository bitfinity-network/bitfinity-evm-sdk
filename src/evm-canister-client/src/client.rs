use candid::Principal;
use did::block::BlockResult;
use did::{BasicAccount, BlockNumber, Bytes, Transaction, TransactionReceipt, H160, H256, U256};
use ic_canister_client::{CanisterClient, CanisterClientResult};
use ic_exports::icrc_types::icrc1::account::Subaccount;

use crate::EvmResult;

/// An EVM canister client.
#[derive(Debug)]
pub struct EvmCanisterClient<C>
where
    C: CanisterClient,
{
    /// The canister client.
    client: C,
}

impl<C: CanisterClient> EvmCanisterClient<C> {
    /// Create a new EVM canister client.
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
    ) -> CanisterClientResult<EvmResult<Option<TransactionReceipt>>> {
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

    /// Get the Account information of an address
    ///
    /// # Arguments
    /// * `address` - The address of the account
    ///
    /// # Returns
    /// The account information
    ///  - nonce
    ///  - balance
    pub async fn account_basic(&self, address: H160) -> CanisterClientResult<BasicAccount> {
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
    pub async fn eth_get_code(
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
    ) -> CanisterClientResult<EvmResult<Option<Transaction>>> {
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
    pub async fn mint_native_tokens(
        &self,
        address: H160,
        amount: U256,
    ) -> CanisterClientResult<EvmResult<U256>> {
        self.client
            .update("mint_native_tokens", (address, amount))
            .await
    }

    /// Get the latest block number
    /// See [eth_blockNumber](https://eth.wiki/json-rpc/API#eth_blocknumber)
    ///
    /// # Returns
    /// The latest block number
    pub async fn eth_block_number(&self) -> CanisterClientResult<usize> {
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

    /// Estimate the gas for a call
    /// See [eth_estimateGas](https://eth.wiki/json-rpc/API#eth_estimategas)
    ///
    /// # Arguments
    ///
    /// * `from` - The address of the caller
    /// * `to` - The address of the contract to call
    /// * `gas_limit` - The gas limit for the call
    /// * `value` - The value to send to the contract
    /// * `input` - The data to send to the contract
    ///
    /// # Returns
    ///
    /// The estimated gas for the call
    pub async fn eth_estimate_gas(
        &self,
        from: H160,
        to: Option<H160>,
        gas_limit: u64,
        value: U256,
        input: Bytes,
    ) -> CanisterClientResult<EvmResult<U256>> {
        self.client
            .query(
                "eth_estimate_gas",
                (from, to, gas_limit, value, gas_limit, input),
            )
            .await
    }

    /// Get the transaction count at a given block hash
    /// See [eth_getBlockTransactionCountByHash](https://eth.wiki/json-rpc/
    /// API#eth_getblocktransactioncountbyhash)
    ///
    /// # Arguments
    ///
    /// * `hash` - The block hash
    ///
    /// # Returns
    ///
    /// The transaction count of the address at the given block number
    pub async fn eth_get_block_transaction_count_by_hash(
        &self,
        hash: H256,
    ) -> CanisterClientResult<usize> {
        self.client
            .query("eth_get_block_transaction_count_by_hash", (hash,))
            .await
    }

    /// Get the transaction count at a given block number
    /// See [eth_getBlockTransactionCountByNumber](https://eth.wiki/json-rpc/
    /// API#eth_getblocktransactioncountbynumber)
    ///
    /// # Arguments
    ///
    /// * `block_number` - The block number or tag
    ///
    /// # Returns
    ///
    /// The transaction count of the address at the given block number
    pub async fn eth_get_block_transaction_count_by_block_number(
        &self,
        block_number: BlockNumber,
    ) -> CanisterClientResult<EvmResult<usize>> {
        self.client
            .query("eth_get_block_transaction_count_by_number", (block_number,))
            .await
    }

    /// Reserves address for a given principal
    ///
    /// This is two step process:
    /// 1. Send a transaction using the `send_raw_transaction` method,
    /// attaching the principal that should be reserved as input
    ///
    /// 2. Call this method with the principal and the transaction hash from
    /// the previous step
    ///
    /// # Arguments
    /// * `principal` - The principal to reserve address for
    /// * `tx_hash` - The transaction hash of the transaction that reserved the
    /// address
    pub async fn reserve_address(
        &self,
        principal: Principal,
        tx_hash: H256,
    ) -> CanisterClientResult<EvmResult<()>> {
        self.client
            .update("reserve_address", (principal, tx_hash))
            .await
    }

    /// Checks if address with given principal is reserved
    ///
    /// # Arguments
    /// * `principal` - The principal to check
    /// * `address` - The address to check
    ///
    /// # Returns
    ///
    /// True if address is reserved, false otherwise
    pub async fn is_address_reserved(
        &self,
        principal: Principal,
        address: H160,
    ) -> CanisterClientResult<bool> {
        self.client
            .query("is_address_reserved", (principal, address))
            .await
    }

    /// Returns the chain ID used for signing replay-protected transactions.
    /// See [eth_chainid] (https://eth.wiki/json-rpc/API#eth_chainid)
    ///
    /// # Arguments
    /// None
    ///
    /// # Returns
    ///
    /// `chainId`, hexadecimal value as a string representing the integer of the current chain id.
    pub async fn eth_chain_id(&self) -> CanisterClientResult<u64> {
        self.client.query("eth_chain_id", ()).await
    }
}
