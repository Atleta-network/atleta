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
        .with_chain_type(ChainType::Live)
        .with_properties(properties())
        .with_genesis_config_patch(mainnet_genesis(
            mainnet_keys::sudo_account(),
            vec![
                mainnet_keys::validator_1(),
                mainnet_keys::validator_2(),
                mainnet_keys::validator_3(),
                mainnet_keys::validator_4(),
                mainnet_keys::validator_5(),
                mainnet_keys::validator_6(),
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

    pub fn validator_2() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("a61370983C7347Abe42a7D022872424ed02AF26B")),
            stash: AccountId::from(hex!("8a1D46E9352F2EC83Ed3d003A3279B7c80F870f8")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("1a2ca43a6dc1b614bc4e5b4754557c63ce3877893b40fe17b19fd402ab987974")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("83da365751ca0672cc977c996822a7bfdd54f77be15007a16666549925a070fd")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("1c210753939ec575dfa05a8cb6efb2ae7f4feabf8924226aba457db8623b4c24")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("4e918ac48225dc3c50a52131c5b00c53b5cc81ea3ddefb73813a93b8bc4f9577")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("1a2e59e42707869bb4e9f0f8e4e41fc190641232a5f77abc7d6ce460c5d05827")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("d052b826eb7606d07f6e800c35d139303be9271ff8ab9d33f22567a1807a4852")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("026c1464395fc93996348a348564ec00c4bc0eebfc2b62f9366e3d4e373b620004")).into(),
        }
    }


    pub fn validator_3() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("eEd070f8F636A18fFaB79aFb9699920DFd00a6B6")),
            stash: AccountId::from(hex!("63BFE4945C504085F57b3794539c5a9916D9d509")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("7c933e26e8c2e83b06503e6a86a0ad1918d3663dd1f28cdee6c3ec2d17bf1d56")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("41774af0bf220dae6cc2ff7f4b0635745daa303ea7f5f24e668c8d2790c1ad78")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("9ef053a3fb6cc3822220d1a2b6ea99d2440fa04bbf7cef831966a8e9e042d00a")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("aad234e9e85cc5b9c3823e45e919b7b6f126276761e128a5aeb71831379a512e")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("f424ee603d2ef23df4a33b10cbf9892b029712527fc30d3db41cba3e190e0669")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("4c7493fce9a47fef2cb67cdadb31bbdfbcf4ad21469474d4304d13651b3b5168")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("02cbdcf7ea28228b139f1d6700206714174bd4e2a4b9b0eb38960c8a9eca232920")).into(),
        }
    }

    pub fn validator_4() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("55449792Db180DA77771327467A58e281894Ab49")),
            stash: AccountId::from(hex!("d9994f45076435B56716D44b00C3684ad3bC4b24")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("2892449f7a38a8f54c6d00eb91981d75a96d8529e93fc5f5d1edb85020177200")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("76f518209c68eadc0de0e11e68c351319e9c9c6d551128d16cb805911584977d")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("d65339cb4294718dbca5eba2829aa96e8091fc6d7d36023ba5f11ce039abc80c")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("804e1301c29d168abbe5e770e9d3027669d8b763bc6003c91871fe2256db2372")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("a62ac689f7e514ed8d644d5ba7862763a3e7e5f574e76c43d561f281acfb8c69")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("0a644b61b753db3314e25e74d0a148cb88d0e5cd12d9316bbf1e8d3d0028dc64")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("0398d5f7101c8b3436464b15962c3f6b62de8ede7740555f8fb97f01adbc3e1f1e")).into(),
        }
    }

    pub fn validator_5() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("A5D763a0f9f6BC96FD3758C2C2352BEc1Bf5f9F7")),
            stash: AccountId::from(hex!("345268DF74246cB3c42c6cEAa060044A7D7343b3")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("88fa7eaebdf4e6fd9d62e0cb1398284f682731e2b01f2eec5375aa79120a606d")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("f4381fc2f0b7c5be20f0750649d828e5e5b58d828a36a9198527526665905262")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("3c1304939273e0632c5ccd8e40fcec475e1721ab37796115cd85031dcbda636f")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("d43689d709ca63e97929122a5f899354417c479799e075946597da9be2f6f623")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("86ed1250347eca1cebcc2edc4cbea70714fc5ad4ef726c5a52333e28948a470a")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("f6edbe610ac965cf4cca7f49c32fc9a93fc5887fe2f44cd51d2f5895969f026b")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("0206d2c9d103c8842f116147d8f632c9861b24dfbb7738120d1ffe088ad7137897")).into(),
        }
    }

    pub fn validator_6() -> ValidatorKeys {
        ValidatorKeys {
            id:    AccountId::from(hex!("A64A0f5387de16466137bEa6e01a771612ECedD1")),
            stash: AccountId::from(hex!("6A73b2e30BC13FEbaAC92e140A1aE035Cec1307E")),
            babe:                     sp_core::sr25519::Public::from_raw(hex!("72bb9c5094bdbd2a46f1456ab3d8a7bb8b874978105e0e3df97eaa593864b261")).into(),
            grandpa:                  sp_core::ed25519::Public::from_raw(hex!("b6e51503f2f39da01f19e3420828950e01deed6a22c7c11b12e40f99b4b31dc3")).into(),
            im_online:                sp_core::sr25519::Public::from_raw(hex!("769a292652829ebf581bb4d06b73be5870399f3213a801b840a601da77238618")).into(),
            para_validator:           sp_core::sr25519::Public::from_raw(hex!("4c6f4afc4eb716064396ded3c4103d44220d17bee4b2cc466f9612fe94266633")).into(),
            para_assignment:          sp_core::sr25519::Public::from_raw(hex!("04a52921037ad8dc7c0cf473dab3a5638e1ec09fc95e8bed5ac4febd9c6cee0a")).into(),
            authority_discovery:      sp_core::sr25519::Public::from_raw(hex!("100a11504fef3fcf60cc5a465035ca829d31ffae2da4ebd10e9f1ff33cd9a743")).into(),
            beefy:                    sp_core::ecdsa::Public::from_raw(hex!("03b74d5602f46a58c065385a48bedb181cc06aae90a705d6cf6b17371414ab0bde")).into(),
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
