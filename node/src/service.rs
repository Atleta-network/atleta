//! Service and ServiceFactory implementation. Specialized wrapper over substrate service.

#![deny(unused_results)]

#[cfg(feature = "full-node")]
use {
    polkadot_node_core_approval_voting::Config as ApprovalVotingConfig,
    polkadot_node_core_av_store::Config as AvailabilityConfig,
    polkadot_node_core_candidate_validation::Config as CandidateValidationConfig,
    polkadot_node_core_chain_selection::{
        self as chain_selection_subsystem, Config as ChainSelectionConfig,
    },
    polkadot_node_core_dispute_coordinator::Config as DisputeCoordinatorConfig,
    polkadot_node_network_protocol::{
        peer_set::{PeerSet, PeerSetProtocolNames},
        request_response::ReqProtocolNames,
    },
    sc_client_api::BlockBackend,
    sc_transaction_pool_api::OffchainTransactionPoolFactory,
};

use futures::prelude::*;
use polkadot_node_subsystem_util::database::Database;

#[cfg(feature = "full-node")]
pub use {
    crate::relay_chain_selection::SelectRelayChain,
    polkadot_overseer::{Handle, OverseerConnector},
};

use frame_benchmarking_cli::SUBSTRATE_REFERENCE_HARDWARE;
use polkadot_node_subsystem_types::DefaultSubsystemClient;
pub use sc_service::{config::DatabaseSource, ChainSpec, Configuration, TaskManager};
use std::{collections::HashMap, path::Path, sync::Arc, time::Duration};

#[cfg(feature = "full-node")]
pub use polkadot_service::{Error, ExtendedOverseerGenArgs, OverseerGen, OverseerGenArgs};

// Substrate
use sc_client_api::Backend as BackendT;
use sc_consensus::{BasicQueue, BoxBlockImport};
use sc_consensus_babe::{BabeLink, BabeWorkerHandle};
use sc_executor::WasmExecutor;
use sc_network_sync::strategy::warp::WarpSyncProvider;
use sc_service::{error::Error as ServiceError, PartialComponents};
use sc_telemetry::{Telemetry, TelemetryHandle, TelemetryWorker};
use sp_core::U256;
use sp_runtime::traits::Block as BlockT;
// Runtime
use atleta_runtime::{opaque::Block, Hash, RuntimeApi, TransactionConverter};

pub use crate::eth::{db_config_dir, EthConfiguration};
use crate::eth::{
    new_frontier_partial, spawn_frontier_tasks, BackendType, FrontierBackend,
    FrontierPartialComponents, StorageOverride, StorageOverrideHandler,
};

#[cfg(feature = "full-node")]
pub fn open_database(db_source: &DatabaseSource) -> Result<Arc<dyn Database>, Error> {
    let parachains_db = match db_source {
        DatabaseSource::RocksDb { path, .. } => crate::parachains_db::open_creating_rocksdb(
            path.clone(),
            crate::parachains_db::CacheSizes::default(),
        )?,
        DatabaseSource::ParityDb { path, .. } => crate::parachains_db::open_creating_paritydb(
            path.parent().ok_or(Error::DatabasePathRequired)?.into(),
            crate::parachains_db::CacheSizes::default(),
        )?,
        DatabaseSource::Auto { paritydb_path, rocksdb_path, .. } => {
            if paritydb_path.is_dir() && paritydb_path.exists() {
                crate::parachains_db::open_creating_paritydb(
                    paritydb_path.parent().ok_or(Error::DatabasePathRequired)?.into(),
                    crate::parachains_db::CacheSizes::default(),
                )?
            } else {
                crate::parachains_db::open_creating_rocksdb(
                    rocksdb_path.clone(),
                    crate::parachains_db::CacheSizes::default(),
                )?
            }
        },
        DatabaseSource::Custom { .. } => {
            unimplemented!("No polkadot subsystem db for custom source.");
        },
    };
    Ok(parachains_db)
}

pub const AVAILABILITY_CONFIG: AvailabilityConfig = AvailabilityConfig {
    col_data: crate::parachains_db::REAL_COLUMNS.col_availability_data,
    col_meta: crate::parachains_db::REAL_COLUMNS.col_availability_meta,
};

/// Only enable the benchmarking host functions when we actually want to benchmark.
#[cfg(feature = "runtime-benchmarks")]
pub type HostFunctions =
    (sp_io::SubstrateHostFunctions, frame_benchmarking::benchmarking::HostFunctions);
/// Otherwise we use empty host functions for ext host functions.
#[cfg(not(feature = "runtime-benchmarks"))]
pub type HostFunctions = sp_io::SubstrateHostFunctions;
/// Full backend.
pub type FullBackend = sc_service::TFullBackend<Block>;
/// Full client.
pub type FullClient = sc_service::TFullClient<Block, RuntimeApi, WasmExecutor<HostFunctions>>;
type BasicImportQueue = sc_consensus::DefaultImportQueue<Block>;
type FullPool = sc_transaction_pool::FullPool<Block, FullClient>;
type FullSelectChain = sc_consensus::LongestChain<FullBackend, Block>;
type GrandpaBlockImport<C> =
    sc_consensus_grandpa::GrandpaBlockImport<FullBackend, Block, C, FullSelectChain>;
type GrandpaLinkHalf<C> = sc_consensus_grandpa::LinkHalf<Block, C, FullSelectChain>;

/// The minimum period of blocks on which justifications will be
/// imported and generated.
const GRANDPA_JUSTIFICATION_PERIOD: u32 = 512;

pub fn new_partial<BIQ>(
    config: &Configuration,
    eth_config: &EthConfiguration,
    build_import_queue: BIQ,
) -> Result<
    PartialComponents<
        FullClient,
        FullBackend,
        FullSelectChain,
        BasicImportQueue,
        FullPool,
        (
            Option<Telemetry>,
            BoxBlockImport<Block>,
            BabeLink<Block>,
            BabeWorkerHandle<Block>,
            GrandpaLinkHalf<FullClient>,
            sc_consensus_beefy::BeefyVoterLinks<Block>,
            FrontierBackend<FullClient>,
            Arc<dyn StorageOverride<Block>>,
        ),
    >,
    ServiceError,
>
where
    BIQ: FnOnce(
        Arc<FullClient>,
        &Configuration,
        &EthConfiguration,
        &TaskManager,
        Option<TelemetryHandle>,
        GrandpaBlockImport<FullClient>,
        FullSelectChain,
        OffchainTransactionPoolFactory<Block>,
    ) -> Result<
        ((BasicImportQueue, BabeWorkerHandle<Block>), BoxBlockImport<Block>, BabeLink<Block>),
        ServiceError,
    >,
{
    let telemetry = config
        .telemetry_endpoints
        .clone()
        .filter(|x| !x.is_empty())
        .map(|endpoints| -> Result<_, sc_telemetry::Error> {
            let worker = TelemetryWorker::new(16)?;
            let telemetry = worker.handle().new_telemetry(endpoints);
            Ok((worker, telemetry))
        })
        .transpose()?;

    let executor = sc_service::new_wasm_executor(config);

    let (client, backend, keystore_container, task_manager) =
        sc_service::new_full_parts::<Block, RuntimeApi, _>(
            config,
            telemetry.as_ref().map(|(_, telemetry)| telemetry.handle()),
            executor,
        )?;
    let client = Arc::new(client);

    let telemetry = telemetry.map(|(worker, telemetry)| {
        task_manager.spawn_handle().spawn("telemetry", None, worker.run());
        telemetry
    });

    let select_chain = sc_consensus::LongestChain::new(backend.clone());

    let transaction_pool = sc_transaction_pool::BasicPool::new_full(
        config.transaction_pool.clone(),
        config.role.is_authority().into(),
        config.prometheus_registry(),
        task_manager.spawn_essential_handle(),
        client.clone(),
    );

    let (grandpa_block_import, grandpa_link) = sc_consensus_grandpa::block_import(
        client.clone(),
        GRANDPA_JUSTIFICATION_PERIOD,
        &client,
        select_chain.clone(),
        telemetry.as_ref().map(|x| x.handle()),
    )?;

    let storage_override = Arc::new(StorageOverrideHandler::new(client.clone()));
    let frontier_backend = match eth_config.frontier_backend_type {
        BackendType::KeyValue => FrontierBackend::KeyValue(Arc::new(fc_db::kv::Backend::open(
            Arc::clone(&client),
            &config.database,
            &db_config_dir(config),
        )?)),
        BackendType::Sql => {
            let db_path = db_config_dir(config).join("sql");
            std::fs::create_dir_all(&db_path).expect("failed creating sql db directory");
            let backend = futures::executor::block_on(fc_db::sql::Backend::new(
                fc_db::sql::BackendConfig::Sqlite(fc_db::sql::SqliteBackendConfig {
                    path: Path::new("sqlite:///")
                        .join(db_path)
                        .join("frontier.db3")
                        .to_str()
                        .unwrap(),
                    create_if_missing: true,
                    thread_count: eth_config.frontier_sql_backend_thread_count,
                    cache_size: eth_config.frontier_sql_backend_cache_size,
                }),
                eth_config.frontier_sql_backend_pool_size,
                std::num::NonZeroU32::new(eth_config.frontier_sql_backend_num_ops_timeout),
                storage_override.clone(),
            ))
            .unwrap_or_else(|err| panic!("failed creating sql backend: {:?}", err));
            FrontierBackend::Sql(Arc::new(backend))
        },
    };

    let (_, beefy_voter_links, _) = sc_consensus_beefy::beefy_block_import_and_links(
        grandpa_block_import.clone(),
        backend.clone(),
        client.clone(),
        config.prometheus_registry().cloned(),
    );

    let ((import_queue, worker_handle), block_import, babe_link) = build_import_queue(
        client.clone(),
        config,
        eth_config,
        &task_manager,
        telemetry.as_ref().map(|x| x.handle()),
        grandpa_block_import,
        select_chain.clone(),
        OffchainTransactionPoolFactory::new(transaction_pool.clone()),
    )?;

    Ok(PartialComponents {
        client,
        backend,
        keystore_container,
        task_manager,
        select_chain,
        import_queue,
        transaction_pool,
        other: (
            telemetry,
            block_import,
            babe_link,
            worker_handle,
            grandpa_link,
            beefy_voter_links,
            frontier_backend,
            storage_override,
        ),
    })
}

/// Build the import queue for the template runtime (aura + grandpa).
pub fn build_babe_grandpa_import_queue(
    client: Arc<FullClient>,
    config: &Configuration,
    eth_config: &EthConfiguration,
    task_manager: &TaskManager,
    telemetry: Option<TelemetryHandle>,
    grandpa_block_import: GrandpaBlockImport<FullClient>,
    select_chain: FullSelectChain,
    offchain_tx_pool_factory: OffchainTransactionPoolFactory<Block>,
) -> Result<
    ((BasicImportQueue, BabeWorkerHandle<Block>), BoxBlockImport<Block>, BabeLink<Block>),
    ServiceError,
> {
    // TODO should we use this instead of babe block import?
    // let _frontier_block_import =
    //     FrontierBlockImport::new(grandpa_block_import.clone(), client.clone());

    let (block_import, babe_link) = sc_consensus_babe::block_import(
        sc_consensus_babe::configuration(&*client)?,
        grandpa_block_import.clone(),
        client.clone(),
    )?;

    let slot_duration = babe_link.config().slot_duration();
    let justification_import = grandpa_block_import;
    let target_gas_price = eth_config.target_gas_price;
    let import_queue = sc_consensus_babe::import_queue(sc_consensus_babe::ImportQueueParams {
        link: babe_link.clone(),
        block_import: block_import.clone(),
        justification_import: Some(Box::new(justification_import)),
        client: client.clone(),
        select_chain,
        create_inherent_data_providers: move |_, ()| async move {
            let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

            let slot =
                sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                    *timestamp,
                    slot_duration,
                );
            let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));

            Ok((slot, timestamp, dynamic_fee))
        },
        spawner: &task_manager.spawn_essential_handle(),
        registry: config.prometheus_registry(),
        telemetry,
        offchain_tx_pool_factory,
    })?;

    Ok((import_queue, Box::new(block_import), babe_link))
}

/// Builds a new service for a full client.
pub async fn new_full<
    OverseerGenerator: OverseerGen,
    Network: sc_network::NetworkBackend<Block, <Block as BlockT>::Hash>,
>(
    mut config: Configuration,
    eth_config: EthConfiguration,
    polkadot_service::NewFullParams {
        is_parachain_node,
        enable_beefy: _,
        force_authoring_backoff: _,
        jaeger_agent: _,
        telemetry_worker_handle: _,
        node_version,
        secure_validator_mode,
        workers_path,
        workers_names,
        overseer_gen,
        overseer_message_channel_capacity_override,
        malus_finality_delay: _malus_finality_delay,
        hwbench,
        execute_workers_max_num: _,
        prepare_workers_soft_max_num,
        prepare_workers_hard_max_num,
    }: polkadot_service::NewFullParams<OverseerGenerator>,
) -> Result<TaskManager, Error> {
    use polkadot_node_network_protocol::request_response::IncomingRequest;
    use sc_network_sync::WarpSyncParams;

    let role = config.role.clone();
    let prometheus_registry = config.prometheus_registry().cloned();

    let overseer_connector = OverseerConnector::default();
    let overseer_handle = Handle::new(overseer_connector.handle());

    let auth_or_collator = role.is_authority() || is_parachain_node.is_collator();
    let build_import_queue = build_babe_grandpa_import_queue;

    let PartialComponents {
        client,
        backend,
        mut task_manager,
        import_queue,
        keystore_container,
        select_chain: _,
        transaction_pool,
        other:
            (
                mut telemetry,
                block_import,
                babe_link,
                worker_handle,
                grandpa_link,
                _beefy_links,
                frontier_backend,
                storage_override,
            ),
    } = new_partial(&config, &eth_config, build_import_queue)?;

    let select_chain = if auth_or_collator {
        let metrics =
            polkadot_node_subsystem_util::metrics::Metrics::register(prometheus_registry.as_ref())?;

        SelectRelayChain::new_with_overseer(
            backend.clone(),
            overseer_handle.clone(),
            metrics,
            Some(task_manager.spawn_handle()),
        )
    } else {
        SelectRelayChain::new_longest_chain(backend.clone())
    };

    let target_gas_price = eth_config.target_gas_price;
    let slot_duration = babe_link.config().slot_duration();

    let FrontierPartialComponents { filter_pool, fee_history_cache, fee_history_cache_limit } =
        new_frontier_partial(&eth_config)?;

    let mut net_config =
        sc_network::config::FullNetworkConfiguration::<_, _, Network>::new(&config.network);
    let metrics = Network::register_notification_metrics(
        config.prometheus_config.as_ref().map(|cfg| &cfg.registry),
    );

    let auth_disc_publish_non_global_ips = config.network.allow_non_globals_in_dht;
    let auth_disc_public_addresses = config.network.public_addresses.clone();

    let genesis_hash = client.block_hash(0).ok().flatten().expect("Genesis block exists; qed");
    let peer_store_handle = net_config.peer_store_handle();

    let grandpa_protocol_name =
        sc_consensus_grandpa::protocol_standard_name(&genesis_hash, &config.chain_spec);

    let (grandpa_protocol_config, grandpa_notification_service) =
        sc_consensus_grandpa::grandpa_peers_set_config::<_, Network>(
            grandpa_protocol_name.clone(),
            metrics.clone(),
            Arc::clone(&peer_store_handle),
        );

    // validation/collation protocols are enabled only if `Overseer` is enabled
    let peerset_protocol_names =
        PeerSetProtocolNames::new(genesis_hash, config.chain_spec.fork_id());

    // If this is a validator or running alongside a parachain node, we need to enable the
    // networking protocols.
    //
    // Collators and parachain full nodes require the collator and validator networking to send
    // collations and to be able to recover PoVs.
    let notification_services =
        if role.is_authority() || is_parachain_node.is_running_alongside_parachain_node() {
            use polkadot_network_bridge::{peer_sets_info, IsAuthority};
            let is_authority = if role.is_authority() { IsAuthority::Yes } else { IsAuthority::No };

            peer_sets_info::<_, Network>(
                is_authority,
                &peerset_protocol_names,
                metrics.clone(),
                Arc::clone(&peer_store_handle),
            )
            .into_iter()
            .map(|(config, (peerset, service))| {
                net_config.add_notification_protocol(config);
                (peerset, service)
            })
            .collect::<HashMap<PeerSet, Box<dyn sc_network::NotificationService>>>()
        } else {
            std::collections::HashMap::new()
        };

    let req_protocol_names = ReqProtocolNames::new(genesis_hash, config.chain_spec.fork_id());

    let (collation_req_v1_receiver, cfg) =
        IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
    net_config.add_request_response_protocol(cfg);
    let (collation_req_v2_receiver, cfg) =
        IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
    net_config.add_request_response_protocol(cfg);
    let (available_data_req_receiver, cfg) =
        IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
    net_config.add_request_response_protocol(cfg);
    let (pov_req_receiver, cfg) =
        IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
    net_config.add_request_response_protocol(cfg);
    let (chunk_req_receiver, cfg) =
        IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
    net_config.add_request_response_protocol(cfg);

    let grandpa_hard_forks = Vec::new();

    let warp_sync_params = {
        net_config.add_notification_protocol(grandpa_protocol_config);
        let warp_sync: Arc<dyn WarpSyncProvider<Block>> =
            Arc::new(sc_consensus_grandpa::warp_proof::NetworkProvider::new(
                backend.clone(),
                grandpa_link.shared_authority_set().clone(),
                grandpa_hard_forks,
            ));
        Some(WarpSyncParams::WithProvider(warp_sync))
    };

    let ext_overseer_args = if is_parachain_node.is_running_alongside_parachain_node() {
        None
    } else {
        let parachains_db = open_database(&config.database)?;
        let candidate_validation_config = if !role.is_authority() {
            let (prep_worker_path, exec_worker_path) = crate::workers::determine_workers_paths(
                workers_path,
                workers_names,
                node_version.clone(),
            )?;

            log::info!("🚀 Using prepare-worker binary at: {:?}", prep_worker_path);
            log::info!("🚀 Using execute-worker binary at: {:?}", exec_worker_path);

            Some(CandidateValidationConfig {
                artifacts_cache_path: config
                    .database
                    .path()
                    .ok_or(Error::DatabasePathRequired)?
                    .join("pvf-artifacts"),
                node_version,
                secure_validator_mode,
                prep_worker_path,
                exec_worker_path,
                pvf_execute_workers_max_num: 4,
                pvf_prepare_workers_soft_max_num: prepare_workers_soft_max_num.unwrap_or(1),
                pvf_prepare_workers_hard_max_num: prepare_workers_hard_max_num.unwrap_or(2),
            })
        } else {
            None
        };
        let (statement_req_receiver, cfg) =
            IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
        net_config.add_request_response_protocol(cfg);
        let (candidate_req_v2_receiver, cfg) =
            IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
        net_config.add_request_response_protocol(cfg);
        let (dispute_req_receiver, cfg) =
            IncomingRequest::get_config_receiver::<_, Network>(&req_protocol_names);
        net_config.add_request_response_protocol(cfg);
        let approval_voting_config = ApprovalVotingConfig {
            col_approval_data: crate::parachains_db::REAL_COLUMNS.col_approval_data,
            slot_duration_millis: slot_duration.as_millis(),
        };
        let dispute_coordinator_config = DisputeCoordinatorConfig {
            col_dispute_data: crate::parachains_db::REAL_COLUMNS.col_dispute_coordinator_data,
        };
        let chain_selection_config = ChainSelectionConfig {
            col_data: crate::parachains_db::REAL_COLUMNS.col_chain_selection_data,
            stagnant_check_interval: Default::default(),
            stagnant_check_mode: chain_selection_subsystem::StagnantCheckMode::PruneOnly,
        };
        Some(ExtendedOverseerGenArgs {
            keystore: keystore_container.local_keystore(),
            parachains_db,
            candidate_validation_config,
            availability_config: AVAILABILITY_CONFIG,
            pov_req_receiver,
            chunk_req_receiver,
            statement_req_receiver,
            candidate_req_v2_receiver,
            approval_voting_config,
            dispute_req_receiver,
            dispute_coordinator_config,
            chain_selection_config,
        })
    };

    let (network, system_rpc_tx, tx_handler_controller, network_starter, sync_service) =
        sc_service::build_network(sc_service::BuildNetworkParams {
            config: &config,
            net_config,
            client: client.clone(),
            transaction_pool: transaction_pool.clone(),
            spawn_handle: task_manager.spawn_handle(),
            import_queue,
            block_announce_validator_builder: None,
            warp_sync_params,
            block_relay: None,
            metrics,
        })?;

    if config.offchain_worker.enabled {
        task_manager.spawn_handle().spawn(
            "offchain-workers-runner",
            "offchain-worker",
            sc_offchain::OffchainWorkers::new(sc_offchain::OffchainWorkerOptions {
                runtime_api_provider: client.clone(),
                is_validator: config.role.is_authority(),
                keystore: Some(keystore_container.keystore()),
                offchain_db: backend.offchain_storage(),
                transaction_pool: Some(OffchainTransactionPoolFactory::new(
                    transaction_pool.clone(),
                )),
                network_provider: Arc::new(network.clone()),
                enable_http_requests: true,
                custom_extensions: |_| vec![],
            })
            .run(client.clone(), task_manager.spawn_handle())
            .boxed(),
        );
    }

    let role = config.role.clone();
    let force_authoring = config.force_authoring;
    let backoff_authoring_blocks =
        Some(sc_consensus_slots::BackoffAuthoringOnFinalizedHeadLagging::default());
    let name = config.network.node_name.clone();
    let frontier_backend = Arc::new(frontier_backend);
    let enable_grandpa = !config.disable_grandpa;
    let prometheus_registry = config.prometheus_registry().cloned();

    // Sinks for pubsub notifications.
    // Everytime a new subscription is created, a new mpsc channel is added to the sink pool.
    // The MappingSyncWorker sends through the channel on block import and the subscription emits a notification to the subscriber on receiving a message through this channel.
    // This way we avoid race conditions when using native substrate block import notification stream.
    let pubsub_notification_sinks: fc_mapping_sync::EthereumBlockNotificationSinks<
        fc_mapping_sync::EthereumBlockNotification<Block>,
    > = Default::default();
    let pubsub_notification_sinks = Arc::new(pubsub_notification_sinks);

    // for ethereum-compatibility rpc.
    config.rpc_id_provider = Some(Box::new(fc_rpc::EthereumSubIdProvider));

    let pending_create_inherent_data_providers = move |_, ()| async move {
        let current = sp_timestamp::InherentDataProvider::from_system_time();
        let next_slot = current.timestamp().as_millis() + slot_duration.as_millis();
        let timestamp = sp_timestamp::InherentDataProvider::new(next_slot.into());
        let slot =
            sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                *timestamp,
                slot_duration,
            );
        let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
        Ok((slot, timestamp, dynamic_fee))
    };

    let eth_rpc_params = crate::rpc::EthDeps {
        client: client.clone(),
        pool: transaction_pool.clone(),
        graph: transaction_pool.pool().clone(),
        converter: Some(TransactionConverter),
        is_authority: config.role.is_authority(),
        enable_dev_signer: eth_config.enable_dev_signer,
        network: network.clone(),
        sync: sync_service.clone(),
        frontier_backend: match &*frontier_backend {
            fc_db::Backend::KeyValue(b) => b.clone(),
            fc_db::Backend::Sql(b) => b.clone(),
        },
        storage_override: storage_override.clone(),
        block_data_cache: Arc::new(fc_rpc::EthBlockDataCacheTask::new(
            task_manager.spawn_handle(),
            storage_override.clone(),
            eth_config.eth_log_block_cache,
            eth_config.eth_statuses_cache,
            prometheus_registry.clone(),
        )),
        filter_pool: filter_pool.clone(),
        max_past_logs: eth_config.max_past_logs,
        fee_history_cache: fee_history_cache.clone(),
        fee_history_cache_limit,
        execute_gas_limit_multiplier: eth_config.execute_gas_limit_multiplier,
        forced_parent_hashes: None,
        pending_create_inherent_data_providers,
    };

    if let Some(hwbench) = hwbench {
        sc_sysinfo::print_hwbench(&hwbench);
        match SUBSTRATE_REFERENCE_HARDWARE.check_hardware(&hwbench) {
            Err(err) if role.is_authority() => {
                log::warn!(
				"⚠️  The hardware does not meet the minimal requirements {} for role 'Authority' find out more at:\n\
				https://wiki.polkadot.network/docs/maintain-guides-how-to-validate-polkadot#reference-hardware",
				err
			);
            },
            _ => {},
        }

        if let Some(ref mut telemetry) = telemetry {
            let telemetry_handle = telemetry.handle();
            task_manager.spawn_handle().spawn(
                "telemetry_hwbench",
                None,
                sc_sysinfo::initialize_hwbench_telemetry(telemetry_handle, hwbench),
            );
        }
    }

    let overseer_client = client.clone();
    let spawner = task_manager.spawn_handle();

    let authority_discovery_service =
		// We need the authority discovery if this node is either a validator or running alongside a parachain node.
		// Parachains node require the authority discovery for finding relay chain validators for sending
		// their PoVs or recovering PoVs.
		if role.is_authority() || is_parachain_node.is_running_alongside_parachain_node() {
			use futures::StreamExt;
			use sc_network::{Event, NetworkEventStream};

			let authority_discovery_role = if role.is_authority() {
				sc_authority_discovery::Role::PublishAndDiscover(keystore_container.keystore())
			} else {
				// don't publish our addresses when we're not an authority (collator, cumulus, ..)
				sc_authority_discovery::Role::Discover
			};
			let dht_event_stream =
				network.event_stream("authority-discovery").filter_map(|e| async move {
					match e {
						Event::Dht(e) => Some(e),
						_ => None,
					}
				});

			let (worker, service) = sc_authority_discovery::new_worker_and_service_with_config(
				sc_authority_discovery::WorkerConfig {
					publish_non_global_ips: auth_disc_publish_non_global_ips,
					public_addresses: auth_disc_public_addresses,
					// Require that authority discovery records are signed.
					strict_record_validation: true,
					..Default::default()
				},
				client.clone(),
				Arc::new(network.clone()),
				Box::pin(dht_event_stream),
				authority_discovery_role,
				prometheus_registry.clone(),
			);

			task_manager.spawn_handle().spawn(
				"authority-discovery-worker",
				Some("authority-discovery"),
				Box::pin(worker.run()),
			);
			Some(service)
		} else {
			None
		};

    let runtime_client = Arc::new(DefaultSubsystemClient::new(
        overseer_client.clone(),
        OffchainTransactionPoolFactory::new(transaction_pool.clone()),
    ));

    let overseer_handle = if let Some(authority_discovery_service) = authority_discovery_service {
        let (overseer, overseer_handle) = overseer_gen
            .generate::<sc_service::SpawnTaskHandle, DefaultSubsystemClient<FullClient>>(
                overseer_connector,
                OverseerGenArgs {
                    runtime_client,
                    network_service: network.clone(),
                    sync_service: sync_service.clone(),
                    authority_discovery_service,
                    collation_req_v1_receiver,
                    collation_req_v2_receiver,
                    available_data_req_receiver,
                    registry: prometheus_registry.as_ref(),
                    spawner,
                    is_parachain_node,
                    overseer_message_channel_capacity_override,
                    req_protocol_names,
                    peerset_protocol_names,
                    notification_services,
                },
                ext_overseer_args,
            )
            .map_err(|e| {
                gum::error!("Failed to init overseer: {}", e);
                e
            })?;
        let handle = Handle::new(overseer_handle.clone());

        {
            let handle = handle.clone();

            task_manager.spawn_essential_handle().spawn_blocking(
                "overseer",
                None,
                Box::pin(async move {
                    use futures::{pin_mut, select, FutureExt};

                    let forward = polkadot_overseer::forward_events(overseer_client, handle);

                    let forward = forward.fuse();
                    let overseer_fut = overseer.run().fuse();

                    pin_mut!(overseer_fut);
                    pin_mut!(forward);

                    select! {
                        () = forward => (),
                        () = overseer_fut => (),
                        complete => (),
                    }
                }),
            );
        }
        Some(handle)
    } else {
        assert!(
            !auth_or_collator,
            "Precondition congruence (false) is guaranteed by manual checking. qed"
        );

        None
    };

    let rpc_builder = {
        // all these double clones are actually needed here, because we need Fn, but not FnOnce
        let client = client.clone();
        let pool = transaction_pool.clone();
        let select_chain = select_chain.clone();
        let keystore = keystore_container.keystore().clone();
        let pubsub_notification_sinks = pubsub_notification_sinks.clone();
        let justification_stream = grandpa_link.justification_stream();
        let shared_authority_set = grandpa_link.shared_authority_set().clone();
        let finality_provider = sc_consensus_grandpa::FinalityProofProvider::new_for_service(
            backend.clone(),
            Some(shared_authority_set.clone()),
        );

        Box::new(move |deny_unsafe, subscription_executor: sc_rpc::SubscriptionTaskExecutor| {
            let shared_voter_state = sc_consensus_grandpa::SharedVoterState::empty();
            let deps = crate::rpc::FullDeps {
                client: client.clone(),
                pool: pool.clone(),
                select_chain: select_chain.clone(),
                deny_unsafe,
                command_sink: None,
                eth: eth_rpc_params.clone(),
                babe: crate::rpc::BabeDeps {
                    keystore: keystore.clone(),
                    worker_handle: worker_handle.clone(),
                },
                grandpa: crate::rpc::GrandpaDeps {
                    shared_voter_state,
                    shared_authority_set: shared_authority_set.clone(),
                    justification_stream: justification_stream.clone(),
                    subscription_executor: subscription_executor.clone(),
                    finality_provider: finality_provider.clone(),
                },
            };

            crate::rpc::create_full(deps, subscription_executor, pubsub_notification_sinks.clone())
                .map_err(Into::into)
        })
    };

    let _rpc_handlers = sc_service::spawn_tasks(sc_service::SpawnTasksParams {
        config,
        client: client.clone(),
        backend: backend.clone(),
        task_manager: &mut task_manager,
        keystore: keystore_container.keystore(),
        transaction_pool: transaction_pool.clone(),
        rpc_builder,
        network: network.clone(),
        system_rpc_tx,
        tx_handler_controller,
        sync_service: sync_service.clone(),
        telemetry: telemetry.as_mut(),
    })?;

    spawn_frontier_tasks(
        &task_manager,
        client.clone(),
        backend.clone(),
        frontier_backend,
        filter_pool,
        storage_override,
        fee_history_cache,
        fee_history_cache_limit,
        sync_service.clone(),
        pubsub_notification_sinks,
    )
    .await;

    if role.is_authority() {
        let proposer_factory = sc_basic_authorship::ProposerFactory::new(
            task_manager.spawn_handle(),
            client.clone(),
            transaction_pool.clone(),
            prometheus_registry.as_ref(),
            telemetry.as_ref().map(|x| x.handle()),
        );

        let overseer_handle =
            overseer_handle.as_ref().ok_or(Error::AuthoritiesRequireRealOverseer)?.clone();
        let slot_duration = babe_link.config().slot_duration();
        let babe_config = sc_consensus_babe::BabeParams {
            keystore: keystore_container.keystore(),
            client: client.clone(),
            select_chain,
            env: proposer_factory,
            block_import,
            sync_oracle: sync_service.clone(),
            justification_sync_link: sync_service.clone(),
            create_inherent_data_providers: move |parent, ()| {
                let client_clone = client.clone();
                let overseer_handle = overseer_handle.clone();
                async move {
                    let parachain =
                        polkadot_node_core_parachains_inherent::ParachainsInherentDataProvider::new(
                            client_clone.clone(),
                            overseer_handle,
                            parent,
                        );

                    let timestamp = sp_timestamp::InherentDataProvider::from_system_time();

                    let slot =
                        sp_consensus_babe::inherents::InherentDataProvider::from_timestamp_and_slot_duration(
                            *timestamp,
                            slot_duration,
                        );

                    // TODO huh?
                    // let dynamic_fee = fp_dynamic_fee::InherentDataProvider(U256::from(target_gas_price));
                    // Ok((slot, timestamp, dynamic_fee))
                    Ok((slot, timestamp, parachain))
                }
            },
            force_authoring,
            backoff_authoring_blocks,
            babe_link,
            block_proposal_slot_portion: sc_consensus_babe::SlotProportion::new(2f32 / 3f32),
            max_block_proposal_slot_portion: None,
            telemetry: telemetry.as_ref().map(|x| x.handle()),
        };

        let babe = sc_consensus_babe::start_babe(babe_config)?;
        task_manager.spawn_essential_handle().spawn_blocking(
            "babe-proposer",
            Some("block-authoring"),
            babe,
        );
    }

    if enable_grandpa {
        // if the node isn't actively participating in consensus then it doesn't
        // need a keystore, regardless of which protocol we use below.
        let keystore = if role.is_authority() { Some(keystore_container.keystore()) } else { None };

        let grandpa_config = sc_consensus_grandpa::Config {
            // FIXME #1578 make this available through chainspec
            gossip_duration: Duration::from_millis(1000),
            justification_generation_period: GRANDPA_JUSTIFICATION_PERIOD,
            name: Some(name),
            observer_enabled: false,
            keystore,
            local_role: role,
            telemetry: telemetry.as_ref().map(|x| x.handle()),
            protocol_name: grandpa_protocol_name,
        };

        // start the full GRANDPA voter
        // NOTE: non-authorities could run the GRANDPA observer protocol, but at
        // this point the full voter should provide better guarantees of block
        // and vote data availability than the observer. The observer has not
        // been tested extensively yet and having most nodes in a network run it
        // could lead to finality stalls.
        let grandpa_voter =
            sc_consensus_grandpa::run_grandpa_voter(sc_consensus_grandpa::GrandpaParams {
                config: grandpa_config,
                link: grandpa_link,
                network,
                sync: sync_service,
                notification_service: grandpa_notification_service,
                voting_rule: sc_consensus_grandpa::VotingRulesBuilder::default().build(),
                prometheus_registry,
                shared_voter_state: sc_consensus_grandpa::SharedVoterState::empty(),
                telemetry: telemetry.as_ref().map(|x| x.handle()),
                offchain_tx_pool_factory: OffchainTransactionPoolFactory::new(transaction_pool),
            })?;

        // the GRANDPA voter task is considered infallible, i.e.
        // if it fails we take down the service with it.
        task_manager
            .spawn_essential_handle()
            .spawn_blocking("grandpa-voter", None, grandpa_voter);
    }

    network_starter.start_network();
    Ok(task_manager)
}

// /// Build a full node.
// ///
// /// The actual "flavor", aka if it will use `Polkadot`, `Rococo` or `Kusama` is determined based on
// /// [`IdentifyVariant`] using the chain spec.
// #[cfg(feature = "full-node")]
pub async fn build_full<OverseerGenerator: OverseerGen>(
    config: Configuration,
    eth_config: EthConfiguration,
    params: polkadot_service::NewFullParams<OverseerGenerator>,
) -> Result<TaskManager, Error> {
    use crate::service;

    match config.network.network_backend {
        sc_network::config::NetworkBackendType::Libp2p => {
            service::new_full::<_, sc_network::NetworkWorker<Block, Hash>>(
                config, eth_config, params,
            )
            .await
        },
        sc_network::config::NetworkBackendType::Litep2p => {
            service::new_full::<_, sc_network::Litep2pNetworkBackend>(config, eth_config, params)
                .await
        },
    }
}

pub fn new_chain_ops(
    config: &mut Configuration,
    eth_config: &EthConfiguration,
) -> Result<
    (
        Arc<FullClient>,
        Arc<FullBackend>,
        BasicQueue<Block>,
        TaskManager,
        FrontierBackend<FullClient>,
    ),
    ServiceError,
> {
    config.keystore = sc_service::config::KeystoreConfig::InMemory;
    let PartialComponents { client, backend, import_queue, task_manager, other, .. } =
        new_partial::<_>(config, eth_config, build_babe_grandpa_import_queue)?;
    Ok((client, backend, import_queue, task_manager, other.6))
}
