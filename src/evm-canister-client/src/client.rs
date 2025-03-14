use candid::Principal;
use did::block::BlockResult;
use did::build::BuildData;
use did::error::Result;
use did::evm_state::{EvmGlobalState, EvmResetState};
use did::permission::{Permission, PermissionList};
use did::revert_blocks::RevertToBlockArgs;
use did::send_raw_transaction::SendRawTransactionRequest;
use did::state::BasicAccount;
use did::transaction::StorableExecutionResult;
use did::unsafe_blocks::ValidateUnsafeBlockArgs;
use did::{
    Block, BlockConfirmationData, BlockConfirmationResult, BlockConfirmationStrategy, BlockNumber,
    BlockchainBlockInfo, BlockchainStorageLimits, Bytes, EstimateGasRequest, EvmStats, Transaction,
    TransactionReceipt, H160, H256, U256, U64,
};
use ic_canister_client::{CanisterClient, CanisterClientResult};
pub use ic_log::writer::{Log, Logs};

use crate::EvmResult;

/// An EVM canister client.
#[derive(Debug, Clone)]
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
    ) -> CanisterClientResult<Option<TransactionReceipt>> {
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
        transaction: SendRawTransactionRequest,
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
    pub async fn admin_mint_native_tokens(
        &self,
        address: H160,
        amount: U256,
    ) -> CanisterClientResult<EvmResult<(H256, U256)>> {
        self.client
            .update("admin_mint_native_tokens", (address, amount))
            .await
    }

    /// Get the latest block number
    /// See [eth_blockNumber](https://eth.wiki/json-rpc/API#eth_blocknumber)
    ///
    /// # Returns
    /// The latest block number
    pub async fn eth_block_number(&self) -> CanisterClientResult<u64> {
        self.client.query("eth_block_number", ()).await
    }

    /// Get the block by hash
    /// See [eth_getBlockByHash](https://eth.wiki/json-rpc/API#eth_getblockbyhash)
    ///
    /// # Arguments
    /// * `hash` - The hash of the block
    /// * `include_transactions` - Whether to include the transactions in the
    ///   block
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
    ///   block
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

    /// Get the blocks range by number using a single request.
    ///
    /// # Arguments
    /// * `from` - The index of the first block
    /// * `count` - Number of blocks to return
    /// * `include_transactions` - Whether to include the transactions in the
    ///   block
    ///
    /// # Returns
    ///
    /// The block at the given block number or tag
    pub async fn get_blocks_by_number(
        &self,
        from: U64,
        count: U64,
        include_transactions: bool,
    ) -> CanisterClientResult<EvmResult<Vec<BlockResult>>> {
        self.client
            .query(
                "eth_get_blocks_by_number",
                (from, count, include_transactions),
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
    /// * `request` - The request to estimate the gas for the call
    /// # Returns
    ///
    /// The estimated gas for the call
    pub async fn eth_estimate_gas(
        &self,
        request: EstimateGasRequest,
    ) -> CanisterClientResult<EvmResult<U256>> {
        self.client.query("eth_estimate_gas", (request,)).await
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
    /// The transaction count at a given block number
    pub async fn eth_get_block_transaction_count_by_number(
        &self,
        block_number: BlockNumber,
    ) -> CanisterClientResult<EvmResult<usize>> {
        self.client
            .query("eth_get_block_transaction_count_by_number", (block_number,))
            .await
    }

    /// Returns the current logger filter
    pub async fn get_logger_filter(&self) -> CanisterClientResult<Option<String>> {
        self.client.query("get_logger_filter", ()).await
    }

    /// Updates the runtime configuration of the logger with a new filter in the same form as the `RUST_LOG`
    /// environment variable.
    /// Example of valid filters:
    /// - info
    /// - debug,crate1::mod1=error,crate1::mod2,crate2=debug
    ///
    /// # Arguments
    ///
    /// * `filter` - The new filter.
    pub async fn set_logger_filter(&self, filter: &str) -> CanisterClientResult<Result<()>> {
        self.client.update("set_logger_filter", (filter,)).await
    }

    /// Gets the application logs
    /// - `count` is the number of logs to return
    /// - `offset` is the offset from the first log to return
    pub async fn ic_logs(&self, count: usize, offset: usize) -> CanisterClientResult<Result<Logs>> {
        self.client.query("ic_logs", (count, offset)).await
    }

    /// Returns the global state of the EVM.
    pub async fn get_evm_global_state(&self) -> CanisterClientResult<EvmGlobalState> {
        self.client.query("get_evm_global_state", ()).await
    }

    /// Sets the global state of the EVM.
    pub async fn admin_set_evm_global_state(
        &self,
        state: EvmGlobalState,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_set_evm_global_state", (state,))
            .await
    }

    /// Adds permissions to a principal and returns the principal permissions
    pub async fn admin_ic_permissions_add(
        &self,
        principal: Principal,
        permissions: Vec<Permission>,
    ) -> CanisterClientResult<Result<PermissionList>> {
        self.client
            .update("admin_ic_permissions_add", (principal, permissions))
            .await
    }

    /// Removes permissions from a principal and returns the principal permissions
    pub async fn admin_ic_permissions_remove(
        &self,
        principal: Principal,
        permissions: Vec<Permission>,
    ) -> CanisterClientResult<Result<PermissionList>> {
        self.client
            .update("admin_ic_permissions_remove", (principal, permissions))
            .await
    }

    /// Returns the permissions of a principal
    pub async fn admin_ic_permissions_get(
        &self,
        principal: Principal,
    ) -> CanisterClientResult<Result<PermissionList>> {
        self.client
            .query("admin_ic_permissions_get", (principal,))
            .await
    }

    /// Sets the min gas price
    pub async fn admin_set_min_gas_price(
        &self,
        min_gas_price: u128,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_set_min_gas_price", (min_gas_price,))
            .await
    }

    /// Enable or disable unsafe blocks. This function requires admin permissions.
    pub async fn admin_enable_unsafe_blocks(
        &self,
        enabled: bool,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_enable_unsafe_blocks", (enabled,))
            .await
    }

    /// Returns whether unsafe blocks are enabled
    pub async fn is_unsafe_blocks_enabled(&self) -> CanisterClientResult<bool> {
        self.client.query("is_unsafe_blocks_enabled", ()).await
    }

    /// Validate unsafe block on the EVM.
    ///
    /// Only those with [`did::permission::Permission::ValidateUnsafeBlocks`] can call this method.
    pub async fn validate_unsafe_block(
        &self,
        args: ValidateUnsafeBlockArgs,
    ) -> CanisterClientResult<Result<()>> {
        self.client.update("validate_unsafe_block", (args,)).await
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

    /// Returns the block gas limit. This is the maximum amount of gas that can
    /// be used in a block.
    pub async fn get_block_gas_limit(&self) -> CanisterClientResult<u64> {
        self.client.query("get_block_gas_limit", ()).await
    }

    /// Returns the block size limit.
    pub async fn get_block_size_limit(&self) -> CanisterClientResult<u64> {
        self.client.query("get_block_size_limit", ()).await
    }

    /// Returns the history size. This is the number of blocks for which any
    /// EVM state-related information can be acquired.
    pub async fn get_evm_state_history_size(&self) -> CanisterClientResult<u64> {
        self.client.query("get_evm_state_history_size", ()).await
    }

    /// Returns the min gas price
    pub async fn get_min_gas_price(&self) -> CanisterClientResult<u128> {
        self.client.query("get_min_gas_price", ()).await
    }

    /// Returns the min gas price
    pub async fn get_genesis_accounts(&self) -> CanisterClientResult<Vec<(H160, U256)>> {
        self.client.query("get_genesis_accounts", ()).await
    }

    /// Returns the list of eth accounts owned by the client.
    pub async fn eth_accounts(&self) -> CanisterClientResult<Vec<H160>> {
        self.client.query("eth_accounts", ()).await
    }

    /// Returns Keccak-256 (not the standardized SHA3-256) of the given data.
    pub async fn web3_sha3(&self, data: String) -> CanisterClientResult<EvmResult<String>> {
        self.client.query("web3_sha3", (data,)).await
    }

    /// Returns the current client version.
    pub async fn web3_client_version(&self) -> CanisterClientResult<String> {
        self.client.query("web3_client_version", ()).await
    }

    /// Returns number of peers currently connected to the client.
    pub async fn net_peer_count(&self) -> CanisterClientResult<u64> {
        self.client.query("net_peer_count", ()).await
    }

    /// Returns an object with data about the sync status or false.
    pub async fn eth_syncing(&self) -> CanisterClientResult<bool> {
        self.client.query("eth_syncing", ()).await
    }

    /// Returns true if client is actively mining new blocks.
    pub async fn eth_mining(&self) -> CanisterClientResult<bool> {
        self.client.query("eth_mining", ()).await
    }

    /// Returns the number of hashes per second that the node is mining with.
    pub async fn eth_hashrate(&self) -> CanisterClientResult<u64> {
        self.client.query("eth_hashrate", ()).await
    }

    /// Returns the current network id.
    pub async fn net_version(&self) -> CanisterClientResult<u64> {
        self.client.query("net_version", ()).await
    }

    /// Returns the current Ethereum protocol version.
    pub async fn eth_protocol_version(&self) -> CanisterClientResult<u64> {
        self.client.query("eth_protocol_version", ()).await
    }

    /// Returns true if client is actively listening for network connections.
    pub async fn net_listening(&self) -> CanisterClientResult<bool> {
        self.client.query("net_listening", ()).await
    }

    /// Returns the max batch requests. This is the maximum amount of requests allowed in a batch
    pub async fn get_max_batch_requests(&self) -> CanisterClientResult<u32> {
        self.client.query("get_max_batch_requests", ()).await
    }

    /// Sets the max batch requests. This is the maximum amount of requests allowed in a batch
    pub async fn admin_set_max_batch_requests(
        &self,
        size: u32,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_set_max_batch_requests", (size,))
            .await
    }

    /// Sets the block gas limit. This is the maximum amount of gas that can be
    /// used in a block.
    /// This function can only be called by the admin.
    ///
    /// # Arguments
    /// * `limit` - the new block gas limit, this must be greater than 0
    ///
    /// # Errors
    /// * `Internal` - if the block gas limit is 0
    /// * `NotAuthorized` - if the caller is not the admin
    pub async fn admin_set_block_gas_limit(&self, limit: u64) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_set_block_gas_limit", (limit,))
            .await
    }

    /// Sets the history size - the number of blocks for which any
    /// EVM state-related information can be acquired.
    pub async fn admin_set_evm_state_history_size(
        &self,
        history_size: u64,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_set_evm_state_history_size", (history_size,))
            .await
    }

    /// Returns the execution result of a transaction by transaction hash.
    pub async fn get_tx_execution_result_by_hash(
        &self,
        hash: H256,
    ) -> CanisterClientResult<Option<StorableExecutionResult>> {
        self.client
            .query("get_tx_execution_result_by_hash", (hash,))
            .await
    }

    /// Returns the build data of the canister.
    pub async fn get_canister_build_data(&self) -> CanisterClientResult<BuildData> {
        self.client.query("get_canister_build_data", ()).await
    }

    /// Disables/Enables the processing of transactions.
    pub async fn admin_disable_process_pending_transactions(
        &self,
        value: bool,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_disable_process_pending_transactions", (value,))
            .await
    }

    /// Returns the current status of the processing of transactions.
    pub async fn is_process_pending_transactions_disabled(&self) -> CanisterClientResult<bool> {
        self.client
            .query("is_process_pending_transactions_disabled", ())
            .await
    }

    /// Enable/Disable creation of empty blocks.
    pub async fn admin_allow_empty_blocks(&self, value: bool) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_allow_empty_blocks", (value,))
            .await
    }

    /// Returns the current status of the creation of empty blocks.
    pub async fn is_empty_block_enabled(&self) -> CanisterClientResult<bool> {
        self.client.query("is_empty_block_enabled", ()).await
    }

    /// Manages the reset state of the EVM.
    /// This endpoint is for initializing the EVM state or for recovering from a corrupted state.
    pub async fn reset_state(&self, state: EvmResetState) -> CanisterClientResult<Result<()>> {
        self.client.update("reset_state", (state,)).await
    }

    /// Disable/Enable the inspect message
    pub async fn admin_disable_inspect_message(
        &self,
        value: bool,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_disable_inspect_message", (value,))
            .await
    }

    /// Returns whether the inspect message is disabled.
    pub async fn is_inspect_message_disabled(&self) -> CanisterClientResult<bool> {
        self.client.query("is_inspect_message_disabled", ()).await
    }

    /// Returns the current transaction processing interval in seconds
    pub async fn get_transaction_processing_interval_secs(&self) -> CanisterClientResult<u64> {
        self.client
            .query("get_transaction_processing_interval_secs", ())
            .await
    }

    /// Sets the transaction processing interval.
    /// This function can only be called by the admin.
    ///
    /// # Arguments
    /// * `secs` - the new transaction processing interval in seconds
    ///
    /// # Errors
    /// * `NotAuthorized` - if the caller is not the admin
    pub async fn admin_set_transaction_processing_interval_secs(
        &self,
        secs: u64,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_set_transaction_processing_interval_secs", (secs,))
            .await
    }

    /// Returns number of WASM pages allocated for the given memory id
    pub async fn memory_pages_allocated_for_id(&self, memory_id: u8) -> CanisterClientResult<u64> {
        self.client
            .query("memory_pages_allocated_for_id", (memory_id,))
            .await
    }

    /// Set the blockchain size limit for transactions, receipts, and blocks.
    pub async fn admin_set_blockchain_size_limit(
        &self,
        limit: BlockchainStorageLimits,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_set_blockchain_size_limit", (limit,))
            .await
    }

    /// Returns the current blockchain size limit for transactions, receipts, and blocks.
    pub async fn get_blockchain_size_limit(&self) -> CanisterClientResult<BlockchainStorageLimits> {
        self.client.query("get_blockchain_size_limit", ()).await
    }

    /// Sets the block size limit.
    pub async fn admin_set_block_size_limit(
        &self,
        block_size: u64,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_set_block_size_limit", (block_size,))
            .await
    }

    /// Allocates `pages` additional pages of stable storage memory.
    /// Returns error in case when failed to allocate the missing pages.
    pub async fn admin_reserve_stable_storage_pages(
        &self,
        pages: u64,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_reserve_stable_storage_pages", (pages,))
            .await
    }

    /// Returns `EvmStats` which contains statistics for the Network
    pub async fn ic_stats(&self) -> CanisterClientResult<Result<EvmStats>> {
        self.client.query("ic_stats", ()).await
    }

    /// Disable/Enable the transaction pre-execution
    pub async fn admin_disable_tx_pre_execution(
        &self,
        value: bool,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_disable_tx_pre_execution", (value,))
            .await
    }

    /// Returns whether the transaction pre-execution is disabled.
    pub async fn is_tx_pre_execution_disabled(&self) -> CanisterClientResult<bool> {
        self.client.query("is_tx_pre_execution_disabled", ()).await
    }

    /// Appends a new block to the blockchain.
    pub async fn append_block(
        &self,
        block: Block<Transaction>,
    ) -> CanisterClientResult<Result<()>> {
        self.client.update("append_block", (block,)).await
    }

    /// Drops all blocks up to the `block_number`.
    ///
    /// Only the blocks for which state history is preserved can be reverted.
    ///
    /// # Errors
    ///
    /// * [`EvmError::BlockDoesNotExist`] if given block number is higher than the tip of the
    ///   chain.
    /// * [`EvmError::NoHistoryDataForBlock`] if trying to revert to a block for which no history
    ///   is preserved.
    /// * [`EvmError::BadRequest`] if given arguments do not match actual values expected by the
    ///   EVM.
    pub async fn revert_to_block(
        &self,
        args: RevertToBlockArgs,
    ) -> CanisterClientResult<Result<()>> {
        self.client.update("revert_to_block", (args,)).await
    }

    /// Returns the current block confirmation strategy
    pub async fn get_block_confirmation_strategy(
        &self,
    ) -> CanisterClientResult<BlockConfirmationStrategy> {
        self.client
            .query("get_block_confirmation_strategy", ())
            .await
    }

    /// Sets the block confirmation strategy.
    /// This function can only be called by the admin.
    pub async fn admin_set_block_confirmation_strategy(
        &self,
        strategy: BlockConfirmationStrategy,
    ) -> CanisterClientResult<Result<()>> {
        self.client
            .update("admin_set_block_confirmation_strategy", (strategy,))
            .await
    }

    /// Attempt to confirm the block with the given hash
    pub async fn send_confirm_block(
        &self,
        data: BlockConfirmationData,
    ) -> CanisterClientResult<Result<BlockConfirmationResult>> {
        self.client.update("send_confirm_block", (data,)).await
    }

    /// Returns information about the blockchain blocks
    pub async fn get_blockchain_block_info(&self) -> CanisterClientResult<BlockchainBlockInfo> {
        self.client.query("get_blockchain_block_info", ()).await
    }
}
