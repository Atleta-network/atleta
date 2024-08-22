use crate::service::EthConfiguration;
use clap::Parser;
use polkadot_primitives::CollatorPair;

/// Available Sealing methods.
#[derive(Copy, Clone, Debug, Default, clap::ValueEnum)]
pub enum Sealing {
    /// Seal using rpc method.
    #[default]
    Manual,
    /// Seal when transaction is executed.
    Instant,
}

#[derive(Debug, clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[allow(missing_docs)]
    #[command(flatten)]
    pub run: RunCmd,

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

    /// Setup a GRANDPA scheduled voting pause.
    ///
    /// This parameter takes two values, namely a block number and a delay (in
    /// blocks). After the given block number is finalized the GRANDPA voter
    /// will temporarily stop voting for new blocks until the given delay has
    /// elapsed (i.e. until a block at height `pause_block + delay` is imported).
    #[arg(long = "grandpa-pause", num_args = 2)]
    pub grandpa_pause: Vec<u32>,
}

/// Is this node a collator?
#[derive(Clone)]
pub enum IsCollator {
    /// This node is a collator.
    Yes(CollatorPair),
    /// This node is not a collator.
    No,
}
