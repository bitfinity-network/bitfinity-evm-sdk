pub const EIP1559_INITIAL_BASE_FEE: u128 = 1000000000;
pub const EIP1559_ELASTICITY_MULTIPLIER: u128 = 2;
pub const EIP1559_BASE_FEE_MAX_CHANGE_DENOMINATOR: u128 = 8;

/// Identifier for Eip2930 Transaction
pub const TRANSACTION_TYPE_EIP2930: u64 = 1;
/// Identifier for Eip1559 Transaction
pub const TRANSACTION_TYPE_EIP1559: u64 = 2;
