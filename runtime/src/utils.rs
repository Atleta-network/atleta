/// Macro to set a value (e.g. when using the `parameter_types` macro) to either a production value
/// or to an environment variable or testing value (in case the `fast-runtime` feature is selected).
/// Note that the environment variable is evaluated _at compile time_.
///
/// Usage:
/// ```Rust
/// parameter_types! {
///     // Note that the env variable version parameter cannot be const.
///     pub LaunchPeriod: BlockNumber = conf!(7 * DAYS, 1, "KSM_LAUNCH_PERIOD");
///     pub const VotingPeriod: BlockNumber = conf!(7 * DAYS, 1 * MINUTES);
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
