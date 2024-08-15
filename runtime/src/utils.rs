/// Macro to set a value (e.g. when using the `parameter_types` macro) to either a production value
/// or to an environment variable or testing value (in case the `testnet-runtime` or `devnet-runtime` feature is selected).
///
/// Usage:
/// ```Rust
/// parameter_types! {
///     pub LaunchPeriod: BlockNumber = conf!(mainnet: 7 * DAYS, testnet: 1 * DAYS, devnet: HOURS);
///     pub const VotingPeriod: BlockNumber = conf!(mainnet: 7 * DAYS, testnet: 5 * MINUTES, devnet: 1 * MINUTES);
/// }
/// ```
#[macro_export]
macro_rules! conf {
    (mainnet: $prod:expr, testnet: $test:expr, devnet: $dev:expr) => {
        match () {
            _ if cfg!(feature = "testnet-runtime") => $test,
            _ if cfg!(feature = "devnet-runtime") => $dev,
            _ if cfg!(feature = "mainnet-runtime") => $prod,
            _ => panic!("No valid runtime feature selected."),
        }
    };
}
