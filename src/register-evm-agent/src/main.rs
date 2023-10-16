use anyhow::{Ok, Result};
use clap::Parser;
use cli::{generate_wallet, Commands, ReserveMinterCli};

#[macro_use]
extern crate log;

mod agent;
mod cli;
mod constant;
mod error;
mod reservation;
mod transaction;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = ReserveMinterCli::parse();

    match cli.command {
        Commands::GenerateWallet => {
            generate_wallet()?;
            Ok(())
        }
        Commands::Reserve(reserve_args) => reserve_args.exec().await,
        Commands::SignTransaction(sign_transaction_args) => sign_transaction_args.exec().await,
    }
}
