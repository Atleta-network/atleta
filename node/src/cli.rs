use crate::eth::EthConfiguration;
use clap::Parser;
use polkadot_service::SubstrateServiceError;

#[cfg(feature = "full-node")]
use polkadot_node_core_av_store::Error as AvailabilityError;

/// Available Sealing methods.
#[derive(Copy, Clone, Debug, Default, clap::ValueEnum)]
pub enum Sealing {
    /// Seal using rpc method.
    #[default]
    Manual,
    /// Seal when transaction is executed.
    Instant,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    AddrFormatInvalid(#[from] std::net::AddrParseError),

    #[error(transparent)]
    Sub(#[from] SubstrateServiceError),

    #[error(transparent)]
    Blockchain(#[from] sp_blockchain::Error),

    #[error(transparent)]
    Consensus(#[from] sp_consensus::Error),

    #[error("Failed to create an overseer")]
    Overseer(#[from] polkadot_overseer::SubsystemError),

    #[error(transparent)]
    Prometheus(#[from] prometheus_endpoint::PrometheusError),

    #[error(transparent)]
    Telemetry(#[from] sc_telemetry::Error),

    #[error(transparent)]
    Jaeger(#[from] polkadot_node_jaeger::JaegerError),

    #[cfg(feature = "full-node")]
    #[error(transparent)]
    Availability(#[from] AvailabilityError),
}

#[derive(Debug, clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[allow(missing_docs)]
    #[command(flatten)]
    pub run: RunCmd,

    #[clap(flatten)]
    pub storage_monitor: sc_storage_monitor::StorageMonitorParams,

    /// Choose sealing method.
    #[arg(long, value_enum, ignore_case = true)]
    pub sealing: Option<Sealing>,

    #[command(flatten)]
    pub eth: EthConfiguration,
}

#[derive(Debug, clap::Subcommand)]
pub enum Subcommand {
    /// Key management cli utilities
    #[command(subcommand)]
    Key(sc_cli::KeySubcommand),

    /// Build a chain specification.
    BuildSpec(sc_cli::BuildSpecCmd),

    /// Validate blocks.
    CheckBlock(sc_cli::CheckBlockCmd),

    /// Export blocks.
    ExportBlocks(sc_cli::ExportBlocksCmd),

    /// Export the state of a given block into a chain spec.
    ExportState(sc_cli::ExportStateCmd),

    /// Import blocks.
    ImportBlocks(sc_cli::ImportBlocksCmd),

    /// Remove the whole chain.
    PurgeChain(sc_cli::PurgeChainCmd),

    /// Revert the chain to a previous state.
    Revert(sc_cli::RevertCmd),

    /// Sub-commands concerned with benchmarking.
    #[cfg(feature = "runtime-benchmarks")]
    #[command(subcommand)]
    Benchmark(frame_benchmarking_cli::BenchmarkCmd),

    /// Sub-commands concerned with benchmarking.
    #[cfg(not(feature = "runtime-benchmarks"))]
    Benchmark,

    /// Db meta columns information.
    FrontierDb(fc_cli::FrontierDbCmd),
    RuntimeVersion,
}

#[derive(Debug, Parser)]
pub struct RunCmd {
    #[clap(flatten)]
    pub base: sc_cli::RunCmd,

    /// Disable the BEEFY gadget.
    #[arg(long)]
    pub no_beefy: bool,

    /// Allows a validator to run insecurely outside of Secure Validator Mode. Security features
    /// are still enabled on a best-effort basis, but missing features are no longer required. For
    /// more information see <https://github.com/w3f/polkadot-wiki/issues/4881>.
    #[arg(long = "insecure-validator-i-know-what-i-do", requires = "validator")]
    pub insecure_validator: bool,

    /// Setup a GRANDPA scheduled voting pause.
    ///
    /// This parameter takes two values, namely a block number and a delay (in
    /// blocks). After the given block number is finalized the GRANDPA voter
    /// will temporarily stop voting for new blocks until the given delay has
    /// elapsed (i.e. until a block at height `pause_block + delay` is imported).
    #[arg(long = "grandpa-pause", num_args = 2)]
    pub grandpa_pause: Vec<u32>,

    /// Enable the block authoring backoff that is triggered when finality is lagging.
    #[arg(long)]
    pub force_authoring_backoff: bool,

    /// Add the destination address to the 'Jaeger' agent.
    ///
    /// Must be valid socket address, of format `IP:Port` (commonly `127.0.0.1:6831`).
    #[arg(long)]
    pub jaeger_agent: Option<String>,

    /// Add the destination address to the `pyroscope` agent.
    ///
    /// Must be valid socket address, of format `IP:Port` (commonly `127.0.0.1:4040`).
    #[arg(long)]
    pub pyroscope_server: Option<String>,

    /// Disable automatic hardware benchmarks.
    ///
    /// By default these benchmarks are automatically ran at startup and measure
    /// the CPU speed, the memory bandwidth and the disk speed.
    ///
    /// The results are then printed out in the logs, and also sent as part of
    /// telemetry, if telemetry is enabled.
    #[arg(long)]
    pub no_hardware_benchmarks: bool,

    /// Overseer message capacity override.
    ///
    /// **Dangerous!** Do not touch unless explicitly advised to.
    #[arg(long)]
    pub overseer_channel_capacity_override: Option<usize>,
    /// Path to the directory where auxiliary worker binaries reside.
    ///
    /// If not specified, the main binary's directory is searched first, then
    /// `/usr/lib/polkadot` is searched.
    ///
    /// TESTING ONLY: if the path points to an executable rather then directory,
    /// that executable is used both as preparation and execution worker.
    #[arg(long, value_name = "PATH")]
    pub workers_path: Option<std::path::PathBuf>,

    /// Override the maximum number of pvf execute workers.
    ///
    ///  **Dangerous!** Do not touch unless explicitly advised to.
    #[arg(long)]
    pub execute_workers_max_num: Option<usize>,
    /// Override the maximum number of pvf workers that can be spawned in the pvf prepare
    /// pool for tasks with the priority below critical.
    ///
    ///  **Dangerous!** Do not touch unless explicitly advised to.

    #[arg(long)]
    pub prepare_workers_soft_max_num: Option<usize>,
    /// Override the absolute number of pvf workers that can be spawned in the pvf prepare pool.
    ///
    ///  **Dangerous!** Do not touch unless explicitly advised to.
    #[arg(long)]
    pub prepare_workers_hard_max_num: Option<usize>,
    /// TESTING ONLY: disable the version check between nodes and workers.
    #[arg(long, hide = true)]
    pub disable_worker_version_check: bool,
}
