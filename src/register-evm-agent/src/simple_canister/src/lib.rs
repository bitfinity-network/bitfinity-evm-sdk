mod canister;
pub mod error;
pub mod memory;
mod state;

pub use crate::canister::TempCanister;

pub fn idl() -> String {
    let idl = TempCanister::idl();
    candid::bindings::candid::compile(&idl.env.env, &Some(idl.actor))
}
