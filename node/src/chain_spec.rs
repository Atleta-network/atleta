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
use atleta_runtime::{
    constants::currency::*, opaque::SessionKeys, AccountId, Balance, Block, MaxNominations,
    RuntimeGenesisConfig, SS58Prefix, Signature, StakerStatus, BABE_GENESIS_EPOCH_CONFIG,
    WASM_BINARY,
};
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;

// Parachain
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
}

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig, Extensions>;

// Public accoint type
#[allow(dead_code)]
type AccountPublic = <Signature as Verify>::Signer;

// Dev chain config
pub fn development_config() -> ChainSpec {
    use devnet_keys::*;

    ChainSpec::builder(WASM_BINARY.expect("WASM not available"), Default::default())
        .with_name("Development")
        .with_id("dev")
        .with_chain_type(ChainType::Development)
        .with_properties(properties())
        .with_genesis_config_patch(testnet_genesis(
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
        .with_genesis_config_patch(testnet_genesis(
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
        .with_genesis_config_patch(testnet_genesis(
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
        .build()
}

pub fn mainnet_config() -> ChainSpec {
    ChainSpec::builder(WASM_BINARY.expect("WAST not found"), Default::default())
        .with_name("Atleta")
        .with_id("mainnet")
        .with_chain_type(ChainType::Custom("Mainet".to_string()))
        .with_properties(properties())
        .with_genesis_config_patch(mainnet_genesis(
            mainnet_keys::sudo_account(),
            vec![
                mainnet_keys::validator_1(),
                /*
                mainnet_keys::validator_2(),
                mainnet_keys::validator_3(),
                mainnet_keys::validator_4(),
                mainnet_keys::validator_5(),
                */
            ],
            mainnet_keys::prefunded(),
            SS58Prefix::get() as u64,
        ))
        .build()
}

// TODO: add technical committee
fn mainnet_genesis(
    sudo_key: AccountId,
    // initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId, ValidatorId, AssignmentId, AuthorityDiscoveryId, BeefyId)>,
    validators_keys: Vec<mainnet_keys::ValidatorKeys>,
    initial_balances: impl IntoIterator<Item = (AccountId, Balance)>,
    chain_id: u64,
) -> serde_json::Value {
    const VALIDATOR_INITIAL_BALANCE: Balance = 75_000 * DOLLARS;
    const STASH_INITIAL_BALANCE: Balance = 25_000 * DOLLARS;

    let mut initial_balances =
        std::collections::BTreeMap::<AccountId, Balance>::from_iter(initial_balances);

    for keys in &validators_keys {
        initial_balances.insert(keys.id, VALIDATOR_INITIAL_BALANCE);
    }

    let stakers = validators_keys
        .iter()
        .map(|keys| {
            (keys.id, keys.stash, STASH_INITIAL_BALANCE, StakerStatus::<AccountId>::Validator)
        })
        .collect::<Vec<_>>();

    serde_json::json!({
        "sudo": {
            "key": Some(sudo_key),
        },
        "balances": {
            "balances": initial_balances.into_iter().collect::<Vec<_>>(),
        },
        "babe": {
            "epochConfig": Some(BABE_GENESIS_EPOCH_CONFIG),
        },
        "session": {
            "keys": validators_keys
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
        "staking": {
            "validatorCount": validators_keys.len() as u32,
            "minimumValidatorCount": validators_keys.len() as u32,
            "invulnerables": validators_keys.iter().map(|x| x.id).collect::<Vec<_>>(),
            "slashRewardFraction": Perbill::from_percent(5),
            // TODO: verify
            "stakers": stakers,
            "minValidatorBond": 75_000 * DOLLARS,
            "minNominatorBond": 1_000 * DOLLARS,
        },
        "nominationPools": {
            "minCreateBond": 100 * DOLLARS,
            "minJoinBond": 100 * DOLLARS,
        },
        "elections": {
            "members": validators_keys
                .iter()
                .take((validators_keys.len() + 1) / 2)
                .cloned()
                .map(|member| (member.id, STASH_INITIAL_BALANCE))
                .collect::<Vec<_>>(),
        },
        "technicalCommittee": {
            "members": validators_keys
                .iter()
                .take((validators_keys.len() + 1) / 2)
                .cloned()
                .map(|keys| keys.id)
                .collect::<Vec<_>>(),
        },
        "evmChainId": {
            "chainId": chain_id,
        },
    })
}

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
    sudo_key: AccountId,
    mut endowed_accounts: Vec<AccountId>,
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        BabeId,
        GrandpaId,
        ImOnlineId,
        ValidatorId,
        AssignmentId,
        AuthorityDiscoveryId,
        BeefyId,
    )>,
    initial_nominators: Vec<AccountId>,
    chain_id: u64,
) -> serde_json::Value {
    // endow all authorities and nominators.
    initial_authorities
        .iter()
        .map(|x| &x.0)
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
        .map(|x| (x.0, x.1, STASH, StakerStatus::Validator))
        .chain(initial_nominators.iter().map(|x| {
            use rand::{seq::SliceRandom, Rng};
            let limit = (MaxNominations::get() as usize).min(initial_authorities.len());
            let count = rng.gen::<usize>() % limit;
            let nominations = initial_authorities
                .as_slice()
                .choose_multiple(&mut rng, count)
                .map(|choice| choice.0)
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

    serde_json::json!({
        "sudo": {
            "key": Some(sudo_key),
        },
        "balances": {
            "balances": endowed_accounts.iter().cloned().map(|k| (k, ENDOWMENT)).collect::<Vec<_>>(),
        },
        "babe": {
            "epochConfig": Some(BABE_GENESIS_EPOCH_CONFIG),
        },
        "session": {
            "keys": initial_authorities
                .iter()
                .map(|x| (x.1, x.0, session_keys(x.2.clone(), x.3.clone(), x.4.clone(), x.5.clone(), x.6.clone(), x.7.clone(), x.8.clone())))
                .collect::<Vec<_>>(),
        },
        "staking": {
            "validatorCount": initial_authorities.len() as u32,
            "minimumValidatorCount": initial_authorities.len() as u32,
            "invulnerables": initial_authorities.iter().map(|x| x.0).collect::<Vec<_>>(),
            "slashRewardFraction": Perbill::from_percent(10),
            "stakers": stakers.clone(),
            "minValidatorBond": 75_000 * DOLLARS,
            "minNominatorBond": 10 * DOLLARS,
        },
        "elections": {
            "members": endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .map(|member| (member, STASH))
                .collect::<Vec<_>>(),
        },
        "technicalCommittee": {
            "members": endowed_accounts
                .iter()
                .take((num_endowed_accounts + 1) / 2)
                .cloned()
                .collect::<Vec<_>>(),
        },
        "evmChainId": { "chainId": chain_id },
        "evm": {
            "accounts": evm_accounts,
        },
        "nominationPools": {
            "minCreateBond": 10 * DOLLARS,
            "minJoinBond": DOLLARS,
        },
    })
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

    pub(super) fn diego_session_keys() -> (
        AccountId,
        AccountId,
        BabeId,
        GrandpaId,
        ImOnlineId,
        ValidatorId,
        AssignmentId,
        AuthorityDiscoveryId,
        BeefyId,
    ) {
        (
            AccountId::from(hex!("FFa4645462F429E8FB9a6534E22f9f4f75094aB4")), // stash
            diego(),
            sp_core::sr25519::Public::from_raw(hex!(
                "562cd8c70c00ec3a3a031f5c9885978dd03a3a4fdb27bcf126887a9da11ff405"
            ))
            .into(),
            sp_core::ed25519::Public::from_raw(hex!(
                "ffe39c882d4ec6800a7501e1ccf3193b1f4d789d599d37f03db7f92bffb26471"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "76bb986cb29126a2d7848317cd1dcbdbdd743bf69c0daf673674dbed19b70e4d"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "928b6cc65c0af10060c041ab2cf2a7acd5ee5cfe983d33df47b4569513601119"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "54f81f0289afd8d99f9ec60efeb6541bbc953d60d1743f0e41d0ec5f4e1c8b54"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "4c6feb3c3f7c547a4630e181853d885498c55d72df686d7ab8e5d9854fbdeb7e"
            ))
            .into(),
            sp_core::ecdsa::Public::from_raw(hex!(
                "02e5967d94fbffa084b5c818adf500e0ef85d1aaf125e8bf90d3ca3d85b4ccd9f9"
            ))
            .into(),
        )
    }

    pub(super) fn pele_session_keys() -> (
        AccountId,
        AccountId,
        BabeId,
        GrandpaId,
        ImOnlineId,
        ValidatorId,
        AssignmentId,
        AuthorityDiscoveryId,
        BeefyId,
    ) {
        (
            AccountId::from(hex!("55DE108cb01Acf946A0ddE3C40D5EdE3AE9201C1")), // stash
            pele(),
            sp_core::sr25519::Public::from_raw(hex!(
                "84bb180709195c3f12bc22e16fb971a0369ebd45b6b8334f6f03d50aa986c213"
            ))
            .into(),
            sp_core::ed25519::Public::from_raw(hex!(
                "16ec13de87e30ee2eb9be5874558a9a82a39d2707d3ab67670c5e94bb64646ac"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "30a332f8874e0f7a66770917b27aba5fc5ca25f81c31332baaf5f1e897e4b404"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "801c1833f11bacf7e886ba0f638ea5f94a55f4d0e25ed7e055a6b05392982173"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "b2e8ded1345c31db85751ba36053f6e0ae03a53474118ed2ffc771c702e2b536"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "387681fa78ded7783f7bf0cd05d249b9b624125392c2ec0766602dde1f02e454"
            ))
            .into(),
            sp_core::ecdsa::Public::from_raw(hex!(
                "03f077b84a3d6fc7032a797cc8f068e43c0358c0181234cc7309159e57056a11e3"
            ))
            .into(),
        )
    }

    pub(super) fn franz_session_keys() -> (
        AccountId,
        AccountId,
        BabeId,
        GrandpaId,
        ImOnlineId,
        ValidatorId,
        AssignmentId,
        AuthorityDiscoveryId,
        BeefyId,
    ) {
        (
            AccountId::from(hex!("F87EfACD0e08cF7F6667B2a8BEc9fC3a2DB1572F")), // stash
            franz(),
            sp_core::sr25519::Public::from_raw(hex!(
                "22c09973a99e38bcf899411fac369257bd8971eddc167e718b1a9014279a2415"
            ))
            .into(),
            sp_core::ed25519::Public::from_raw(hex!(
                "e73dc222fb879f67add8aeedf30156a47fd8740d02432e3db5d4ebe8c78f1b87"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "44bea1479765faa200b5ee7b37ac00795891ff97fd629c2676c481bbb6e27f61"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "b8dd60e50c7ca1b47feb4faa58f1f2741fff68c527c297fc70ba72b91436cd71"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "98fbdacb195db4f1a31238687e0e2e2c3311a3c6f00fef708a645630ce97f716"
            ))
            .into(),
            sp_core::sr25519::Public::from_raw(hex!(
                "60a678a7410322ad55e77064d595ed81e1c4a024fd81adbc3d56e4e4f841781c"
            ))
            .into(),
            sp_core::ecdsa::Public::from_raw(hex!(
                "0200e692bbc231d1521133b3aa20f09b2dfbeca8682fd2ad23fa437fe51e49daab"
            ))
            .into(),
        )
    }
}

#[rustfmt::skip]
mod mainnet_keys {
    use super::*;

    pub fn sudo_account() -> AccountId {
        AccountId::from(hex!("a9a55e9de3c8d70c9f1107b58e33070fa816335c"))
    }

    pub fn prefunded() -> Vec<(AccountId, Balance)> {
        vec![
            (sudo_account(), 3_000_000_000_000 * DOLLARS),
        ]
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

    pub fn validator_1() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("f09513bBf1D425528269F93Fc2fBc307994e1443")),
            stash: AccountId::from(hex!("8a1D46E9352F2EC83Ed3d003A3279B7c80F870f8")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("447025a919267b34e074eee48cc1bd04f185b833cfe262cb7ca44a9a1f39fd24")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("af3f0b366ccd9a7f1f009d20bd6fa3ffdec315c4e49d24eeb9a356b01f190caf")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("d47e92d313adc8b46693e3b0fe84e3d7c8d55445d9b3f28289e188f82e06491c")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("3a5b6abfdfb49830ea593110f36e3beb89fd428d248422eaeeeb205c5db99b0d")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("ecd157484fd9b20cade0dad081b882d823cd4931c5ad0914f151c5292078af4b")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("64a34afd6712b5ec27726c709d36d857dc9f127661873a7ad66116f195419a14")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("0242ac90e719c9b8f1a7b36d14bee73bfa55bf60bbaac9a323624f256649093803")).into(),
        }
    }



    /*
    pub(super) fn validator() -> Keys {
        Keys {
            id:    AccountId::from(hex!("")),
            stash: AccountId::from(hex!("")),
            babe:                sp_core::sr25519::Public::from_raw(hex!("")).into(),
            grandpa:             sp_core::sr25519::Public::from_raw(hex!("")).into(),
            im_online:           sp_core::sr25519::Public::from_raw(hex!("")).into(),
            assignment:          sp_core::sr25519::Public::from_raw(hex!("")).into(),
            authority_discovery: sp_core::sr25519::Public::from_raw(hex!("")).into(),
            beefy:               sp_core::sr25519::Public::from_raw(hex!("")).into(),
        }
    }
    */
}

fn session_keys(
    babe: BabeId,
    grandpa: GrandpaId,
    im_online: ImOnlineId,
    para_validator: ValidatorId,
    para_assignment: AssignmentId,
    authority_discovery: AuthorityDiscoveryId,
    beefy: BeefyId,
) -> SessionKeys {
    SessionKeys {
        babe,
        grandpa,
        im_online,
        para_validator,
        para_assignment,
        authority_discovery,
        beefy,
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
pub fn authority_keys_from_seed(
    s: &str,
) -> (
    AccountId,
    AccountId,
    BabeId,
    GrandpaId,
    ImOnlineId,
    ValidatorId,
    AssignmentId,
    AuthorityDiscoveryId,
    BeefyId,
) {
    (
        get_account_id_from_seed::<ecdsa::Public>(&format!("{}//stash", s)),
        get_account_id_from_seed::<ecdsa::Public>(s),
        get_from_seed::<BabeId>(s),
        get_from_seed::<GrandpaId>(s),
        get_from_seed::<ImOnlineId>(s),
        get_from_seed::<ValidatorId>(s),
        get_from_seed::<AssignmentId>(s),
        get_from_seed::<AuthorityDiscoveryId>(s),
        get_from_seed::<BeefyId>(s),
    )
}

// Chain properties
fn properties() -> Properties {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "ATLA".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), SS58Prefix::get().into());
    properties
}
