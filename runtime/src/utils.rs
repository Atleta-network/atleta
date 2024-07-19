/// Macro to set a value (e.g. when using the `parameter_types` macro) to either a production value
/// or to an environment variable or testing value (in case the `fast-runtime` feature is selected).
///
/// Usage:
/// ```Rust
/// parameter_types! {
///     pub LaunchPeriod: BlockNumber = conf!(mainnet: 7 * DAYS, testnet: 1);
///     pub const VotingPeriod: BlockNumber = conf!(mainnet: 7 * DAYS, testnet: 1 * MINUTES);
/// }
/// ```
#[macro_export]
macro_rules! conf {
    (mainnet: $prod:expr, testnet: $test:expr) => {
        if cfg!(feature = "fast-runtime") {
            $test
        } else {
            $prod
        }
    };
}