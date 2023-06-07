/// Amount to mint on testnet
pub const AMOUNT_TO_MINT: u128 = 10_u128.pow(18);

/// network name for production
pub const NETWORK_IC: &str = "ic";
/// network name for local replica
pub const NETWORK_LOCAL: &str = "local";

/// method to register an IC agent on the EVMC
pub const METHOD_REGISTER_IC_AGENT: &str = "register_ic_agent";
/// method to verify registration of IC agent on the EMVC
pub const METHOD_VERIFY_REGISTRATION: &str = "verify_registration";
/// method to check whether a certain address is already registered
pub const METHOD_ADDRESS_REGISTERED: &str = "is_address_registered";
/// method to mint EVM native tokens
pub const METHOD_MINT_EVM_TOKENS: &str = "mint_evm_tokens";
/// method to query account basic for wallet address
pub const METHOD_ACCOUNT_BASIC: &str = "account_basic";
/// method to query registration minter address and registration fee
pub const METHOD_REGISTRATION_IC_AGENT_INFO: &str = "registration_ic_agent_info";
