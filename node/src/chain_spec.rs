use hex_literal::hex;
use std::collections::BTreeMap;

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

#[cfg(any(feature = "testnet-runtime", feature = "devnet-runtime"))]
use atleta_runtime::FaucetConfig;
use atleta_runtime::{
    constants::currency::*, opaque::SessionKeys, AccountId, BabeConfig, Balance, BalancesConfig,
    Block, ConfigurationConfig, EVMChainIdConfig, ElectionsConfig, MaxNominations,
    NominationPoolsConfig, RuntimeGenesisConfig, SS58Prefix, SessionConfig, Signature,
    StakerStatus, StakingConfig, SudoConfig, TechnicalCommitteeConfig, BABE_GENESIS_EPOCH_CONFIG,
    WASM_BINARY,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;

use polkadot_primitives::{AssignmentId, AuthorityDiscoveryId, ValidatorId};

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
    fn from(keys: ValidatorKeys) -> Self {
        SessionKeys {
            babe: keys.babe,
            grandpa: keys.grandpa,
            im_online: keys.im_online,
            para_validator: keys.para_validator,
            para_assignment: keys.para_assignment,
            authority_discovery: keys.authority_discovery,
            beefy: keys.beefy,
        }
    }
}

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
                alith(),
                vec![alith(), baltathar(), charleth(), dorothy(), ethan(), faith(), goliath()],
                vec![authority_keys_from_seed("Alice")],
                vec![],
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
                alith(),
                vec![alith(), baltathar(), charleth(), dorothy(), ethan(), faith(), goliath()],
                vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
                vec![],
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
                lionel(),
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
            max_nominator_count: Some(100_000),
            max_validator_count: Some(100),
            ..Default::default()
        },
        elections: ElectionsConfig {
            members: endowed_accounts
                .iter()
                .take(num_endowed_accounts.div_ceil(2))
                .cloned()
                .map(|member| (member, STASH))
                .collect::<Vec<_>>(),
        },
        technical_committee: TechnicalCommitteeConfig {
            members: endowed_accounts
                .iter()
                .take(num_endowed_accounts.div_ceil(2))
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
                mainnet_genesis::prefunded(),
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
            min_validator_bond: 5_000 * UNITS,
            max_nominator_count: Some(100_000),
            max_validator_count: Some(15),
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
            min_join_bond: 1_000 * UNITS,
            min_create_bond: 1_000 * UNITS,
            max_pools: Some(1_000),
            max_members_per_pool: Some(10_000),
            max_members: Some(1_000_000),
            global_max_commission: None,
            ..Default::default()
        },
        council: mainnet_genesis::council_config(),
        evm_chain_id: EVMChainIdConfig { chain_id, _marker: Default::default() },
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
    use super::ValidatorKeys;
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
mod technical_addresses {
    use super::*;

    pub fn treasury() -> AccountId {
        AccountId::from(hex!("0000000000000000000000000000000000000001"))
    }

    pub fn liquidity_reserves() -> AccountId {
        AccountId::from(hex!("0000000000000000000000000000000000000002"))
    }

    pub fn liquidity() -> AccountId {
        AccountId::from(hex!("0000000000000000000000000000000000000003"))
    }

    pub fn staking_rewards() -> AccountId {
        AccountId::from(hex!("0000000000000000000000000000000000000004"))
    }
}

#[rustfmt::skip]
mod mainnet_genesis {
    use atleta_runtime::CouncilConfig;
    use super::*;

    pub fn sudo_account() -> AccountId {
        AccountId::from(hex!("226e562Ca44a997894d4eAe15e147b145387DC12"))
    }

    pub fn prefunded() -> Vec<(AccountId, Balance)> {
        technical_allocation()
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
                AccountId::from(hex!("f4585fF1A4DBaa05A5c13807C301feb30fF982D0")),
                AccountId::from(hex!("72E2812EBB8fCA781286e09B42Eb9EC5fF2E6576")),
                AccountId::from(hex!("5753aEe0cE28478e1C76d39f762144aECF7B2635")),
                AccountId::from(hex!("C3755A655e9408C263830b1637279703CA3AAE0d")),
            ],
            ..Default::default()
        }
    }

    pub fn technical_committee_config() -> TechnicalCommitteeConfig {
        TechnicalCommitteeConfig {
            members: vec![
                AccountId::from(hex!("629b2b2003FE9762bBdDf4BcCBc58727687B5f69")),
                AccountId::from(hex!("Ef20c48CAd2c3e1A877aBB212504fca96414C4B9")),
            ],
            ..Default::default()
        }
    }

    pub fn validators() -> Vec<ValidatorKeys> {
        vec![validator_1(), validator_2(), validator_3(), validator_4(), validator_5(), validator_6(), validator_7(), validator_8(), validator_9(), validator_10(), validator_11(), validator_12(), validator_13(), validator_14(), validator_15(),]
    }

    pub fn validator_1() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("f4585fF1A4DBaa05A5c13807C301feb30fF982D0")),
            stash: AccountId::from(hex!("629b2b2003FE9762bBdDf4BcCBc58727687B5f69")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("1687155208fbb09cc5f3012e4e5ba21bd3c22d08141eae1f0bb693e47a6ced7f")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("d4b42db4a721bcf06a4aec0ecfb97fa48f71522db4d5d676fc05e92e56b786cf")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("5e8ef71cafca582ea996a71780d14224126cbc6f22e8b933311ad51b52f4c76f")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("c627e51cf39bf07ecf84404fb8d1c0b78d1e268054bd20b0a64b47f5a8328733")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("fc9be628151f1638cf3eaead1b5223dda466a9a1534d04cf75999221cb581c41")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("9aba447aeccc10f2ae14bd3cceb0c16093069c43b6e40eba15c081fb116ee955")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("02d6a6e49d0c71f9dd8f1f7492040926030a5d2f6f2d355060088d54e5b5484dd0")).into(),
        }
    }

    pub fn validator_2() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("72E2812EBB8fCA781286e09B42Eb9EC5fF2E6576")),
            stash: AccountId::from(hex!("Ef20c48CAd2c3e1A877aBB212504fca96414C4B9")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("1e9fe343dd78478b3c4b75d2e68588974f9bea74099694129fc1f6c1d36dd32d")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("f0faac54dc11d7af98631e18466196c5890c6e0ead401421e64d55151ff88443")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("765f94addb4c0b6fd7b798699cd0e2e1ba1d47d69f1ffd1f42ed7a588cfad578")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("be0909c1dbf166bccf5b384dad45b44f6e54906027d6dd0a7f0f91e96a8bd906")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("2ab932c0dc72e11506101ed16ef3bb100d75b271c48813fd771c851deedca157")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("2273fa1c5bea76286811df3324c4a1ecb7feb42b4f9c92990c51e6442b087b4f")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("03404903e27d5a93fc1c02c03c47a881c1f7c69ef7cc34a2fe225d1293007967e0")).into(),
        }
    }

    pub fn validator_3() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("5753aEe0cE28478e1C76d39f762144aECF7B2635")),
            stash: AccountId::from(hex!("3a15e55f51D7683FA7A74c4Af5e796a3198c738b")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("bcdceb876e049ad92bb08ebc739fd6da8598ce7d4436b219932e4696c9c2226b")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("5ef7edadd51e1b1dcbf3aaf94606be8587ae203d2746a0388a00eeeae6620776")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("e67a9b71a3a6716c7bcfa797afbc4842752f113488b0ebf7092f8ef5955ce531")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("a0224e71a848a82dcc31e76957cdba48eceb15964066b8ad541407f1a1d80907")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("de6c8545eb59c4dd880d2d8f4ddb39da91082c61ef944c3bbac233b59555626a")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("487a039388f076cece25c3597f49421dcbf55f8b21d920baf6d5cbaf2ae72323")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("0319c7e221a18e5d9999307884e0693f2020541a33e9a45a423352d64cb34de946")).into(),
        }
    }

    pub fn validator_4() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("C3755A655e9408C263830b1637279703CA3AAE0d")),
            stash: AccountId::from(hex!("4c81ab855E1b742D8f77644fC99FcfB198177743")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("dec0eea0b8d5a502bc26e8a50a1a9741a3589e96ab0450d1b59e3ffc3cbcd612")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("b70b0911314113ff9e146bca64b0b493048e6a1cdbd98c3a57ce7441da818c66")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("ca6852fa578d555ef52fb174b83f3b01cf719c012f05bd34e3922a928287464a")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("8692c6817b149f56472b8a49cb1486fdbbc77bf6b517ce6fb4fb63315201c26a")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("c0a158e5cb7d19bf15faa50edae6fbbff6e40e0e0badfd7e9d0b305d2a1a493b")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("56aeabc93d6031e59fde3eaed61e4f56c2ba42662caa81cc9996a6dc6a95a372")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("0342496ab7769f7129ee6ee1eef1248f338cee2c7bc86bb72b359351132fc7d823")).into(),
        }
    }

    pub fn validator_5() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("b4f509853781dAB0000312A0E44A30EcF6c69Ce6")),
            stash: AccountId::from(hex!("E1E1098d7B37e217c24cFF33f0CD12F570069491")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("a0dcb3be997f7119e20357eebb0912abae7c446993295d9f4d1847fec575394d")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("94826edab8a9eda2c46fe89f46fb4236bf86ff3e71b092dc914592ead984d89f")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("e61d383ffba35d05d0b4deb3c22705abd9f576187c36d296698e87de8a58f214")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("eecec470b6c6999cb37d47649e8fefb7897e9b714c76602c80d5df81c6432b1b")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("26745165b685cfdaf92559f59478b16fc1db20785bf50a025abac64ed4bd1b68")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("a68473b9b7ad0e45dc240fd04952efff3acb2294914ab0a42a7f051bf6775434")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("021604dbb42fc7f8d5920f50c86f054cd3f7eb45610146fd2b605e208d5f21230b")).into(),
        }
    }

    pub fn validator_6() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("6056f158410Daaf9Da13b3999b033018E1f92CA2")),
            stash: AccountId::from(hex!("7F3F3222bf0AB9db116FBFbB3cC140a28e8A22b9")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("d6c5909fb50ef0bed4bbd9d24fbf99660d7f9c2793652dfb45aca12dad254036")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("400cf2a5208fa590620db1d71df0aa95ec89e83cb9d3c1bd44100f25c073bce5")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("44c4a07276132ba12879d9f20fb0e797a123677433cd09cb4de630d9b7eae85a")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("a46eb16cd66cbec566f0f0e7c64d2953e51b77d97cc61566747a3db312f91d66")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("52aeeadfe983702284d5ececb1222a5b426c10c7971dce7bb0efb5a59a7edf5a")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("ce1e75566b14efc70b0a549c696569064a83b6c833193f2dac6356eb2c28a67a")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("0259e460b35f58b65125f77f2f440cb4d47ea5a16e25a0b3a79c762bc6e2e406a5")).into(),
        }
    }

    pub fn validator_7() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("81b303f80EF2B326480533CeB3fF632A9EeaA08c")),
            stash: AccountId::from(hex!("18B802e55e5Fa859416Fe1A307aFd843a9939688")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("be06dd12a46b9218db5a93edd53599aa8e1293afb64b79a2da58c30b79e58b50")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("e2d89bf1bbae47fa132f92f378c62f4be906a11b37d9eb7485f0bf3ea3ff6575")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("3ac33c30b40e5fe88e17bdf7c2adf3b9036dd6b58c422f54cecde580c262c765")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("a06f10ebe88f16dce5b9268d2bc7451d987688a24e00012a4f2fd25922159310")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("22285bbb06aaae2d2eb9e6a8836a7a52c5ce9e7eafc2382a12c1ea90d2eb4232")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("4839e7319212443344dfb623a66ad22e71cfceeabf2cd897b2ff5399ffa4a535")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("037240af63cd110bdadc6a4bbf1a50772282955f2fae82a82550782b560d216945")).into(),
        }
    }

    pub fn validator_8() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("f6b6EE195405CbA3B2561c10A77d7C2A05833566")),
            stash: AccountId::from(hex!("402e7930b88872347005Bb04ee6F3ECE26c2d2f5")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("98d8cc0db3dbf8060bcdd00843fbf407f4558ce088425f53dc64468a78563349")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("fe2a073a6625e89983e345353b9db461c63cfced8e540d49207ba55acf55ea35")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("d6ccdea8531f0b8dfe476fff9a7a32c1d1af6a61315c3760e7ffe178b2020962")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("4ef3aac61a56644e45d2314c7436ccccb2751f3845dbb30fec5cd364a53a4760")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("400d756a07a2b0df362e544630702f22bfd21b54e5726ced101b4f5aa714813c")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("1c775415dce33c068968b84a1d89cddeac6adf5107bc0d64cbe6485d81677707")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("032bd29f5f43d69c8c8ab1ec49faef71ceea057cdeaa53b28d9702a188968f83dc")).into(),
        }
    }

    pub fn validator_9() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("19118738Bad99b84168059A85d37FEb0845A374f")),
            stash: AccountId::from(hex!("ecd7b020AF9b13773A8feB3A08B031e6f7831C18")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("f883ca47e7fe17d8dbcd487401adba7166600cef548cafd3561f7e91c84d6739")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("e9cc536b2a242e392d5a09590cbc4c12c9ec4f7e22cf465ae8b576d6a8551291")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("9615cf77eb7ba8eae5dc3edb57ba2e5c58cd8266376be88eaaad950e0e812059")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("04f350f1d97225b8ef3c241982a632702cbf2a274faf4455904f2f041836757f")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("34c785aa5457d576215c7dac0c8920e4c1852401e812c9fe655bbe7ba0c39766")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("a288f8cf3f8d19af59007fac12e230c7b49e222024c4e7616455f2347d24d451")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("0258e0f8ef5b82b8597772c666bbf8782120d94257d6f7a283327eddd698906046")).into(),
        }
    }

    pub fn validator_10() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("8b4838C2F9b0589F339B582509cE23FEE2A3A342")),
            stash: AccountId::from(hex!("B23Fa8b8050D213204991b9723ac57E8667b2a9a")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("fc6f260a24209c32df10930605b4caff389544303cd04959f61f8d2d39c8662b")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("ed374f6393238751c2b5110a87500a1773f6f06d9f89160525d9be7caf486914")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("960effa74c02dc73b9edff7bc4f57ad03feb174d0c185f21a567fa74d108bc30")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("7c25aaea337e6da04174b27cc8699859e23587d3aa9741b0ee58a65c3b8fdd62")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("0ec583908980633ee9ff7adf408dda49dfa86a6fac2fb4aac00863548092d653")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("3ec050f14147d894ec9bcb01b6cd22bdf5ad836298e7f7122759a9edcbad6204")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("036212a40eebc11e49fc45dd067029107dba9a2e12520eb47d0b5292d9aad066a7")).into(),
        }
    }

    pub fn validator_11() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("75BE8Bc4F649040abf0384fD28517f7D23BAeCBe")),
            stash: AccountId::from(hex!("50035d01BdD01dbA568fff42F20332aA5D4D4129")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("fc4a2c2926c3348623804d0b63d5cf7acb75d0b3820fc9a777e4d057254c4011")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("0c6749dd8a1b42bdb2949368ad579100d8708fa3e6c42118356effcd87c5281e")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("e490ab333cc3643b247895a5ae39c58309a86b6b284535657f5169d9ff1d7b5e")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("ea63011fe41dbba9f68753ece04f938a53814b2d22b010a9af6836d0c825f837")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("2675dfdcaaa1d10b36a0906d91a0741bbd65b1e6435147f5542762b8974e881b")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("c8ecd75c1ae54981dcd9029588716d0f83efa6707c48c1a79e49b902ee40fa7f")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("0293edc97d44230de58427b08c4d4b84e3d20a71542e7f849a9efdddd4e27714e1")).into(),
        }
    }

    pub fn validator_12() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("558665a3a35DC6ca649B02a7259DE9a5b78CB058")),
            stash: AccountId::from(hex!("bEa9A57046110E7Cbb4e34c6053eCb4cb7494f87")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("5824d6510207e3f6e289fb3dbbcad037126089e7919bee054a26320514e55737")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("9ca96de1e1f8efda9327f2a3c99ea88dbeb2be10d4aa9f88086ec0fc4bd97c46")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("2ce4e129fffda719f26103a28150bd4be436ab7017d0fdccd2bfe4e6a4070029")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("14c2ecfbd9be5ab5cf57fc808b1bac4f3dfea6b79fd97d73f44d2b86ec9c192e")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("f659474dc24540dc162bc25a3cd6c2f0ccb65b4dde800b7e9617b039ffcace51")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("d63080438956af78fa40b005470da0ab7b6f795fb36a8a917b2544ce93fa1871")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("03a832315580d34671615ec05ca3285dfa80748bf0fc3d236e6eea054f1e9dd156")).into(),
        }
    }

    pub fn validator_13() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("265E0660f3C2279E156f266Ce83cE76133e312F3")),
            stash: AccountId::from(hex!("B08322a32CFef1A3210663177D85C79aB3C5A746")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("126f7e63a547895330c3952f4588f250ec31f45b47237551964e7309d4a20b37")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("6638d6978d3d819988e8bb55bdf79c8e47cd0ecf49f993044cc00dff8a593182")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("eada0c1cdea4a328b1fb927084d763f80866701af4a8014f317d4fe57499811c")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("7a432fe786c3b62e0483ed74986b7b0433e86bc0606bf0ff9eeedc81780b1316")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("c6559aaad167f438309718241453c6ce0c6f40fd0d756bdfaaea7686e2f55a0a")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("089d4c0fb2991865eec57eeea174c5e8b56a226ac3ac387767d930df3fb1bf1a")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("021161f421cad730e39c2d5845718e16925280af9da17466e18bea5d2205f238e2")).into(),
        }
    }

    pub fn validator_14() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("f6D98AA3D28F85403C24280C4d558D8AD19EEb74")),
            stash: AccountId::from(hex!("c21f897C6bCbBe29D5547bd6e42eFe986353b339")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("56cffedd3cbac6b1ce77593ee92b02354dde1e6607ad5ce278338bae4fc2677f")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("ef62b04a781d6be0434527ec0817596d72a4eab3da8fdb5862ccb64a10d47447")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("d2d3ce47535502e50c4c67c7f299e4c15362b1ad76bdcc1bb1ee8624fff26e0f")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("f6464d3aeccfb96f1c969a598fd61587130f74e66e166c2d6d72528145ea4f60")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("8c7186e2157e362d5c8877a448258aef4c36a3608d128cafe24a155492a17014")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("724d50c7f207f4eea3f6f4a2ae7ac4d0c6269e61438d48df1a80935e4c825258")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("03aea98267a55b78b148207cae0e3ac5bf683a590dc0e25cf0b7f14bea3f708ac7")).into(),
        }
    }

    pub fn validator_15() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("8A9224cF588D7c4754CB265228FF050815Da29F3")),
            stash: AccountId::from(hex!("cBC48Ac780166901fa1e0435C990A7a91b439Ef1")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("9ef99d4a46edef266a9d6e0ef9c065e6bbfb384b9985a30646bbece5e89e8539")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("6d5cd8f278a1544e46759acae56efd7ac9c8a69e788ee8818384b23fb4feacdc")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("76e7e69a3ca1da2760f7776361407c01a6b9891336fe0c00dcb2e627b0e6227e")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("18fa09afb23f555d787dbd5b49c97bfafb7902e46bde2fda6da1ca659abb744f")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("78b51464d5fdd529b82d66e23c97a8d836be7a326ce6eb272d8afd5c3a79017f")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("4c81e0ca3f78120fb9bbb5326e85d34028dc0f43d827816c9aa31f9619a5c419")).into(),
            beefy:                    ecdsa::Public::from_raw(hex!("03c8aa12c7724c13da51825cd2fd42cde883121d37e427287d909c9bf8657a249f")).into(),
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
