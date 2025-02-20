use std::collections::BTreeMap;

// 3rd party imports
use hex_literal::hex;

// Substrate
use sc_chain_spec::{ChainSpecExtension, ChainType, Properties};
use serde::{Deserialize, Serialize};
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_beefy::ecdsa_crypto::AuthorityId as BeefyId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
#[allow(unused_imports)]
use sp_core::ecdsa;
use sp_core::{Pair, Public};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

// Frontier
#[cfg(any(feature = "testnet-runtime", feature = "devnet-runtime"))]
use atleta_runtime::FaucetConfig;
use atleta_runtime::{
    constants::currency::*, opaque::SessionKeys, AccountId, BabeConfig, Balance, BalancesConfig,
    Block, ConfigurationConfig, EVMChainIdConfig, MaxNominations, NominationPoolsConfig,
    RuntimeGenesisConfig, SS58Prefix, SessionConfig, Signature, StakerStatus, StakingConfig,
    SudoConfig, TechnicalCommitteeConfig, BABE_GENESIS_EPOCH_CONFIG, WASM_BINARY,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use polkadot_primitives::{AssignmentId, AuthorityDiscoveryId, ValidatorId};

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<Block>,
    /// The light sync state.
    ///
    /// This value will be set by the `sync-state rpc` implementation.
    pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;

// Public account type
#[allow(dead_code)]
type AccountPublic = <Signature as Verify>::Signer;

// Development config
pub fn development_config() -> ChainSpec {
    use devnet_keys::*;

    ChainSpec::builder(WASM_BINARY.expect("WASM not available"), Default::default())
        .with_name("Development")
        .with_id("dev")
        .with_chain_type(ChainType::Development)
        .with_properties(properties())
        .with_genesis_config(
            serde_json::to_value(testnet_genesis(
                // Sudo account (Alith)
                alith(),
                // Pre-funded accounts
                vec![alith(), baltathar(), charleth(), dorothy(), ethan(), faith(), goliath()],
                // Initial Validators and PoA authorities
                vec![authority_keys_from_seed("Alice")],
                // Initial nominators
                vec![],
                // Ethereum chain ID
                SS58Prefix::get() as u64,
            ))
            .expect("Invalid genesis config"),
        )
        .build()
}

// Dev chain config
pub fn devnet_config() -> ChainSpec {
    use devnet_keys::*;

    ChainSpec::builder(WASM_BINARY.expect("WASM not available"), Default::default())
        .with_name("Devnet")
        .with_id("devnet")
        .with_chain_type(ChainType::Development)
        .with_properties(properties())
        .with_genesis_config(
            serde_json::to_value(testnet_genesis(
                // Sudo account (Alith)
                alith(),
                // Pre-funded accounts
                vec![alith(), baltathar(), charleth(), dorothy(), ethan(), faith(), goliath()],
                // Initial Validators and PoA authorities
                vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
                // Initial nominators
                vec![],
                // Ethereum chain ID
                SS58Prefix::get() as u64,
            ))
            .expect("Invalid genesis config"),
        )
        .build()
}

// Local testnet config
pub fn local_testnet_config() -> ChainSpec {
    use devnet_keys::*;

    ChainSpec::builder(WASM_BINARY.expect("WASM not available"), Default::default())
        .with_name("Local Testnet")
        .with_id("local")
        .with_chain_type(ChainType::Local)
        .with_properties(properties())
        .with_genesis_config(
            serde_json::to_value(testnet_genesis(
                // Sudo account (Alith)
                alith(),
                // Pre-funded accounts
                vec![alith(), baltathar(), charleth(), dorothy(), ethan(), faith(), goliath()],
                vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
                vec![],
                // Ethereum chain ID
                SS58Prefix::get() as u64,
            ))
            .expect("Invalid genesis config"),
        )
        .build()
}

// Testnet config
pub fn testnet_config() -> ChainSpec {
    use testnet_keys::*;

    ChainSpec::builder(WASM_BINARY.expect("WASM not available"), Default::default())
        .with_name("Olympia")
        .with_id("testnet")
        .with_chain_type(ChainType::Custom("Testnet".to_string()))
        .with_properties(properties())
        .with_genesis_config(
            serde_json::to_value(testnet_genesis(
                // Sudo account (Alith)
                lionel(),
                // Pre-funded accounts
                vec![
                    lionel(),
                    diego(),
                    pele(),
                    franz(),
                    johan(),
                    ronaldo(),
                    zinedine(),
                    cristiano(),
                    michel(),
                    roberto(),
                ],
                vec![diego_session_keys(), pele_session_keys(), franz_session_keys()],
                vec![],
                // Ethereum chain ID
                SS58Prefix::get() as u64,
            ))
            .expect("Invalid genesis config"),
        )
        .build()
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
    sudo_key: AccountId,
    mut endowed_accounts: Vec<AccountId>,
    initial_authorities: Vec<ValidatorKeys>,
    initial_nominators: Vec<AccountId>,
    chain_id: u64,
) -> RuntimeGenesisConfig {
    initial_authorities
        .iter()
        .map(|x| [&x.id, &x.stash])
        .chain(initial_nominators.iter().map(|x| [x, x]))
        .for_each(|x| {
            for i in x {
                if !endowed_accounts.contains(i) {
                    endowed_accounts.push(*i)
                }
            }
        });

    let num_endowed_accounts = endowed_accounts.len();

    const ENDOWMENT: Balance = 1_000_000 * UNITS;
    const STASH: Balance = ENDOWMENT / 1000;
    let mut rng = rand::thread_rng();
    let stakers = initial_authorities
        .iter()
        .map(|x| (x.id, x.stash, STASH, StakerStatus::Validator))
        .chain(initial_nominators.iter().map(|x| {
            use rand::{seq::SliceRandom, Rng};
            let limit = (MaxNominations::get() as usize).min(initial_authorities.len());
            let count = rng.gen::<usize>() % limit;
            let nominations = initial_authorities
                .as_slice()
                .choose_multiple(&mut rng, count)
                .map(|choice| choice.id)
                .collect::<Vec<_>>();
            (*x, *x, STASH, StakerStatus::Nominator(nominations))
        }))
        .collect::<Vec<_>>();

    RuntimeGenesisConfig {
        babe: BabeConfig { epoch_config: BABE_GENESIS_EPOCH_CONFIG, ..Default::default() },
        balances: BalancesConfig {
            balances: endowed_accounts.iter().cloned().map(|k| (k, ENDOWMENT)).collect::<Vec<_>>(),
        },
        configuration: ConfigurationConfig { config: default_parachains_host_configuration() },
        session: SessionConfig {
            keys: initial_authorities
                .iter()
                .cloned()
                .map(|keys| {
                    let id = keys.id;
                    let stash = keys.stash;
                    let session_keys: SessionKeys = keys.into();
                    (stash, id, session_keys)
                })
                .collect::<Vec<_>>(),
        },
        sudo: SudoConfig { key: Some(sudo_key) },
        staking: StakingConfig {
            validator_count: initial_authorities.len() as u32,
            minimum_validator_count: initial_authorities.len() as u32,
            invulnerables: initial_authorities.iter().map(|x| x.id).collect::<Vec<_>>(),
            slash_reward_fraction: Perbill::from_percent(10),
            stakers: stakers.clone(),
            min_nominator_bond: 10 * UNITS,
            min_validator_bond: 1_000 * UNITS,
            ..Default::default()
        },
        technical_committee: TechnicalCommitteeConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect::<Vec<_>>(),
            ..Default::default()
        },
        evm_chain_id: EVMChainIdConfig { chain_id, ..Default::default() },
        nomination_pools: NominationPoolsConfig {
            min_create_bond: 10 * UNITS,
            min_join_bond: UNITS,
            ..Default::default()
        },
        #[cfg(any(feature = "testnet-runtime", feature = "devnet-runtime"))]
        faucet: FaucetConfig { initial_balance: 1_000_000 * UNITS },
        ..Default::default()
    }
}

pub fn stagenet_config() -> ChainSpec {
    ChainSpec::builder(WASM_BINARY.expect("WASM not found"), Default::default())
        .with_name("Stagenet")
        .with_id("stagenet")
        .with_chain_type(ChainType::Live)
        .with_properties(properties())
        .with_genesis_config(
            serde_json::to_value(mainnet_genesis(
                stagenet_keys::sudo_account(),
                stagenet_keys::validators(),
                stagenet_keys::prefunded(),
                SS58Prefix::get() as u64,
            ))
            .expect("Invalid genesis config"),
        )
        .build()
}

pub fn mainnet_config() -> ChainSpec {
    ChainSpec::builder(WASM_BINARY.expect("WASM not found"), Default::default())
        .with_name("Atleta")
        .with_id("mainnet")
        .with_chain_type(ChainType::Live)
        .with_properties(properties())
        .with_genesis_config(
            serde_json::to_value(mainnet_genesis(
                mainnet_genesis::sudo_account(),
                mainnet_genesis::validators(),
                mainnet_genesis::technical_allocation(),
                SS58Prefix::get() as u64,
            ))
            .expect("Invalid genesis config"),
        )
        .build()
}

fn mainnet_genesis(
    sudo_key: AccountId,
    validators_keys: Vec<ValidatorKeys>,
    initial_balances: impl IntoIterator<Item = (AccountId, Balance)>,
    chain_id: u64,
) -> RuntimeGenesisConfig {
    const VALIDATOR_INITIAL_BALANCE: Balance = 75_000 * UNITS;
    const STASH_INITIAL_BALANCE: Balance = 25_000 * UNITS;

    let mut initial_balances = BTreeMap::<AccountId, Balance>::from_iter(initial_balances);

    for keys in &validators_keys {
        initial_balances.insert(keys.id, VALIDATOR_INITIAL_BALANCE);
    }

    let stakers = validators_keys
        .iter()
        .map(|keys| {
            (keys.id, keys.stash, STASH_INITIAL_BALANCE, StakerStatus::<AccountId>::Validator)
        })
        .collect::<Vec<_>>();

    RuntimeGenesisConfig {
        babe: BabeConfig { epoch_config: BABE_GENESIS_EPOCH_CONFIG, ..Default::default() },
        balances: BalancesConfig { balances: initial_balances.into_iter().collect::<Vec<_>>() },
        configuration: ConfigurationConfig { config: default_parachains_host_configuration() },
        sudo: SudoConfig { key: Some(sudo_key) },
        staking: StakingConfig {
            validator_count: validators_keys.len() as u32,
            minimum_validator_count: validators_keys.len() as u32,
            invulnerables: validators_keys.iter().map(|x| x.id).collect::<Vec<_>>(),
            slash_reward_fraction: Perbill::from_percent(5),
            stakers,
            min_nominator_bond: 1_000 * UNITS,
            min_validator_bond: 75_000 * UNITS,
            ..Default::default()
        },
        session: SessionConfig {
            keys: validators_keys
                .iter()
                .cloned()
                .map(|keys| {
                    let id = keys.id;
                    let stash = keys.stash;
                    let session_keys: SessionKeys = keys.into();
                    (stash, id, session_keys)
                })
                .collect::<Vec<_>>(),
        },
        technical_committee: mainnet_genesis::technical_committee_config(),
        nomination_pools: NominationPoolsConfig {
            min_join_bond: 100 * UNITS,
            min_create_bond: 1_000 * UNITS,
            ..Default::default()
        },
        council: mainnet_genesis::council_config(),
        evm_chain_id: EVMChainIdConfig { chain_id, ..Default::default() },
        ..Default::default()
    }
}

mod devnet_keys {
    use super::*;

    pub(super) fn alith() -> AccountId {
        AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac"))
    }

    pub(super) fn baltathar() -> AccountId {
        AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0"))
    }

    pub(super) fn charleth() -> AccountId {
        AccountId::from(hex!("798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc"))
    }

    pub(super) fn dorothy() -> AccountId {
        AccountId::from(hex!("773539d4Ac0e786233D90A233654ccEE26a613D9"))
    }

    pub(super) fn ethan() -> AccountId {
        AccountId::from(hex!("Ff64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB"))
    }

    pub(super) fn faith() -> AccountId {
        AccountId::from(hex!("C0F0f4ab324C46e55D02D0033343B4Be8A55532d"))
    }

    pub(super) fn goliath() -> AccountId {
        AccountId::from(hex!("7BF369283338E12C90514468aa3868A551AB2929"))
    }
}

mod testnet_keys {
    use super::*;

    pub(super) fn lionel() -> AccountId {
        AccountId::from(hex!("08e390762f64ABA6F9F9269589e1A702623e90F1"))
    }

    pub(super) fn diego() -> AccountId {
        AccountId::from(hex!("d04a0d2CfBA9d3ae7054dF317e5e1E6bBbBA2472"))
    }

    pub(super) fn pele() -> AccountId {
        AccountId::from(hex!("8834dc7eB54957Bf37CAC825E93D9632dC42c3f2"))
    }

    pub(super) fn franz() -> AccountId {
        AccountId::from(hex!("5124ed655cc596DBD17afddE990E46857B5421F2"))
    }

    pub(super) fn johan() -> AccountId {
        AccountId::from(hex!("3bc92E5C6637aC3a2F98c103967cDBB44586D1D4"))
    }

    pub(super) fn ronaldo() -> AccountId {
        AccountId::from(hex!("30ceFB3383dBDAd376d2036CabeaA7d6BedD883F"))
    }

    pub(super) fn zinedine() -> AccountId {
        AccountId::from(hex!("004D1B6AbBf790d69a498531760E1219a67D009c"))
    }

    pub(super) fn cristiano() -> AccountId {
        AccountId::from(hex!("681547651C2e060444E718cc55b9bB6b1f780a3F"))
    }

    pub(super) fn michel() -> AccountId {
        AccountId::from(hex!("97F10eE955879f3EddeE3368365d0fCC5c816652"))
    }

    pub(super) fn roberto() -> AccountId {
        AccountId::from(hex!("8ec8036d2746f635A32164f9e6C8c3f654d8Ab42"))
    }

    pub(super) fn diego_session_keys() -> ValidatorKeys {
        ValidatorKeys {
            id: AccountId::from(hex!("FFa4645462F429E8FB9a6534E22f9f4f75094aB4")),
            stash: AccountId::from(hex!("d04a0d2CfBA9d3ae7054dF317e5e1E6bBbBA2472")),
            babe: sp_core::sr25519::Public::from_raw(hex!(
                "562cd8c70c00ec3a3a031f5c9885978dd03a3a4fdb27bcf126887a9da11ff405"
            ))
            .into(),
            grandpa: sp_core::ed25519::Public::from_raw(hex!(
                "ffe39c882d4ec6800a7501e1ccf3193b1f4d789d599d37f03db7f92bffb26471"
            ))
            .into(),
            im_online: sp_core::sr25519::Public::from_raw(hex!(
                "76bb986cb29126a2d7848317cd1dcbdbdd743bf69c0daf673674dbed19b70e4d"
            ))
            .into(),
            para_validator: sp_core::sr25519::Public::from_raw(hex!(
                "928b6cc65c0af10060c041ab2cf2a7acd5ee5cfe983d33df47b4569513601119"
            ))
            .into(),
            para_assignment: sp_core::sr25519::Public::from_raw(hex!(
                "54f81f0289afd8d99f9ec60efeb6541bbc953d60d1743f0e41d0ec5f4e1c8b54"
            ))
            .into(),
            authority_discovery: sp_core::sr25519::Public::from_raw(hex!(
                "4c6feb3c3f7c547a4630e181853d885498c55d72df686d7ab8e5d9854fbdeb7e"
            ))
            .into(),
            beefy: sp_core::ecdsa::Public::from_raw(hex!(
                "02e5967d94fbffa084b5c818adf500e0ef85d1aaf125e8bf90d3ca3d85b4ccd9f9"
            ))
            .into(),
        }
    }

    pub(super) fn pele_session_keys() -> ValidatorKeys {
        ValidatorKeys {
            id: AccountId::from(hex!("8834dc7eB54957Bf37CAC825E93D9632dC42c3f2")),
            stash: AccountId::from(hex!("55DE108cb01Acf946A0ddE3C40D5EdE3AE9201C1")),
            babe: sp_core::sr25519::Public::from_raw(hex!(
                "84bb180709195c3f12bc22e16fb971a0369ebd45b6b8334f6f03d50aa986c213"
            ))
            .into(),
            grandpa: sp_core::ed25519::Public::from_raw(hex!(
                "16ec13de87e30ee2eb9be5874558a9a82a39d2707d3ab67670c5e94bb64646ac"
            ))
            .into(),
            im_online: sp_core::sr25519::Public::from_raw(hex!(
                "30a332f8874e0f7a66770917b27aba5fc5ca25f81c31332baaf5f1e897e4b404"
            ))
            .into(),
            para_validator: sp_core::sr25519::Public::from_raw(hex!(
                "801c1833f11bacf7e886ba0f638ea5f94a55f4d0e25ed7e055a6b05392982173"
            ))
            .into(),
            para_assignment: sp_core::sr25519::Public::from_raw(hex!(
                "b2e8ded1345c31db85751ba36053f6e0ae03a53474118ed2ffc771c702e2b536"
            ))
            .into(),
            authority_discovery: sp_core::sr25519::Public::from_raw(hex!(
                "387681fa78ded7783f7bf0cd05d249b9b624125392c2ec0766602dde1f02e454"
            ))
            .into(),
            beefy: sp_core::ecdsa::Public::from_raw(hex!(
                "03f077b84a3d6fc7032a797cc8f068e43c0358c0181234cc7309159e57056a11e3"
            ))
            .into(),
        }
    }

    pub(super) fn franz_session_keys() -> ValidatorKeys {
        ValidatorKeys {
            id: AccountId::from(hex!("5124ed655cc596DBD17afddE990E46857B5421F2")),
            stash: AccountId::from(hex!("F87EfACD0e08cF7F6667B2a8BEc9fC3a2DB1572F")),
            babe: sp_core::sr25519::Public::from_raw(hex!(
                "22c09973a99e38bcf899411fac369257bd8971eddc167e718b1a9014279a2415"
            ))
            .into(),
            grandpa: sp_core::ed25519::Public::from_raw(hex!(
                "e73dc222fb879f67add8aeedf30156a47fd8740d02432e3db5d4ebe8c78f1b87"
            ))
            .into(),
            im_online: sp_core::sr25519::Public::from_raw(hex!(
                "44bea1479765faa200b5ee7b37ac00795891ff97fd629c2676c481bbb6e27f61"
            ))
            .into(),
            para_validator: sp_core::sr25519::Public::from_raw(hex!(
                "b8dd60e50c7ca1b47feb4faa58f1f2741fff68c527c297fc70ba72b91436cd71"
            ))
            .into(),
            para_assignment: sp_core::sr25519::Public::from_raw(hex!(
                "98fbdacb195db4f1a31238687e0e2e2c3311a3c6f00fef708a645630ce97f716"
            ))
            .into(),
            authority_discovery: sp_core::sr25519::Public::from_raw(hex!(
                "60a678a7410322ad55e77064d595ed81e1c4a024fd81adbc3d56e4e4f841781c"
            ))
            .into(),
            beefy: sp_core::ecdsa::Public::from_raw(hex!(
                "0200e692bbc231d1521133b3aa20f09b2dfbeca8682fd2ad23fa437fe51e49daab"
            ))
            .into(),
        }
    }
}

#[rustfmt::skip]
mod stagenet_keys {
    use super::*;

    pub fn sudo_account() -> AccountId {
        AccountId::from(hex!("937FecD86668311a03d3CbC6fEeeD5262d77D112"))
    }

    pub fn prefunded() -> Vec<(AccountId, Balance)> {
        const INITIAL_TREASURY_ALLOCATION: Balance = 290_770_000 * UNITS;
        const INITIAL_LIQUIDITY_ALLOCATION: Balance = 933_000_000 * UNITS;
        const INITIAL_STAKING_ALLOCATION: Balance = 670_800_000 * UNITS;
        const INITIAL_LIQUIDITY_RESERVES_ALLOCATION: Balance = 1_105_200_000 * UNITS;
        const INITIAL_SUDO_ACCOUNT_ALLOCATION: Balance = 5_000 * UNITS;

        vec![
            (sudo_account(), INITIAL_SUDO_ACCOUNT_ALLOCATION),
            (technical_addresses::treasury(), INITIAL_TREASURY_ALLOCATION),
            (technical_addresses::liquidity_reserves(), INITIAL_LIQUIDITY_RESERVES_ALLOCATION),
            (technical_addresses::liquidity(), INITIAL_LIQUIDITY_ALLOCATION),
            (technical_addresses::staking_rewards(), INITIAL_STAKING_ALLOCATION),
        ]
    }

    pub fn validators() -> Vec<ValidatorKeys> {
        vec![validator_1(), validator_2(), validator_3()]
    }

    pub fn validator_1() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("04d1191600fa259ddD2b5bED9F81015b4b68c014")),
            stash: AccountId::from(hex!("05564c8dF908539a2e496C7ddb2F41B8b2beFCaa")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("24e7fb09be5c771d5817efc33b0fe1196b4c252c71d558cf17a91d6a61f09d49")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("24914446bad39a3f95253adca2eab197ec34394526d022a363786d064f0e908b")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("9e9ae326252709d58daa5b9f19f6212fdd052b0c30d96f7cdabc116cf31dc648")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("523604ab6c9ef09e44b92cdf06397b8bbe9698daca1c364e41adc5c81fa63221")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("94d563b742e04c56e503c98d72c876f60cffdbd8ed980154197c79260da6b00c")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("aee3875f768b33a0a868ae7c6e6b09a7cd89435111162fd5d902a62ac9ac9b3f")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("024e7f30da63df3b319aecfe4356e1c7c32f1f70c15b6420825572aeab2d555696")).into(),
        }
    }

    pub fn validator_2() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("3619acB32a35eb4cb2604FB5F5fFfcee1A690734")),
            stash: AccountId::from(hex!("A5Fbd0509079e75c70815A050dC4c0e5F27E3c9b")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("3885bf307307e04ec3554c8840dc7dae749172072897037bf176c897e97b6837")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("6f83d51bad7cd39ef9601c385750cf4e722059f5dfb492614c0fd353a0eb5862")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("20f894e171e4dc99b2eb85a3b1f9a443de6ea450ffbadd2cd2f302604246503f")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("52734cc800decf712ab42aa51cd2ee6764f59ba295d584c915690b6d80594440")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("3485f1a3601a0c3767b9180fd7d84389235f54587254be542f69b021dd5b6151")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("fabb5cdaec93e96dd0b7b7673d8e79a0af9d60abc1d2e4ec9d143dc8a15dcc5b")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("037ad4ca3ebf0c55e9a47ca49cc43a110d2fe1f24ac5d72db288ab5788560093ab")).into(),
        }
    }

    pub fn validator_3() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("a485363Fffc08BB2B17286c3c32B33294A28AF6A")),
            stash: AccountId::from(hex!("C14E277a3c032AA11324B3F44f36B2f8CB04A61E")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("5e5b26fb481a6bb8e8e3f76bc42cac8f8e37a2d7b33cbadafc962aee4638b16e")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("b628d661eef1efe640daff977e484e7c865d8ecd6f54dd0c5bf5345b2925b49f")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("38e248ccf2011d84f5725eaf231c67bae7fc7c7c3fedc94ea7a5dbf05e741806")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("8a624320841b8260bd83597344cc49405142113624447dd0fc3c8ba07724b37d")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("d69573b81008d6ad64869f1209150774f4919fb308365cb33071d30a712aa277")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("90f9bc30714c32a880c610aaa632758b00d6e6e24761495bc41287a0bd9f3b5f")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("02675b062ca4fbe50ce0294f766c7166c2eda7a7cc6af0ddf18e76b029287d18a3")).into(),
        }
    }

    /*
    pub fn validator_*() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("")),
            stash: AccountId::from(hex!("")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("")).into(),
        }
    }
    */
}

#[rustfmt::skip]
mod mainnet_genesis {
    use atleta_runtime::CouncilConfig;
    use super::*;

    pub fn sudo_account() -> AccountId {
        AccountId::from(hex!("9aC7b66BA5153013Eda183Bc865716B52C935296"))
    }

    pub fn technical_allocation() -> Vec<(AccountId, Balance)> {
        const INITIAL_TREASURY_ALLOCATION: Balance = 289_870_000 * UNITS;
        const INITIAL_LIQUIDITY_ALLOCATION: Balance = 933_000_000 * UNITS;
        const INITIAL_STAKING_ALLOCATION: Balance = 670_800_000 * UNITS;
        const INITIAL_LIQUIDITY_RESERVES_ALLOCATION: Balance = 1_105_200_000 * UNITS;
        const INITIAL_SUDO_ACCOUNT_ALLOCATION: Balance = 5_000 * UNITS;

        vec![
            (sudo_account(), INITIAL_SUDO_ACCOUNT_ALLOCATION),
            (technical_addresses::treasury(), INITIAL_TREASURY_ALLOCATION),
            (technical_addresses::liquidity_reserves(), INITIAL_LIQUIDITY_RESERVES_ALLOCATION),
            (technical_addresses::liquidity(), INITIAL_LIQUIDITY_ALLOCATION),
            (technical_addresses::staking_rewards(), INITIAL_STAKING_ALLOCATION),
        ]
    }

    pub fn council_config() -> CouncilConfig {
         CouncilConfig {
            members: vec![
                AccountId::from(hex!("85fc1309AcD66a3a6109487980D3e186B5718D51")),
                AccountId::from(hex!("881fe63dfEE7611CC005e2b0e4577B8c3BF0D478")),
                AccountId::from(hex!("7c7e63c46e4E1cC71a759f591197A26A98a6146b")),
                AccountId::from(hex!("ccfe5bb109F0abCdAF88Ecf3e6C8ac22Fa66b389")),
            ],
            ..Default::default()
        }
    }

    pub fn technical_committee_config() -> TechnicalCommitteeConfig {
        TechnicalCommitteeConfig {
            members: vec![
                AccountId::from(hex!("79A65ccb3daA8389Ea4669406f2439acBAeAC5d4")),
                AccountId::from(hex!("9dc4DaaBEB464Ca48e1f0Be5497DFc395a8FE1Fb")),
            ],
            ..Default::default()
        }
    }

    pub fn validators() -> Vec<ValidatorKeys> {
        vec![validator_1(), validator_2(), validator_3(), validator_4(), validator_5(), validator_6(), validator_7(), validator_8(), validator_9(), validator_10(), validator_11(), validator_12(), validator_13(), validator_14(), validator_15(),]
    }

    pub fn validator_1() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("Fa0d70f2b88912E0099de496Bfd8F296d97d08e9")),
            stash: AccountId::from(hex!("69Eb52bC75CE33bFd4e122ccDE023322ab01A1DC")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("ceedd14ca9ea378e1d37a51d063f46bf6da59a094f5b09da5c053a5eb1361837")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("ad0d2e41709caba94d620e13c60baaae7b4f1157fcc01b6539c3f4e51e5359da")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("5051cbe4f31040504ea1260855992bcc1291c6f6a173f9a4912474dd43331d3d")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("3a3e809c43feeac019488979c647f80b074d8bdfaf9b28efff082c818cda4436")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("9e7fc69110b1c016c1e0ff724e9d0c414057f20e83bf336cfcbc030e5760ab67")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("bc9b5df3dd1aaad6ebcef83e445ab51a2a157f31a66f87c712e852ff3e639b21")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("02fa9eeda7ac82992ff6cc31cc8be9f82215d1404544f7e225b094e38079b5a1eb")).into(),
        }
    }

    pub fn validator_2() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("1609B6765B3fCEd963eFD100f2F78fF75D37bcCC")),
            stash: AccountId::from(hex!("f2C803A8E946b51bF75C3213c5c4b782e3b4ebD5")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("02956798aec549d8f335a5bc4e46aea35850d46a39cc549046a8b1806a9b8951")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("1f95cd7c8e854ad6d6feff5c4f9f2efafd914e27c55debcb6d5ed5ca2f1c2ab5")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("b203871b718f84d3107463cdc3fa38ef1ce2a1005d18e92b4c820d3004840668")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("9e2875713cb18e6296329cb759699d6b481fd88928b0a121ad33209a595a093d")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("aa8f07cfc632893f370549a7d4b1abb004002aec1d177ccf714dee81faa6597c")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("98e55ef9cd1158b7c81b995bab903156c040a12c97b7d0415649fb7a4e029a14")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("02cb7925d2cefcb071bce68fafd02cde1ac0d2be394be7a720a129ad5a8b20d1be")).into(),
        }
    }

    pub fn validator_3() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("551D0F08F6840aB1C3df35246F77F3688749a701")),
            stash: AccountId::from(hex!("5dE950e1a26e719504396FeACb20102Cb0bBD9A1")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("2c80590d5c1d84a89d90ca8f9dbdab26b216fe3d5d36525b64816766d50b1c17")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("1590eb01221bc9967c05066e453f7a6f91e518e33d09217734451eb16961c89c")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("f2cc22e6eaad8930b53038220f4b80d275889e70ed3b8e9aa50dd720d016cb64")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("4a24a56db95efc5f3a7dcc2c5f5e628bb6fc8785dc1eb54d44b2644397c6d639")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("b823d6f99b7edd36903d2f09708d1fc95a770039d9041ef329cbc2d5916ba547")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("fa01317728cd5e09cfabca456ba23a5ad2bea6f293b5bfea9ddc45dfa873fa10")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("034fdedad971547836757223c6d9c50f29e2332c7506cd287d2d62584dbe02185b")).into(),
        }
    }

    pub fn validator_4() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("0ac7a66767155cA74Bb774af117D8e08eaB73b4F")),
            stash: AccountId::from(hex!("afcDa4839270e7F7B691B1e8F387b516100f6dc1")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("f6d576c88e3ba12967cc3856d829f81e7eac75000bf5e51cdcc5a35f28118b78")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("48fa9e9cbf7d34331a944422e5308577372d0c178b3f38e43ac2ebd189d15c4b")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("fa245890c56778b94aa9d33a0b41dd299f3c688b84e863c61ca31d1441e50a44")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("08611495a33837dbe5077cc3582bcd9d43724072c29ed30c1f94ac4291c37a1d")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("b2fa2b49afb7a6bf8fcee9f6e7226d4eac4fd66ab1093a55b2adb3d410482c4f")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("c6b848fa3b94e1714cd86f86b473b693b595785499a7e275a9d45b2eb066b54e")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("02d544b96db8d107f5790f8fc241776fec026a6a1071d9d35a16f3130ecdc22856")).into(),
        }
    }

    pub fn validator_5() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("72aab2682C62B22BaE7fB5cA5F61eEd4f41Da72c")),
            stash: AccountId::from(hex!("603D72B1b97748584FafbD839FECF9070906b22c")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("10b3762b95bf3eb57d7fad228432ecec272f097f0bf80da4ccdc2a4c72f73212")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("1548048ac53b55084597650870697a8feb0e09e2bd7603fcdb7483f845bfebc8")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("720f3dddc9ad53c0a09c64f8d6b3a0bbf745d3796c38b0dae410c12c65b7e523")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("cc42e750b410c94fa49b880b9fedc4dd8db558051896328ccb74be8161b9777d")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("6a8507b19999c8b46c3bbb6b30c7eea0fa958a64be29d6946f733399e6b6166a")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("0c850496c776998c55d745bc7e7c582c0ab2e8d931a02fe68447e3ba0f830476")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("0361289495c0643accc8b1bad631647dd05727d14dc940ccadcee9246eaf01501c")).into(),
        }
    }

    pub fn validator_6() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("B25e361cb8Ca3418c0f51d6b7C4a0810053fa090")),
            stash: AccountId::from(hex!("69Cd2Db68348edacA3f0A3fb1de5fd16c2397109")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("f6153ae9834af2cf6594745e157c451f7b3608571cf8f2049b9802efdbd1ce22")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("a40db50c9fe98f095f9dc4ec213ab595b07df16d09a14b8d8232c641f3021584")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("1607bc929ad5ae15eca870fc68031c8392cf230ad9f8b6db0d03ef27161afd27")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("988ca1f4deb6dbd4f9914643ff9c1d08c1c0c0d491194b9de9cbc8176b590954")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("00321e44b7388e8fa63e4730a383b5847af51ddd15972dd9caf9ccbb31203950")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("d2c825b9e039a043453aeb11d793841a51e32df50fa865c9aa12bc6c880fdb16")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("02f5155d0e9dd5c88c83c4cbe24065daf5db50f7c922d11184e22da342f9588ab6")).into(),
        }
    }

    pub fn validator_7() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("27B8BfbC77770Ab8c1a917d381a56C58272F3060")),
            stash: AccountId::from(hex!("40A77Fd2Bb8f4278360C9C1f200a3237F5eE3687")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("56e261e95aac28ee224bc6ef8b41585c1914d4754dc437556dcceb951d32603d")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("e41b93cdf1597f1115b623a8396998c6ed2064b83a0ee9c48d77a7f65683845f")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("049be74dfcfa8ab26559d47cac354cf5aaddbe610686c1fadd1c44f056939935")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("6c433b8500cc8694a757e2b05ee7c88c472007e3b58f3e382dde516c1aeb0e6e")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("5824faf65ba998fac539e60be2cef723f50e67a0fe1e22016785e91f146db10c")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("ee13a48978de4196440741e86d636b4365f110feb846a5293e3b738d9bd6ba65")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("0322857a4fb3efb64983bfcd795b4fa50aa611d2d7f5367262a2daf08d4744c29b")).into(),
        }
    }

    pub fn validator_8() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("a68c27f8470DbFcFAd56Fda15cf2CEFb37a6Beac")),
            stash: AccountId::from(hex!("aCDd9aE38771D394DB5598181f5A7e044Db0F000")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("a6077133cb4f9be15be450480bf82566372b43f08392663f6af2b2613e5e9b77")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("b778415defac58f277684da8ffd96fa9beb7c9861f504dfffb23621437cc9545")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("96277da628006faefd69e25dac9e15a95a4d80481f72fb2a09d440aaf009c836")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("968871680339e53a0426f11de7443cdefdf2936c284f308cc5c1f0205532b35f")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("7c723ebc3f0722205303c2056f511096e4b6b238875d4f5a8c435e5b3ee04676")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("44431fcf3c3e6857d27f3481593c488b286c49761447643d5eb0ea94f7686452")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("02f659a48c40772452bd0bb9d7d788a6af46e979fe2af0da0eefe3a94ebcff32fe")).into(),
        }
    }

    pub fn validator_9() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("c7cbeeC537710329F01d1402D778471A4955982A")),
            stash: AccountId::from(hex!("8BEB4d41C1232E9aAD6de6d9f6e1fB320da118a8")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("76f4940e06d2c84f28d48cc04b9f64af764319c13696f3c034ad9ed4e685bd18")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("f13a14c59956a607f48ae46f6331032c4ffe4adc958bfbbc8e24738041ddd77d")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("94d80d6e8d3d49af50558e802d256c65ab9e9d42f7082b6ff8bd19fad6f45767")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("ee6c7e62c644379c20836792ecdbdbdce354c8fbcfd167e4fc6ed2d160f9f22f")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("1cebf6baec0656eb740ae20807eeec0a134c901bc8c122b5023a76d737490e33")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("88c523da614298da297a6e953e40f20d7e956164d63c6f56139c8bd280333c76")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("03e8a2292b130f9f0e25faf4a4f69dc029a595687509e6f5ee72afa80d83a2b9ba")).into(),
        }
    }

    pub fn validator_10() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("8EfA6A74e3F76297fBda11F101C2Bc9f5294D4C3")),
            stash: AccountId::from(hex!("aD8308252eE05A917f10BBc5e5d4212E42d3F47f")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("00709fc5e80fbc279896eecdb7c0ea6ef574c6ce343cdf4f1ab96f6832d88016")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("80da96977364176a66fbbf8ee096af1332c902f636ce77f007fed2699b941b15")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("aa042be3f296d825274dfb9924daea2000dffe8b89bf145fafcde092838dd31a")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("d21e3bd6a6f8313c7077607c1ddfcf2deaa15dbee57ca5707bc6568b39b80437")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("d8d3c137bb3c25caa1f9d7245d20986684a1d348befd41ae8becb67f024c1454")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("4e9db56c531881f003e441fa547290703c7e1cb3081223750a5f4e72f4dc2612")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("03dc9312b651940c136af3546836eaf033b94eac5e114a333850fca4f595d0dd71")).into(),
        }
    }

    pub fn validator_11() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("29F61f6332B1a29B1df543c03d8B70AdEb70Ac20")),
            stash: AccountId::from(hex!("9851b23408E8eD8a65Ff9b13b6970D84f054a661")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("0ae8d2587db8a72762c0a4dae27316153eb5392c63108c3fb853f63f2330e157")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("4deeda2273f91d5a282ef254791171916402c9025c740dbe628e5dcf022fdcb0")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("be76f9fb1942dbff7234c758c0bd63dc0d63c2cad9bcae0d9b3aad0394430735")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("2e9ff738c10f0632038851266feab012aacbfbcfc14e99fd8bb72987c45e5640")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("4e0580ea27ea1b0800f6c30fe1d89996e2c193475e4ab556f45d623b3555d302")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("e0a0ab04fa6de423a2675f1af9b8d09ed23d903f755fe74a6590811aa15aaa06")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("020ba3267d84e48b6ca466b4d784fc746d0ebf08ffbeaa4dc912064e5be79d504b")).into(),
        }
    }

    pub fn validator_12() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("51d603126aE43399c188A0A0e32Df1a5AE823D46")),
            stash: AccountId::from(hex!("3e407784DFFa72D586b521C908E192A0ec18d124")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("2a7e45fae8765529b97d2175cfa2da68bcf2f6670a25c85fd012967e20f02150")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("22fe577af649dd078d10ec8b3e97906e464141ccb17d41696b4aa0d4f880bdc8")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("aa0b3ad397997ee126b1679f6fae307cab49c1773d6003d14ceab85191a5cd5e")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("da954b666824584615e9ba8456f1bea2354ff37fa7616f44ceffb5965a743127")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("7603bd8e34760a7694b7b902daccfdcd060698cc472197867de295b7d15b8d13")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("6e528daec7e4fabd9573dfd481befc1f4e2659385232c9d1edd41d1c9ccb1a33")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("03f383edaad94e89e995c51494c7622fb66677922b61de1d7b6fe22c57942b5e1a")).into(),
        }
    }

    pub fn validator_13() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("3bDB3d9CD6BBf4415C706203a3029659DDaeB652")),
            stash: AccountId::from(hex!("Af7C00AC23c6f498B99E8b71195Fbeb76E58c3E3")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("967c6c62b862d847020d30d4f167612b776cf7285c0aba9a39e9f29fc1c2be0b")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("dc0b0522424a786bd2485ab4c9d03d7ceeff95230382fef434f8e024dafea7ee")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("d0034704551e81479cb985db0817703c8fedf4e007f7e1dce1e9928cbfefea31")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("2cd6b85578584d447d39c313ca03973a94f112ed7316af671ade3d6890c47e4d")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("823fbca040d2a4fda74287915d4e71a179f6e75a8d19cee784a9e926e06fa46a")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("f01e102877b5706ac939fe4d901f00fd47f4d2da58bb782cb540d6d0f5ead546")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("02548f44f6412833f8fbe541071645fb43638ae83380c785d7e3809022bfae5e20")).into(),
        }
    }

    pub fn validator_14() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("5A9D837491F9AA4e5f53C8D22cd948De6910543d")),
            stash: AccountId::from(hex!("657319ad303180cD965A75A0349E780D8e6Dd362")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("9e63684865d0b3748c2ce0b537391ee154fcac77da7479e48f9dc0510128a601")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("cf6dffcde0b054c0004977bf083746a5618d54854605dc17615e808bec6e204e")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("1e18f5ea4eafc141b4a4a10e8fb9f0b00cd096f50de728acaa46288710543273")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("ea74864473779afc111644567535c4f4bb7c7866f83f9a502e874b6bf96b8d07")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("9e77afa88dbb2345c257559f111a7a3afb25ee367db14b13b913b37d3de0274e")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("f4fde9898166575dd9a0bc1d5c1895c75e0187120fcdece1ebd70c7c803ecd27")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("02098c3bc6e8030925b673276d84fee0af23773241903ba05e3a556cba4bacb6e3")).into(),
        }
    }

    pub fn validator_15() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("20e7b908A6F98fb0F2C085c9cb74689cA02fA753")),
            stash: AccountId::from(hex!("94b3Cb1D5D63b6a925e82C971E9D5228b354Ee41")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("5a614b338708f968b4847bff00cd75cbbaa4c250cfd6edc20e2eb2408405631d")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("e1538a3f793ad691e37617181d61785d0e4667b864b5fe1d139b7b114fdd5fbf")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("38ca655f5f36240e6b7dbaf0916f3b1e2e40c83c3412260fcc1d60682a6f397d")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("d4f06f3daa0d72ce8e426b97b2e356b26d67e957e29f12856593532c237a7b6c")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("be7510540cb237aa668974d6b32e6e00345e5ccbbfff249f84b6dd2e0afcbf16")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("9c126ef065ca900301fcc57356fb3bb7de85f9c4ad5c93b25525ffe002b4447e")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("0250b059dcc20b6520913640be0edd4dd80cdff9da1da2c293c6867fcd5dff0634")).into(),
        }
    }

    /*
    pub fn validator_*() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("")),
            stash: AccountId::from(hex!("")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("")).into(),
        }
    }
    */
}

mod technical_addresses {
    use atleta_runtime::AccountId;
    use sp_runtime::traits::AccountIdConversion;

    pub fn treasury() -> AccountId {
        atleta_runtime::areas::TreasuryPalletId::get().into_account_truncating()
    }

    pub fn staking_rewards() -> AccountId {
        atleta_runtime::areas::StakingRewardsPalletId::get().into_account_truncating()
    }

    pub fn liquidity() -> AccountId {
        atleta_runtime::areas::LiquidityPalletId::get().into_account_truncating()
    }

    pub fn liquidity_reserves() -> AccountId {
        atleta_runtime::areas::LiquidityReservesPalletId::get().into_account_truncating()
    }
}

#[derive(Clone)]
pub struct ValidatorKeys {
    pub id: AccountId,
    pub stash: AccountId,
    pub babe: BabeId,
    pub grandpa: GrandpaId,
    pub im_online: ImOnlineId,
    pub para_validator: ValidatorId,
    pub para_assignment: AssignmentId,
    pub authority_discovery: AuthorityDiscoveryId,
    pub beefy: BeefyId,
}

impl From<ValidatorKeys> for SessionKeys {
    fn from(val: ValidatorKeys) -> SessionKeys {
        SessionKeys {
            babe: val.babe,
            grandpa: val.grandpa,
            im_online: val.im_online,
            para_validator: val.para_validator,
            para_assignment: val.para_assignment,
            authority_discovery: val.authority_discovery,
            beefy: val.beefy,
        }
    }
}

/// Generate a crypto pair from seed.
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Generate an account ID from seed.
/// For use with `AccountId32`, `dead_code` if `AccountId20`.
#[allow(dead_code)]
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Generate authority keys
pub fn authority_keys_from_seed(s: &str) -> ValidatorKeys {
    ValidatorKeys {
        id: get_account_id_from_seed::<ecdsa::Public>(s),
        stash: get_account_id_from_seed::<ecdsa::Public>(&format!("{}//stash", s)),
        babe: get_from_seed::<BabeId>(s),
        grandpa: get_from_seed::<GrandpaId>(s),
        im_online: get_from_seed::<ImOnlineId>(s),
        para_validator: get_from_seed::<ValidatorId>(s),
        para_assignment: get_from_seed::<AssignmentId>(s),
        authority_discovery: get_from_seed::<AuthorityDiscoveryId>(s),
        beefy: get_from_seed::<BeefyId>(s),
    }
}

/// Properties for Atleta network.
fn properties() -> Properties {
    let mut properties = Properties::new();
    properties.insert("isEthereum".into(), true.into());
    properties.insert("tokenSymbol".into(), "ATLA".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), SS58Prefix::get().into());
    properties
}

fn default_parachains_host_configuration(
) -> runtime_parachains::configuration::HostConfiguration<polkadot_primitives::BlockNumber> {
    use polkadot_primitives::{
        vstaging::SchedulerParams,
        {node_features::FeatureIndex, AsyncBackingParams, MAX_CODE_SIZE, MAX_POV_SIZE},
    };

    runtime_parachains::configuration::HostConfiguration {
        validation_upgrade_cooldown: 2u32,
        validation_upgrade_delay: 2,
        code_retention_period: 1200,
        max_code_size: MAX_CODE_SIZE,
        max_pov_size: MAX_POV_SIZE,
        max_head_data_size: 32 * 1024,
        max_upward_queue_count: 8,
        max_upward_queue_size: 1024 * 1024,
        max_downward_message_size: 1024 * 1024,
        max_upward_message_size: 50 * 1024,
        max_upward_message_num_per_candidate: 5,
        hrmp_sender_deposit: 0,
        hrmp_recipient_deposit: 0,
        hrmp_channel_max_capacity: 8,
        hrmp_channel_max_total_size: 8 * 1024,
        hrmp_max_parachain_inbound_channels: 4,
        hrmp_channel_max_message_size: 1024 * 1024,
        hrmp_max_parachain_outbound_channels: 4,
        hrmp_max_message_num_per_candidate: 5,
        dispute_period: 6,
        no_show_slots: 2,
        n_delay_tranches: 25,
        needed_approvals: 2,
        relay_vrf_modulo_samples: 2,
        zeroth_delay_tranche_width: 0,
        minimum_validation_upgrade_delay: 5,
        async_backing_params: AsyncBackingParams {
            max_candidate_depth: 0,
            allowed_ancestry_len: 0,
        },
        node_features: bitvec::vec::BitVec::from_element(
            1u8 << (FeatureIndex::ElasticScalingMVP as usize),
        ),
        scheduler_params: SchedulerParams {
            lookahead: 2,
            group_rotation_frequency: 20,
            paras_availability_period: 4,
            ..Default::default()
        },
        ..Default::default()
    }
}
