use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::constant::{TRANSACTION_TYPE_EIP1559, TRANSACTION_TYPE_EIP2930, TRANSACTION_TYPE_LEGACY};
use crate::transaction::StorableExecutionResult;
use crate::{Transaction, U256, U64};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, CandidType)]
#[serde(rename_all = "camelCase")]
pub struct FeeHistory {
    /// An array of block base fees per gas.
    pub base_fee_per_gas: Vec<U256>,
    /// An array of block gas used ratios.
    /// These are calculated as the ratio of `gas_used` and `gas_limit`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    pub gas_used_ratio: Vec<f64>,
    /// Lowest number block of the returned range.
    pub oldest_block: U256,
    /// An (optional) array of effective priority fee per gas data points from a single
    /// block. All zeroes are returned if the block is empty.
    #[serde(default)]
    pub reward: Option<Vec<Vec<U256>>>,
}

/// A trait which contains helper methods for the calculation
/// of the fees of the transaction
pub trait FeeCalculation {
    /// Transaction type
    fn transaction_type(&self) -> Option<U64>;
    fn gas_price(&self) -> Option<U256>;
    fn max_fee_per_gas(&self) -> Option<U256>;
    fn max_priority_fee_per_gas(&self) -> Option<U256>;

    /// Returns the effective miner gas tip for the given base fee.
    /// This is used in the calculation of the fee history.
    ///
    /// see:
    /// https://github.com/ethereum/go-ethereum/blob/
    /// 5b9cbe30f8ca2487c8991e50e9c939d5e6ec3cc2/core/types/transaction.go#L347
    fn effective_gas_tip(&self, base_fee: Option<U256>) -> Option<U256> {
        if let Some(base_fee) = base_fee {
            let max_fee_per_gas = self.gas_cost();

            if max_fee_per_gas < base_fee {
                None
            } else {
                let effective_max_fee = max_fee_per_gas - base_fee;

                Some(effective_max_fee.min(self.max_priority_fee_or_gas_price()))
            }
        } else {
            Some(self.max_priority_fee_or_gas_price())
        }
    }

    /// Gas cost of the transaction
    fn gas_cost(&self) -> U256 {
        match self.transaction_type().map(u64::from) {
            Some(TRANSACTION_TYPE_EIP1559) => self.max_fee_per_gas().unwrap_or_default(),
            Some(TRANSACTION_TYPE_EIP2930) | Some(TRANSACTION_TYPE_LEGACY) | None => self.gas_price().unwrap_or_default(),
            _ => panic!("invalid transaction type"),
        }
    }

    /// Returns the priority fee or gas price of the transaction
    fn max_priority_fee_or_gas_price(&self) -> U256 {
        match self.transaction_type().map(u64::from) {
            Some(TRANSACTION_TYPE_EIP1559) => self.max_priority_fee_per_gas().unwrap_or_default(),
            Some(TRANSACTION_TYPE_EIP2930) | Some(TRANSACTION_TYPE_LEGACY) | None => self.gas_price().unwrap_or_default(),
            _ => panic!("invalid transaction type"),
        }
    }
}

impl FeeCalculation for Transaction {
    fn transaction_type(&self) -> Option<U64> {
        self.transaction_type
    }

    fn gas_price(&self) -> Option<U256> {
        self.gas_price.clone()
    }

    fn max_fee_per_gas(&self) -> Option<U256> {
        self.max_fee_per_gas.clone()
    }

    fn max_priority_fee_per_gas(&self) -> Option<U256> {
        self.max_priority_fee_per_gas.clone()
    }
}

impl FeeCalculation for StorableExecutionResult {
    fn transaction_type(&self) -> Option<U64> {
        self.transaction_type
    }

    fn gas_price(&self) -> Option<U256> {
        self.gas_price.clone()
    }

    fn max_fee_per_gas(&self) -> Option<U256> {
        self.max_fee_per_gas.clone()
    }

    fn max_priority_fee_per_gas(&self) -> Option<U256> {
        self.max_priority_fee_per_gas.clone()
    }
}
