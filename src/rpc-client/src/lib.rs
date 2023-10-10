use anyhow::Context;
use ethers_core::types::{Block, BlockNumber, Transaction, TransactionReceipt};
use log::trace;
use serde::Deserialize;
use serde_json::{json, Value};

mod methods;
mod request;

pub const MAX_BATCH_REQUESTS: usize = 5; // Max batch size is 5 in EVM

