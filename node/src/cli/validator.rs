use clap::{Parser, Subcommand};
use sc_cli::{CliConfiguration, Error, KeystoreParams, SharedParams, SubstrateCli};
use sc_keystore::LocalKeystore;
use sc_service::config::{BasePath, KeystoreConfig};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_keystore::{KeystoreExt, KeystorePtr};
use sp_session::SessionKeys;

use crate::service::FullClient;

/// Validator related commands.
#[derive(Debug, clap::Parser)]
pub struct ValidateCmd {
    /// Generate session keys and insert them into the keystore.
    #[command(subcommand)]
    subcommand: Option<ValidateSubcommand>,

    #[allow(missing_docs)]
    #[clap(flatten)]
    pub shared_params: SharedParams,
    // /// Decode session keys.
    // DecodeSessionKeys(DecodeSessionKeysCmd),
    //
    //    /// Set session keys.
    //    SetSessionKeys(SetSessionKeysCmd),

    // /// Setup the validator: bond, validate and set session keys.
    // Setup(SetupValidatorCmd),
}

impl ValidateCmd {
    pub fn run<Cli: SubstrateCli>(&self, cli: &Cli, client: &FullClient) -> Result<(), Error> {
        match &self.subcommand {
            Some(sc) => sc.run(cli, client),
            _ => Ok(()),
        }
    }
}

impl CliConfiguration for ValidateCmd {
    fn shared_params(&self) -> &SharedParams {
        &self.shared_params
    }
}

#[derive(Debug, Subcommand)]
pub enum ValidateSubcommand {
    GenerateSessionKeys(GenerateSessionKeysCmd),
}

impl ValidateSubcommand {
    /// Runs the command.
    pub fn run<Cli: SubstrateCli>(&self, cli: &Cli, client: &FullClient) -> Result<(), Error> {
        match self {
            Self::GenerateSessionKeys(cmd) => cmd.run(cli, client),
        }
    }
}

/// `generate-keys` subcommand.
#[derive(Debug, Clone, Parser)]
pub struct GenerateSessionKeysCmd {
    #[allow(missing_docs)]
    #[clap(flatten)]
    pub shared_params: SharedParams,

    #[allow(missing_docs)]
    #[clap(flatten)]
    pub keystore_params: KeystoreParams,
}

impl GenerateSessionKeysCmd {
    /// Run the command
    pub fn run<Cli: SubstrateCli>(&self, cli: &Cli, client: &FullClient) -> Result<(), Error> {
        let base_path = self
            .shared_params
            .base_path()?
            .unwrap_or_else(|| BasePath::from_project("", "", &Cli::executable_name()));
        let chain_id = self.shared_params.chain_id(self.shared_params.is_dev());
        let chain_spec = cli.load_spec(&chain_id)?;
        let config_dir = base_path.config_dir(chain_spec.id());
        let keystore: KeystorePtr = match self.keystore_params.keystore_config(&config_dir)? {
            KeystoreConfig::Path { path, password } => LocalKeystore::open(path, password)?.into(),
            _ => unreachable!("keystore_config always returns path and password; qed"),
        };

        let best_block_hash = client.info().best_hash;
        let mut runtime_api = client.runtime_api();

        runtime_api.register_extension(KeystoreExt::from(keystore.clone()));

        let keys = runtime_api
            .generate_session_keys(best_block_hash, None)
            .map_err(|api_err| Error::Application(Box::new(api_err).into()))?;

        println!("{}", sp_core::bytes::to_hex(&keys, true));

        Ok(())
    }
}

impl CliConfiguration for GenerateSessionKeysCmd {
    fn shared_params(&self) -> &SharedParams {
        &self.shared_params
    }
}
