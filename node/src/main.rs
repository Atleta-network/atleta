//! Substrate Node Template CLI library.

#![warn(missing_docs)]
#![allow(clippy::type_complexity, clippy::too_many_arguments, clippy::large_enum_variant)]
#![cfg_attr(feature = "runtime-benchmarks", deny(unused_crate_dependencies))]

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
mod chain_spec;
mod cli;
mod command;
mod error;
mod eth;
mod grandpa_support;
mod parachains_db;
mod relay_chain_selection;
mod rpc;
mod service;
mod workers;

fn main() -> command::Result<()> {
    command::run()
}
