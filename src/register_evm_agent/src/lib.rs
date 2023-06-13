#[macro_use]
extern crate log;

pub mod agent;
mod constant;
mod error;
mod registration;

pub use error::{Error, Result};
pub use registration::RegistrationService;
