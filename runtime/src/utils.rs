/// Macro to set a value depending on the runtime environment.
/// Supports `mainnet`, `testnet`, and `devnet` configurations.
///
/// This can be used, for example, with the `parameter_types` macro to provide
/// different values for different environments.
///
/// Usage:
/// ```rust
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
