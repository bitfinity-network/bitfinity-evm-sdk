pub const EIP1559_INITIAL_BASE_FEE: u128 = 1000000000;
pub const EIP1559_ELASTICITY_MULTIPLIER: u128 = 2;
pub const EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR: u128 = 8;

/// Identifier for Legacy Transaction
pub const TRANSACTION_TYPE_LEGACY: u64 = 0;
/// Identifier for Eip2930 Transaction
pub const TRANSACTION_TYPE_EIP2930: u64 = 1;
/// Identifier for Eip1559 Transaction
pub const TRANSACTION_TYPE_EIP1559: u64 = 2;

/// The methods will be upgraded when doing http outcalls
pub(crate) const UPGRADE_HTTP_METHODS: &[&str] = &[
    JSON_RPC_METHOD_ETH_SEND_RAW_TRANSACTION_NAME,
    JSON_RPC_METHOD_IC_MINT_NATIVE_TOKEN_NAME,
];

pub const JSON_RPC_METHOD_ETH_SEND_RAW_TRANSACTION_NAME: &str = "eth_sendRawTransaction";

/// This endpoint is used for minting tokens, on the testnet
///
/// NB: This endpoint is only enabled with the testnet feature
pub const JSON_RPC_METHOD_IC_MINT_NATIVE_TOKEN_NAME: &str = "ic_mintNativeToken";
