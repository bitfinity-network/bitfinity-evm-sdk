/// method to register an IC agent on the EVMC
pub const METHOD_RESERVE_ADDRESS: &str = "reserve_address";
/// method to check whether a certain address is already registered
pub const METHOD_ADDRESS_RESERVED: &str = "is_address_reserved";
/// method to mint EVM native tokens
pub const METHOD_MINT_NATIVE_TOKENS: &str = "mint_native_tokens";
/// method to query account basic for wallet address
pub const METHOD_ACCOUNT_BASIC: &str = "account_basic";

/// Method to query the current gas price
pub const METHOD_MIN_GAS_PRICE: &str = "get_min_gas_price";

/// Default GAS LIMIT
pub const DEFAULT_GAS_LIMIT: u64 = 30_000_000;

/// Default CHAIN ID
pub const CHAIN_ID: u64 = 355114;
