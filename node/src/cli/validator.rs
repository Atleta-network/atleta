use clap::{Parser, Subcommand};
use sc_cli::{CliConfiguration, Error, KeystoreParams, SharedParams, SubstrateCli};
use sc_keystore::LocalKeystore;
use sc_service::config::{BasePath, KeystoreConfig};
use sp_api::{ApiExt, ProvideRuntimeApi};
use sp_blockchain::HeaderBackend;
use sp_keystore::{KeystoreExt, KeystorePtr};
use sp_session::SessionKeys;

use atleta_runtime::opaque;

use crate::service::FullClient;

/// Validator related commands.
#[derive(Debug, clap::Parser)]
pub struct ValidateCmd {
    #[allow(missing_docs)]
    #[command(subcommand)]
    subcommand: Option<ValidateSubcommands>,

    #[allow(missing_docs)]
    #[clap(flatten)]
    pub shared_params: SharedParams,
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
pub enum ValidateSubcommands {
    /// Generate session keys and insert them into the keystore.
    GenerateSessionKeys(GenerateSessionKeysCmd),

    /// Decode session keys.
    DecodeSessionKeys(DecodeSessionKeysCmd),

    // /// Insert session keys into the keystore.
    // InsertSessionKeys(InsertSessionKeysCmd),

    //    /// Set session keys.
    //    SetSessionKeys(SetSessionKeysCmd),

    // /// Setup the validator: bond, validate and set session keys.
    // Setup(SetupValidatorCmd),
}

impl ValidateSubcommands {
    /// Runs the command.
    pub fn run<Cli: SubstrateCli>(&self, cli: &Cli, client: &FullClient) -> Result<(), Error> {
        match self {
            Self::GenerateSessionKeys(cmd) => cmd.run(cli, client),
            Self::DecodeSessionKeys(cmd) => cmd.run(),
        }
    }
}

/// `generate-session-keys` subcommand.
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

/// `decode-session-keys` subcommand.
#[derive(Debug, Clone, Parser)]
pub struct DecodeSessionKeysCmd {
    #[allow(missing_docs)]
    #[clap(flatten)]
    pub shared_params: SharedParams,

    #[allow(missing_docs)]
    #[clap(flatten)]
    pub keystore_params: KeystoreParams,

    /// Hex-encoded session keys.
    #[arg(value_name = "SESSION KEYS")]
    pub keys: String,
}

impl DecodeSessionKeysCmd {
    /// Run the command
    pub fn run(&self) -> Result<(), Error> {
        match decode_readable(&self.keys)? {
            Some(decoded) => {
                for key_line in decoded {
                    println!("{}: {}", key_line.0, key_line.1);
                }
            },
            None => eprintln!("Error decoding session keys"),
        }

        Ok(())
    }
}

fn decode_readable(keys: &str) -> Result<Option<Vec<(String, String)>>, Error> {
    let bytes: Vec<u8> = sp_core::bytes::from_hex(keys)
        .map_err(|convert_err| Error::Application(Box::new(convert_err).into()))?;

    Ok(opaque::SessionKeys::decode_into_raw_public_keys(&bytes).map(|decoded| {
        decoded
            .into_iter()
            .map(|(value, key_id)| {
                (
                    String::from_utf8(key_id.0.to_vec()).expect("KeyTypeId string is valid"),
                    sp_core::bytes::to_hex(&value, true),
                )
            })
            .collect()
    }))
}

impl CliConfiguration for DecodeSessionKeysCmd {
    fn shared_params(&self) -> &SharedParams {
        &self.shared_params
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decoding_session_keys_works() {
        let keys = "0xeafcb752d8b82fc872f3b4dcb6c55104b80de3b49796a1e61b9ef310eb5da42dd58e4f129f85b11a2b65d2a8b6cfc3f95c1d43505a1ec25bdcae7ff28471c20140b743a501c25bb8fa033dde00de6c73ebf8c62421d4d1bd67780e8098e8aa23";

        assert_eq!(
            decode_readable(keys).unwrap(),
            Some(vec![
                (
                    "babe".to_string(),
                    "0xeafcb752d8b82fc872f3b4dcb6c55104b80de3b49796a1e61b9ef310eb5da42d"
                        .to_string()
                ),
                (
                    "gran".to_string(),
                    "0xd58e4f129f85b11a2b65d2a8b6cfc3f95c1d43505a1ec25bdcae7ff28471c201"
                        .to_string()
                ),
                (
                    "imon".to_string(),
                    "0x40b743a501c25bb8fa033dde00de6c73ebf8c62421d4d1bd67780e8098e8aa23"
                        .to_string()
                ),
            ])
        )
    }
}
