use std::{collections::BTreeMap, str::FromStr};

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
use sp_core::{Pair, Public, H160, U256};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

// Frontier
use atleta_runtime::{constants::currency::*, opaque::SessionKeys, AccountId, BabeConfig, Balance, BalancesConfig, Block, EVMChainIdConfig, EVMConfig, ElectionsConfig, MaxNominations, NominationPoolsConfig, RuntimeGenesisConfig, SS58Prefix, SessionConfig, Signature, StakerStatus, StakingConfig, SudoConfig, TechnicalCommitteeConfig, BABE_GENESIS_EPOCH_CONFIG, WASM_BINARY};
#[cfg(any(feature = "testnet-runtime", feature = "devnet-runtime"))]
use atleta_runtime::FaucetConfig;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;

// Parachain
use crate::chain_spec::mainnet_keys::ValidatorKeys;
use polkadot_primitives::{AssignmentId, AuthorityDiscoveryId, ValidatorId};
// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

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

// Dev chain config
pub fn development_config() -> ChainSpec {
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
                // Initial PoA authorities
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
                // Initial PoA authorities
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
    // endow all authorities and nominators.
    initial_authorities
        .iter()
        .map(|x| &x.id)
        .chain(initial_nominators.iter())
        .for_each(|x| {
            if !endowed_accounts.contains(x) {
                endowed_accounts.push(*x)
            }
        });

    let num_endowed_accounts = endowed_accounts.len();

    // stakers: all validators and nominators.
    const ENDOWMENT: Balance = 75_000_000 * DOLLARS;
    const STASH: Balance = ENDOWMENT / 1000;
    let mut rng = rand::thread_rng();
    let stakers = initial_authorities
        .iter()
        .map(|x| (x.id, x.id, STASH, StakerStatus::Validator))
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
    let evm_accounts = {
        let mut map = BTreeMap::new();
        map.insert(
            // H160 address of Alice dev account
            // Derived from SS58 (42 prefix) address
            // SS58: 5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY
            // hex: 0xd43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d
            // Using the full hex key, truncating to the first 20 bytes (the first 40 hex chars)
            H160::from_str("d43593c715fdd31c61141abd04a99fd6822c8558")
                .expect("internal H160 is valid; qed"),
            fp_evm::GenesisAccount {
                balance: U256::from_str("0xffffffffffffffffffffffffffffffff")
                    .expect("internal U256 is valid; qed"),
                code: Default::default(),
                nonce: Default::default(),
                storage: Default::default(),
            },
        );
        map.insert(
            // H160 address of CI test runner account
            H160::from_str("6be02d1d3665660d22ff9624b7be0551ee1ac91b")
                .expect("internal H160 is valid; qed"),
            fp_evm::GenesisAccount {
                balance: U256::from_str("0xffffffffffffffffffffffffffffffff")
                    .expect("internal U256 is valid; qed"),
                code: Default::default(),
                nonce: Default::default(),
                storage: Default::default(),
            },
        );
        map.insert(
            // H160 address for benchmark usage
            H160::from_str("1000000000000000000000000000000000000001")
                .expect("internal H160 is valid; qed"),
            fp_evm::GenesisAccount {
                nonce: U256::from(1),
                balance: U256::from(1_000_000_000_000_000_000_000_000u128),
                storage: Default::default(),
                code: vec![0x00],
            },
        );
        map
    };

    RuntimeGenesisConfig {
        babe: BabeConfig { epoch_config: BABE_GENESIS_EPOCH_CONFIG, ..Default::default() },
        balances: BalancesConfig {
            balances: endowed_accounts.iter().cloned().map(|k| (k, ENDOWMENT)).collect::<Vec<_>>(),
        },
        sudo: SudoConfig { key: Some(sudo_key) },
        session: SessionConfig {
            keys: initial_authorities
                .iter()
                .cloned()
                .map(|keys| {
                    let id = keys.id;
                    let stash = keys.id;
                    let session_keys: SessionKeys = keys.into();
                    (stash, id, session_keys)
                })
                .collect::<Vec<_>>(),
        },
        staking: StakingConfig {
            validator_count: initial_authorities.len() as u32,
            minimum_validator_count: initial_authorities.len() as u32,
            invulnerables: initial_authorities.iter().map(|x| x.id).collect::<Vec<_>>(),
            slash_reward_fraction: Perbill::from_percent(10),
            stakers: stakers.clone(),
            min_nominator_bond: 10 * DOLLARS,
            min_validator_bond: 75_000 * DOLLARS,
            ..Default::default()
        },
        elections: ElectionsConfig {
            members: endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .map(|member| (member, STASH))
                .collect::<Vec<_>>(),
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
        evm: EVMConfig { accounts: evm_accounts, ..Default::default() },
        nomination_pools: NominationPoolsConfig {
            min_create_bond: 10 * DOLLARS,
            min_join_bond: DOLLARS,
            ..Default::default()
        },
        #[cfg(any(feature = "testnet-runtime", feature = "devnet-runtime"))]
        faucet: FaucetConfig {
            initial_balance: 1_000_000 * DOLLARS,
        },
        ..Default::default()
    }
}

pub fn mainnet_config() -> ChainSpec {
    ChainSpec::builder(WASM_BINARY.expect("WASM not found"), Default::default())
        .with_name("Atleta mainnet")
        .with_id("mainnet")
        .with_chain_type(ChainType::Live)
        .with_properties(properties())
        .with_genesis_config(
            serde_json::to_value(mainnet_genesis(
                mainnet_keys::sudo_account(),
                mainnet_keys::validators(),
                mainnet_keys::prefunded(),
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
    const VALIDATOR_INITIAL_BALANCE: Balance = 75_000 * DOLLARS;
    const STASH_INITIAL_BALANCE: Balance = 25_000 * DOLLARS;

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
        sudo: SudoConfig { key: Some(sudo_key) },
        staking: StakingConfig {
            validator_count: validators_keys.len() as u32,
            minimum_validator_count: validators_keys.len() as u32,
            invulnerables: validators_keys.iter().map(|x| x.id).collect::<Vec<_>>(),
            slash_reward_fraction: Perbill::from_percent(5),
            stakers,
            min_nominator_bond: 1_000 * DOLLARS,
            min_validator_bond: 5_000 * DOLLARS,
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
        technical_committee: mainnet_keys::technical_committee_config(),
        elections: ElectionsConfig {
            members: validators_keys
                .iter()
                .take((validators_keys.len() + 1) / 2)
                .cloned()
                .map(|member| (member.id, STASH_INITIAL_BALANCE))
                .collect::<Vec<_>>(),
        },
        nomination_pools: NominationPoolsConfig {
            min_join_bond: 100 * DOLLARS,
            min_create_bond: 100 * DOLLARS,
            ..Default::default()
        },
        council: mainnet_keys::council_config(),
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
    use crate::chain_spec::mainnet_keys::ValidatorKeys;

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
mod mainnet_keys {
    use atleta_runtime::CouncilConfig;
    use super::*;

    pub fn sudo_account() -> AccountId {
        AccountId::from(hex!("a9a55e9de3c8d70c9f1107b58e33070fa816335c"))
    }

    pub fn prefunded() -> Vec<(AccountId, Balance)> {
        vec![
            (sudo_account(), 3_000_000_000_000 * DOLLARS),
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

    pub fn validators() -> Vec<ValidatorKeys> {
        vec![validator_1(), validator_2(), validator_3(), validator_4(), validator_5(), validator_6(), validator_7(), validator_8(), validator_9(), validator_10(), validator_11(), validator_12(), validator_13(), validator_14(), validator_15(),]
    }

    pub fn validator_1() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("85fc1309AcD66a3a6109487980D3e186B5718D51")),
            stash: AccountId::from(hex!("7c7e63c46e4E1cC71a759f591197A26A98a6146b")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("5cafb59804791ef43e5f9f896b11a9d708b15d5e244ff2395f6ae4434546642f")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("e3ae18d25daca40e780b86058c035b9ae2d49375d82ac731c7943149eb05b39e")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("7e5a67c3c9aa779818315147c12611a21f93c4f73a775d9f8086ba0674fa3378")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("807e850968725bf7357b4304f9c5bad06210b91d075c25049265ae6e6688bd7a")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("e658bcfd3c0fa580fdce24d4eb4160e259e9598c703c07796f794240a478e276")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("080a82e2e94248255fe064ed4c1e613467b938fffe2aa943d2f5e2e3fa912b79")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("0377c4adfc1777059fa2c5d18ef1e3d70eddc4247a42c34de1d736434636e95512")).into(),
        }
    }

    pub fn validator_2() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("881fe63dfEE7611CC005e2b0e4577B8c3BF0D478")),
            stash: AccountId::from(hex!("ccfe5bb109F0abCdAF88Ecf3e6C8ac22Fa66b389")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("42ff20efa85dd4fab0bbf5646199abdc8343e39de7f379c13fba234d1479ad21")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("434df421db06739df7d7df50ce433590f2b0a813c83a1883fb3669208fdf71cc")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("f6c4cd45e5fde3187a4946de21c5da413a7065534640e62e9919f8beb41df82d")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("40311d2d9855b0ca10af89cc42936d4b22795b79ff638a2853cefe829b14d407")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("4e7901e11e009918c7e0b359b3b39a3ff628ea4e925cd699c5831f4dfc602319")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("d42b8f99b172d493f9a1a4c3a233fe6083ec1cfb6b3431d865037f77bc543238")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("0364b8445825d88c0429db6217e6032f77afa22292acec927fa69cd3abce86888f")).into(),
        }
    }

    pub fn validator_3() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("2e174dF22216994733837135D477E43D9176157a")),
            stash: AccountId::from(hex!("b7Ceed556A295F3b6eDad9084ba075888C1cCf5A")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("107b419f99f2bf1c2731a4c31aaecdad4b59f2389699f331df8c41ace255191f")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("2f32c7cbcd3c665c8b0283a9b33509b71b1d2ee318c65eb8d4e4721cfa1bf353")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("ba4fa81bae920bb75f47a483603fddfddcce66b3efff3ede4aaa78c0fbf5e864")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("d257ab4efd39866f4f35ea523963335141295a4c2dfd6e7e4ccea381e7ac1101")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("02a4e4ad0375491b61551a0a12ad91bc20870def930f1f0ead8c48476fac8c5d")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("669622b3918179bda9c7656500ed81bedfe02afb98d8050ae15ee02c6fdaa74d")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("03e9dd7c1021e1518fc9e149456f8958cc4c0fb81059a87712e4931b188c67ae6d")).into(),
        }
    }

    pub fn validator_4() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("D65393Ec3293dcFDA18F479A150423a9e55f404C")),
            stash: AccountId::from(hex!("879c617Bb9017a9B796341172619d7d078543a55")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("fe3a0bd3299212cccd9d05789ffc7fabd31fb9e213b91aa338104fb859cd841a")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("a1a18c53addecb5694ccdaecc8c0dc048da3a1b571a7ecd652b19203e8de8665")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("64154a027538831e5fa405980639857a108540a06ddac783d641a249031c685d")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("ba3df39921c430f97c3ace3aef2ae09e185654dd81d370adc6983232a6e09043")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("52440315c82dfe44f48cc75a9b3ee61f3b4837a4c2bd129bf65304e366211635")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("46c924fb6758d229300834682dd8e15378a0fe05db9a20f3d75ea9e79d02ca79")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("03e4a6649ce9875c0bcd3fa8800fcabac3c8b3b47128c22124155b855da0dba83b")).into(),
        }
    }

    pub fn validator_5() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("8AB443430fa99B3fd8d5FD6EF295185c5d8b8935")),
            stash: AccountId::from(hex!("dd3357c6f8753AB3906CECc7F54aA6e852F39763")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("bed6a5baf8afe333b740f387cb457e8befebbe6512d8cd4f8288998ad802ac09")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("1235d22b3f3c1a40709ff3217fbc735d1d360ed28a3ce4cdd47565fa92604a52")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("7e0aaf255400f30177f658a7a9837fd3ad7cf46396c27feaee648f3fe865727a")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("8a7c33acd91697a7d7ed68a0330f519fdde235e8dbf18b7544168e1b26da8a0e")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("d0204ced862f962035f6a644851c0b67fb50edcf41202f3639a4a76749b9e232")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("ccdb2092922257e6c2f3ec85a45d0640bad236008bdbc7a6ebf4dd8a0df0e579")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("035a75bc67bef1e4787e5bbf1defa382126d5ebce2ada94c56fc97a7aba707d2d2")).into(),
        }
    }

    pub fn validator_6() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("6ad59e267AaD56F3Afba683183a50CF7d23F84b7")),
            stash: AccountId::from(hex!("4B79B51a894Af77831D28d94a3704CA03c624bD7")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("66accdac232812089280c4fb37798e773706d51ef25db5b4fe6057d60b816f1b")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("63c026f71a62279ba405e0338e13976ff17b46ac71995f9d51fb1bc21eb3c289")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("7682c1ab5ec0869acf2b0e338bda0f7b7c47c98165aff9220c43135d0ad8884a")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("12c5ce15aec11625c73a17be7fa586bc698b80e391b17da0a447cf7db6b8aa04")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("ae90e9bc5b558339a191c92843d541117ed08af9d21bb8a295c91bec53a68511")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("50b28446c1e8145883b34c91f8dd72c0d29c355b5624bed1ea57e6388132c701")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("03da57fabfbcb95ec77163509d59572ddc11c1f7da4afe74926a3dbb86f251ba9a")).into(),
        }
    }

    pub fn validator_7() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("8fd60f3aA39eA47b7975556991D20Ca2ee0Ec093")),
            stash: AccountId::from(hex!("f656C659eF034795DDBFb2A2e6D2B2ac33136B73")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("505945405ab860d9713286b85f9a963e031289246984550c1d5ea9c550f98977")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("843a3944616109213a8ee6988ad5709a0893ae5a7f1a28a939d3df3d63f1d828")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("4085bc1ef3e46f86908ac8504c1d0b709cc5779087254a0593d6c22bf496c011")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("72f579443a54fbf246f2aa865c6f87c72e722f408096ea87c06eccdfeff30e09")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("d65ac005e8f4f539030748338552ec7b63246df9bf2b762b4315052d25c6af05")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("aa15fa02de7b51a92d69e0dc30e82016c3768898978edcca2cfb356df3078326")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("02e8bc1cffd3d5cf4216c47b6be40399a97dd5e276a392e642d5003a75993477fa")).into(),
        }
    }

    pub fn validator_8() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("b77bb935Eb52d827ad60730C68BB9553f588a41a")),
            stash: AccountId::from(hex!("9f7200a5bA24608f52fd66d90E6ae1EDead5165A")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("6a6348af0c69ff5b23ca73b993656cc9337e679d9ef4380d1d31d5bcf7deec78")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("6617813a31e91507211dfbedf82f1ddb73327e06ba060c3e90f24895c82831b8")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("4204171d356dab2baee09175684f78f05f88a5d85f2f838f50acae2b307f8f27")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("34e966d72111c6aedcd9a7cdb7239c422ba522be6f11116d94bf776ba6e46575")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("d4bd278426b0acc51a12383e4999a5fff63cc9b9a2d29bd61d984401e7a57713")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("64c94dbc6b5ba2c8968032f501bb6b4c29b1b4152e64f5640b642c3b2a54fe4b")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("038280402193436973a34b73466955ee3f539f8bd8d3654bc619c9de8ccd307a25")).into(),
        }
    }

    pub fn validator_9() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("3D5FD02d9CCD7dfd3cF647F766836B6eeaBe8756")),
            stash: AccountId::from(hex!("2aac7fd8461728Abd28a1979eD1f5d896901237D")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("48d5689e3ed5bc0410416a96d72f1931d28afb828dabcebc6a036fdeab915c5e")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("5209cefd931f47de71ee7c74b5982052b9bdd021a7846a3914586417515a2d05")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("3c50af70d043a87421fb7006fa8253889f4e3bdbd5aef1f485f00cd05981b72a")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("660f9741985e1c1e63fb4c03b03f78b96e872616a6f7929980be82314756962f")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("2c39422d43378f34d516d90221e4ea9c6a61b2b39fbe3edcf273c2b7668ac61a")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("52d2890bbecd3d02f377a0a9e187fa437185dae485cb04019be8f0b79422f605")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("032d2d5f0813ae81587bd09e738ab715b2d4c0d719369ea4dc81e8623fd3691415")).into(),
        }
    }

    pub fn validator_10() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("E24200FBad898Caf3775eE45051A30bB561b68E8")),
            stash: AccountId::from(hex!("b79A65AEBAc46050Fb77f541f2673c58E1D7D618")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("1e2a2d029753aaca7c0ba169ef20e502eaa532750371327b2f2352c9f2475924")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("3cf39355b9679ec48a79fcdc9ffed1594dbb4f198988c8a797005a6c7efa0c6a")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("6473a1203d0b0fcf568b36ba1f9f5665dad01fc9e9be57fa42d5f572233c840e")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("aa03037c4db1605c902f6ad170f1a0d722bacde506a3330b537a99490696704f")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("94a744182bc09712438abb1c6bc5e4ae8d4950e768acfa7da4b103b2fdd3512d")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("3e32284750e78cf0abc0e108a7cbf6d31d81991b335592e12b5e0c652f41981a")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("020ea94f1675ea92f9efaf3cce8d45cb52e93b5bf98e3ffd2d0e87e4781860e483")).into(),
        }
    }

    pub fn validator_11() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("A7e631085f3c0E49002A29F366db133E50595068")),
            stash: AccountId::from(hex!("f015862613dB2340b0c983d295D8c969A5e5E9dA")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("c823d3e5c8ac2b9074099699d46367332535498fd0d4408643673e2786556a57")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("b9842fa608dc56c7dbd091bda22d42cc776bdb33ec24987964a52894da0738bc")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("6095d9a2c6fbba29a5ffa0826f65aed6415c68b99b5f06344b4937ea24e5e37b")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("82e481a14899933f6b8384f7486e8e0592355506821dda912ea1bd9076dc887e")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("a685cd88bd30d372f4b02b082450bf879daef02c0cad39ea74bb5609a139757c")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("cc44d7412063a4e22a53a7c875401f5299bdc7c033b64a4763a14f0a96cdbd47")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("032ecf253d7d976c3e61d54ad8b9441b7f9db13a0b6ae9a6d6118bd2b58f78a046")).into(),
        }
    }

    pub fn validator_12() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("D41c0Db25c1B85B12c64Ae6e706C691e867f432F")),
            stash: AccountId::from(hex!("5feF6c771F0b1132C42cB32ddd7aEA38af1f4548")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("72f96c90a981039bc1954ad6d9eac4cd689ab828d46d4e4f6e93a9e55b668f3e")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("5421399c8054c6a7c3109f1c6eca4cf08925b81d947c12c18466c9d992c149b5")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("a02a5d34ecf550cce15b33d1cb4882598f3475fbb919ca8f8143c165a7e7e93f")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("462b62b35e5c1729825dfd6be4950f12d4509aaf252d74a3bfc4dcf340a64250")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("7c8ede8308f32559e1eb84a1dc66d80a7e081248a5b0e9629e58d8c4f3ea2416")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("c46857ac2f9fe256a3e208ddc4cb69aff029546b47694dc89e7dcd53f58a1d27")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("03d714d001a3f28da04ab341f69213d0f923c8a18a7227130ab909fb02c8f73796")).into(),
        }
    }

    pub fn validator_13() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("937Fc6E28689570e1851e2388FB2B3B286D22b88")),
            stash: AccountId::from(hex!("9a136DAd9CA9f895411aA5AdBD4bA0d6a0F145B8")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("eed39615624af5596a033333a63321dfeb688a2bd4390a1dfe1d4b09cef04357")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("54fc0918b2f86f74c6de8220d5f133cc2b536c211fe6d6a16defb8b30902baf2")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("ea5e5e43b73b120de6216ad524b0b86e30b5329c2817f0494ec8a9738de36626")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("c47da15b3424db9d94629260c364cfd938e4a6b961eca250d82c401735df5509")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("064605209e359b400548cb6ae8b5e7b83376f5da9d99c901f9f04b88babdf632")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("36e561cf7ccb2ba2e463ddf27f37a8ae7ef98a16dbf2501f9a46e8316ed9ea2c")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("02ec5c45c1b7ea5e72e5ace360d470ab0630f34be9f3377ab026a3ef4f157fce5b")).into(),
        }
    }

    pub fn validator_14() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("79A65ccb3daA8389Ea4669406f2439acBAeAC5d4")),
            stash: AccountId::from(hex!("348c4bB70F10200C7AfF6A2712987c82Bf25E7C3")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("68ee4d167ad36a319fba5282ee64c5494092c661451131071bc20399e159a239")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("5afd0ffa37489fc4da7fda3db87bc4f5ba2d2bd467a82ad6642257832c7ee8de")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("de19f7029d12083a677a563cb643f8d56c3114474d8a8d1892ef8f179612d738")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("6c2674686c2270901620e9b3ae36a2ea835afa53d065a87baa6d060573b3171e")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("cabd04f55fb158c227241b9921f58c522d3869c47f3929589692fde6c98ff338")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("5606a70929def2f52e1d73167c8d8d9cf2fc85587f7b3f01b04b09259b076e6a")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("0341a742d08d3c6d98a5b4ef44a88247215c07c4e645f94da4dc9146e730eab7af")).into(),
        }
    }

    pub fn validator_15() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("37E1Bd705893ba4f38181Ff73E848cCe71d99771")),
            stash: AccountId::from(hex!("9dc4DaaBEB464Ca48e1f0Be5497DFc395a8FE1Fb")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("8cf9c2419210f70ebc2b3a1a84a1c5745e01c5da56b70426ecbe7df25ce14e07")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("c8a53393fc899e353bd508554605a629ed85173990aa82f698eb6d4098c16e17")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("5090031b6feddc697ced0f8f342a6845f31c68f9a7c91e95cf60fa80c8ad815d")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("b044567faacc8acd783f41a4e523209f0793aac5862f827a13b73d097b438f3e")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("eec7c3734e310ee0a899b186e1fca30b098060026f0776f6903d037d9301c72c")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("7ce777124936df39053866b446721ae4a240d80580e2e1863e40b24e4c6cdb43")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("0396b4e92806cc80fe3fb6aca145a20c2a774c11bd643ddaf917da6c2a6fcdd267")).into(),
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

// Chain properties
fn properties() -> Properties {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "ATLA".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), SS58Prefix::get().into());
    properties
}
