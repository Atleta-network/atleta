// This file is part of Substrate.

// Copyright (C) 2017-2021 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use futures::TryFutureExt;
use polkadot_cli::NODE_VERSION;
// Substrate
use sc_cli::SubstrateCli;
use sc_service::DatabaseSource;
// Frontier
pub use crate::error::Error;
// use sc_cli::Error;
use fc_db::kv::frontier_database_dir;
use polkadot_service::OverseerGen;
use std::net::ToSocketAddrs;

use crate::{
    chain_spec,
    cli::{Cli, Subcommand},
    eth::db_config_dir,
    service::{self},
};

#[cfg(feature = "runtime-benchmarks")]
use crate::chain_spec::get_account_id_from_seed;

pub type Result<T> = std::result::Result<T, Error>;

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        "Atleta Network".into()
    }

    fn impl_version() -> String {
        env!("SUBSTRATE_CLI_IMPL_VERSION").into()
    }

    fn description() -> String {
        env!("CARGO_PKG_DESCRIPTION").into()
    }

    fn author() -> String {
        env!("CARGO_PKG_AUTHORS").into()
    }

    fn support_url() -> String {
        "support.anonymous.an".into()
    }

    fn copyright_start_year() -> i32 {
        2024
    }

    fn load_spec(&self, id: &str) -> std::result::Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "dev" => Box::new(chain_spec::development_config()),
            "" | "local" => Box::new(chain_spec::local_testnet_config()),
            "testnet" => Box::new(chain_spec::testnet_config()),
            path => {
                Box::new(chain_spec::ChainSpec::from_json_file(std::path::PathBuf::from(path))?)
            },
        })
    }
}

fn run_node_inner<F>(
    cli: Cli,
    overseer_gen: impl OverseerGen,
    maybe_malus_finality_delay: Option<u32>,
    logger_hook: F,
) -> Result<()>
where
    F: FnOnce(&mut sc_cli::LoggerBuilder, &sc_service::Configuration),
{
    let runner: sc_cli::Runner<Cli> = cli
        .create_runner_with_logger_hook::<sc_cli::RunCmd, F>(&cli.run.base, logger_hook)
        .map_err(Error::from)?;

    // By default, enable BEEFY on all networks, unless explicitly disabled through CLI.
    let enable_beefy = !cli.run.no_beefy;

    let jaeger_agent = if let Some(ref jaeger_agent) = cli.run.jaeger_agent {
        Some(
            jaeger_agent
                .to_socket_addrs()
                .map_err(Error::AddressResolutionFailure)?
                .next()
                .ok_or_else(|| Error::AddressResolutionMissing)?,
        )
    } else {
        None
    };

    let node_version =
        if cli.run.disable_worker_version_check { None } else { Some(NODE_VERSION.to_string()) };

    let secure_validator_mode = cli.run.base.validator && !cli.run.insecure_validator;

    runner.run_node_until_exit(|config| async move {
        let hwbench = (!cli.run.no_hardware_benchmarks)
            .then_some(config.database.path().map(|database_path| {
                let _ = std::fs::create_dir_all(database_path);
                sc_sysinfo::gather_hwbench(Some(database_path))
            }))
            .flatten();

        let database_source: DatabaseSource = config.database.clone();

        let task_manager = service::build_full(
            config,
            cli.eth,
            polkadot_service::NewFullParams {
                is_parachain_node: polkadot_service::IsParachainNode::No,
                enable_beefy,
                force_authoring_backoff: cli.run.force_authoring_backoff,
                jaeger_agent,
                telemetry_worker_handle: None,
                node_version,
                secure_validator_mode,
                workers_path: cli.run.workers_path,
                workers_names: None,
                overseer_gen,
                overseer_message_channel_capacity_override: cli
                    .run
                    .overseer_channel_capacity_override,
                malus_finality_delay: maybe_malus_finality_delay,
                execute_workers_max_num: cli.run.execute_workers_max_num,
                prepare_workers_hard_max_num: cli.run.prepare_workers_hard_max_num,
                prepare_workers_soft_max_num: cli.run.prepare_workers_soft_max_num,
                hwbench,
            },
        )
        .await?;

        if let Some(path) = database_source.path() {
            sc_storage_monitor::StorageMonitorService::try_spawn(
                cli.storage_monitor,
                path.to_path_buf(),
                &task_manager.spawn_essential_handle(),
            )?;
        }

        Ok(task_manager)
    })
}

/// Parse and run command line arguments
pub fn run() -> Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        Some(Subcommand::Key(cmd)) => Ok(cmd.run(&cli)?),
        Some(Subcommand::BuildSpec(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            Ok(runner.sync_run(|config| cmd.run(config.chain_spec, config.network))?)
        },
        Some(Subcommand::CheckBlock(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            runner.async_run(|mut config| {
                let (client, _, import_queue, task_manager, _) =
                    service::new_chain_ops(&mut config, &cli.eth)?;
                Ok((cmd.run(client, import_queue).map_err(Error::SubstrateCli), task_manager))
            })
        },
        Some(Subcommand::ExportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            Ok(runner.async_run(|mut config| {
                let (client, _, _, task_manager, _) =
                    service::new_chain_ops(&mut config, &cli.eth)?;
                Ok((cmd.run(client, config.database).map_err(Error::SubstrateCli), task_manager))
            })?)
        },
        Some(Subcommand::ExportState(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            Ok(runner.async_run(|mut config| {
                let (client, _, _, task_manager, _) =
                    service::new_chain_ops(&mut config, &cli.eth)?;
                Ok((cmd.run(client, config.chain_spec).map_err(Error::SubstrateCli), task_manager))
            })?)
        },
        Some(Subcommand::ImportBlocks(cmd)) => {
            let runner = cli.create_runner(cmd)?;

            Ok(runner.async_run(|mut config| {
                let (client, _, import_queue, task_manager, _) =
                    service::new_chain_ops(&mut config, &cli.eth)?;
                Ok((cmd.run(client, import_queue).map_err(Error::SubstrateCli), task_manager))
            })?)
        },
        Some(Subcommand::PurgeChain(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            Ok(runner.sync_run(|config| {
                // Remove Frontier offchain db
                let db_config_dir = db_config_dir(&config);
                match cli.eth.frontier_backend_type {
                    crate::eth::BackendType::KeyValue => {
                        let frontier_database_config = match config.database {
                            DatabaseSource::RocksDb { .. } => DatabaseSource::RocksDb {
                                path: frontier_database_dir(&db_config_dir, "db"),
                                cache_size: 0,
                            },
                            DatabaseSource::ParityDb { .. } => DatabaseSource::ParityDb {
                                path: frontier_database_dir(&db_config_dir, "paritydb"),
                            },
                            _ => {
                                return Err(format!(
                                    "Cannot purge `{:?}` database",
                                    config.database
                                )
                                .into())
                            },
                        };
                        cmd.run(frontier_database_config)?;
                    },
                    crate::eth::BackendType::Sql => {
                        let db_path = db_config_dir.join("sql");
                        match std::fs::remove_dir_all(&db_path) {
                            Ok(_) => {
                                println!("{:?} removed.", &db_path);
                            },
                            Err(ref err) if err.kind() == std::io::ErrorKind::NotFound => {
                                eprintln!("{:?} did not exist.", &db_path);
                            },
                            Err(err) => {
                                return Err(format!(
                                    "Cannot purge `{:?}` database: {:?}",
                                    db_path, err,
                                )
                                .into())
                            },
                        };
                    },
                };

                cmd.run(config.database)
            })?)
        },
        Some(Subcommand::Revert(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            Ok(runner.async_run(|mut config| {
                let (client, backend, _, task_manager, _) =
                    crate::service::new_chain_ops(&mut config, &cli.eth)?;
                let aux_revert = Box::new(move |client, _, blocks| {
                    sc_consensus_grandpa::revert(client, blocks)?;
                    Ok(())
                });

                Ok((cmd.run(client, backend, Some(aux_revert)), task_manager))
            })?)
        },
        #[cfg(feature = "runtime-benchmarks")]
        Some(Subcommand::Benchmark(cmd)) => {
            use crate::benchmarking::{
                inherent_benchmark_data, RemarkBuilder, TransferKeepAliveBuilder,
            };
            use atleta_runtime::{Block, ExistentialDeposit};
            use frame_benchmarking_cli::{
                BenchmarkCmd, ExtrinsicFactory, SUBSTRATE_REFERENCE_HARDWARE,
            };

            let runner = cli.create_runner(cmd)?;
            match cmd {
                BenchmarkCmd::Pallet(cmd) => runner
                    .sync_run(|config| cmd.run_with_spec::<Block, ()>(Some(config.chain_spec))),
                BenchmarkCmd::Block(cmd) => runner.sync_run(|mut config| {
                    let (client, _, _, _, _) = service::new_chain_ops(&mut config, &cli.eth)?;
                    cmd.run(client)
                }),
                BenchmarkCmd::Storage(cmd) => runner.sync_run(|mut config| {
                    let (client, backend, _, _, _) = service::new_chain_ops(&mut config, &cli.eth)?;
                    let db = backend.expose_db();
                    let storage = backend.expose_storage();
                    cmd.run(config, client, db, storage)
                }),
                BenchmarkCmd::Overhead(cmd) => runner.sync_run(|mut config| {
                    let (client, _, _, _, _) = service::new_chain_ops(&mut config, &cli.eth)?;
                    let ext_builder = RemarkBuilder::new(client.clone());
                    cmd.run(config, client, inherent_benchmark_data()?, Vec::new(), &ext_builder)
                }),
                BenchmarkCmd::Extrinsic(cmd) => runner.sync_run(|mut config| {
                    let (client, _, _, _, _) = service::new_chain_ops(&mut config, &cli.eth)?;
                    // Register the *Remark* and *TKA* builders.
                    let ext_factory = ExtrinsicFactory(vec![
                        Box::new(RemarkBuilder::new(client.clone())),
                        Box::new(TransferKeepAliveBuilder::new(
                            client.clone(),
                            get_account_id_from_seed::<sp_core::ecdsa::Public>("Alice"),
                            ExistentialDeposit::get(),
                        )),
                    ]);

                    cmd.run(client, inherent_benchmark_data()?, Vec::new(), &ext_factory)
                }),
                BenchmarkCmd::Machine(cmd) => {
                    runner.sync_run(|config| cmd.run(&config, SUBSTRATE_REFERENCE_HARDWARE.clone()))
                },
            }
        },
        #[cfg(not(feature = "runtime-benchmarks"))]
        Some(Subcommand::Benchmark) => Err(sc_cli::Error::Input(
            "Benchmarking wasn't enabled when building the node. \
            You can enable it with `--features runtime-benchmarks`."
                .into(),
        )
        .into()),
        Some(Subcommand::FrontierDb(cmd)) => {
            let runner = cli.create_runner(cmd)?;
            Ok(runner.sync_run(|mut config| {
                let (client, _, _, _, frontier_backend) =
                    service::new_chain_ops(&mut config, &cli.eth)?;
                let frontier_backend = match frontier_backend {
                    fc_db::Backend::KeyValue(kv) => kv,
                    _ => panic!("Only fc_db::Backend::KeyValue supported"),
                };
                cmd.run(client, frontier_backend)
            })?)
        },
        Some(Subcommand::RuntimeVersion) => {
            let rv = atleta_runtime::native_version().runtime_version;

            // Constructs determenistic hash for APIs: [(api_id, version)]
            let apis_hash = {
                use std::fmt::Write;

                let mut apis = rv.apis.into_owned();
                apis.sort_by_key(|(api_id, _version)| *api_id);

                let mut api_bytes = vec![];
                for (api_id, version) in apis {
                    api_bytes.extend(api_id);
                    api_bytes.extend(version.to_be_bytes());
                }

                let apis_hash = sp_io::hashing::keccak_256(&api_bytes);
                let mut hash_str = String::new();
                for byte in apis_hash {
                    write!(&mut hash_str, "{:02x}", byte).map_err(|e| {
                        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
                    })?;
                }
                hash_str
            };

            let json = serde_json::json!({
                "spec_name": rv.spec_name.to_string(),
                "spec_version": rv.spec_version,
                "impl_name": rv.impl_name.to_string(),
                "impl_version": rv.impl_version,
                "authoring_version": rv.authoring_version,
                "transaction_version": rv.transaction_version,
                "state_version": rv.state_version,
                "apis_hash": apis_hash,
            });

            let json = serde_json::to_string_pretty(&json).map_err(std::io::Error::from)?;
            println!("{}", json);

            Ok(())
        },
        None => run_node_inner(
            cli,
            polkadot_service::ValidatorOverseerGen,
            None,
            polkadot_node_metrics::logger_hook(),
        ),
    }
}
