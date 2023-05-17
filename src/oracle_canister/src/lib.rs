mod canister;
pub mod error;
mod state;
mod timer;

pub use crate::canister::OracleCanister;

pub fn idl() -> String {
    let idl = OracleCanister::idl();
    candid::bindings::candid::compile(&idl.env.env, &Some(idl.actor))
}
