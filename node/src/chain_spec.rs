use std::{collections::BTreeMap, str::FromStr};

// 3rd party imports
use hex_literal::hex;

// Substrate
use sc_chain_spec::{ChainType, Properties};
use sp_consensus_babe::AuthorityId as BabeId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
#[allow(unused_imports)]
use sp_core::ecdsa;
use sp_core::{Pair, Public, H160, U256};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

// Frontier
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sportchain_runtime::{
    constants::currency::*, opaque::SessionKeys, AccountId, Balance, MaxNominations,
    RuntimeGenesisConfig, SS58Prefix, Signature, StakerStatus, BABE_GENESIS_EPOCH_CONFIG,
    WASM_BINARY,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig>;

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
        .with_name("Testnet")
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

/// Configure initial storage state for FRAME modules.
fn testnet_genesis(
    sudo_key: AccountId,
    mut endowed_accounts: Vec<AccountId>,
    initial_authorities: Vec<(AccountId, AccountId, BabeId, GrandpaId, ImOnlineId)>,
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
                .map(|x| (x.1, x.0, session_keys(x.2.clone(), x.3.clone(), x.4.clone())))
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
            "maxNominatorCount": 16,
        },
        "evmChainId": { "chainId": chain_id },
        "evm": {
            "accounts": evm_accounts,
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

    pub(super) fn diego_session_keys() -> (AccountId, AccountId, BabeId, GrandpaId, ImOnlineId) {
        (
            AccountId::from(hex!("FFa4645462F429E8FB9a6534E22f9f4f75094aB4")), // stash
            diego(),
            sp_core::sr25519::Public(hex!(
                "562cd8c70c00ec3a3a031f5c9885978dd03a3a4fdb27bcf126887a9da11ff405"
            ))
            .into(),
            sp_core::ed25519::Public(hex!(
                "ffe39c882d4ec6800a7501e1ccf3193b1f4d789d599d37f03db7f92bffb26471"
            ))
            .into(),
            sp_core::sr25519::Public(hex!(
                "76bb986cb29126a2d7848317cd1dcbdbdd743bf69c0daf673674dbed19b70e4d"
            ))
            .into(),
        )
    }

    pub(super) fn pele_session_keys() -> (AccountId, AccountId, BabeId, GrandpaId, ImOnlineId) {
        (
            AccountId::from(hex!("55DE108cb01Acf946A0ddE3C40D5EdE3AE9201C1")), // stash
            pele(),
            sp_core::sr25519::Public(hex!(
                "84bb180709195c3f12bc22e16fb971a0369ebd45b6b8334f6f03d50aa986c213"
            ))
            .into(),
            sp_core::ed25519::Public(hex!(
                "16ec13de87e30ee2eb9be5874558a9a82a39d2707d3ab67670c5e94bb64646ac"
            ))
            .into(),
            sp_core::sr25519::Public(hex!(
                "30a332f8874e0f7a66770917b27aba5fc5ca25f81c31332baaf5f1e897e4b404"
            ))
            .into(),
        )
    }

    pub(super) fn franz_session_keys() -> (AccountId, AccountId, BabeId, GrandpaId, ImOnlineId) {
        (
            AccountId::from(hex!("F87EfACD0e08cF7F6667B2a8BEc9fC3a2DB1572F")), // stash
            franz(),
            sp_core::sr25519::Public(hex!(
                "22c09973a99e38bcf899411fac369257bd8971eddc167e718b1a9014279a2415"
            ))
            .into(),
            sp_core::ed25519::Public(hex!(
                "e73dc222fb879f67add8aeedf30156a47fd8740d02432e3db5d4ebe8c78f1b87"
            ))
            .into(),
            sp_core::sr25519::Public(hex!(
                "44bea1479765faa200b5ee7b37ac00795891ff97fd629c2676c481bbb6e27f61"
            ))
            .into(),
        )
    }
}

fn session_keys(babe: BabeId, grandpa: GrandpaId, im_online: ImOnlineId) -> SessionKeys {
    SessionKeys { babe, grandpa, im_online }
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
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AccountId, BabeId, GrandpaId, ImOnlineId) {
    (
        get_account_id_from_seed::<ecdsa::Public>(&format!("{}//stash", s)),
        get_account_id_from_seed::<ecdsa::Public>(s),
        get_from_seed::<BabeId>(s),
        get_from_seed::<GrandpaId>(s),
        get_from_seed::<ImOnlineId>(s),
    )
}

// Chain properties
fn properties() -> Properties {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "BCS".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), SS58Prefix::get().into());
    properties
}
