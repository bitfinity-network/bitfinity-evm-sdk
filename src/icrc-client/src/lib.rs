mod client;
mod error;

pub use error::{IcrcError, IcrcResult};

pub use client::{IcrcCanisterClient, StandardRecord};
