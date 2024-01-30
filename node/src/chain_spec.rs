use std::{collections::BTreeMap, str::FromStr};

// 3rd party imports
use hex_literal::hex;
use serde::{Deserialize, Serialize};
// Substrate
use sc_chain_spec::{ChainType, Properties};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
#[allow(unused_imports)]
use sp_core::ecdsa;
use sp_core::{storage::Storage, Pair, Public, H160, U256};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};
use sp_state_machine::BasicExternalities;
// Frontier
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use sportchain_runtime::{
    constants::currency::*, opaque::SessionKeys, AccountId, Balance, EnableManualSeal,
    MaxNominations, RuntimeGenesisConfig, SS58Prefix, Signature, StakerStatus, WASM_BINARY,
};

// The URL for the telemetry server.
// const STAGING_TELEMETRY_URL: &str = "wss://telemetry.polkadot.io/submit/";

/// Specialized `ChainSpec`. This is a specialization of the general Substrate ChainSpec type.
pub type ChainSpec = sc_service::GenericChainSpec<RuntimeGenesisConfig>;

/// Specialized `ChainSpec` for development.
pub type DevChainSpec = sc_service::GenericChainSpec<DevGenesisExt>;

// Public accoint type
#[allow(dead_code)]
type AccountPublic = <Signature as Verify>::Signer;

/// Extension for the dev genesis config to support a custom changes to the genesis state.
#[derive(Serialize, Deserialize)]
pub struct DevGenesisExt {
    /// Genesis config.
    genesis_config: RuntimeGenesisConfig,
    /// The flag that if enable manual-seal mode.
    enable_manual_seal: Option<bool>,
}

// Impl storage builder
impl sp_runtime::BuildStorage for DevGenesisExt {
    fn assimilate_storage(&self, storage: &mut Storage) -> Result<(), String> {
        BasicExternalities::execute_with_storage(storage, || {
            if let Some(enable_manual_seal) = &self.enable_manual_seal {
                EnableManualSeal::set(enable_manual_seal);
            }
        });
        self.genesis_config.assimilate_storage(storage)
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
pub fn authority_keys_from_seed(s: &str) -> (AccountId, AccountId, AuraId, GrandpaId, ImOnlineId) {
    (
        get_account_id_from_seed::<ecdsa::Public>(&format!("{}//stash", s)),
        get_account_id_from_seed::<ecdsa::Public>(s),
        get_from_seed::<AuraId>(s),
        get_from_seed::<GrandpaId>(s),
        get_from_seed::<ImOnlineId>(s),
    )
}

// Session keys constructor.
fn session_keys(aura: AuraId, grandpa: GrandpaId, im_online: ImOnlineId) -> SessionKeys {
    SessionKeys { aura, grandpa, im_online }
}

// Chain properties
fn properties() -> Properties {
    let mut properties = Properties::new();
    properties.insert("tokenSymbol".into(), "BCS".into());
    properties.insert("tokenDecimals".into(), 18.into());
    properties.insert("ss58Format".into(), SS58Prefix::get().into());
    properties
}

// Dev chain config
pub fn development_config() -> DevChainSpec {
    DevChainSpec::builder(WASM_BINARY.expect("WASM not available"), Default::default())
        .with_name("Development")
        .with_id("dev")
        .with_chain_type(ChainType::Development)
        .with_properties(properties())
        .with_genesis_config_patch(testnet_genesis(
            // Sudo account (Alith)
            AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")),
            // Pre-funded accounts
            vec![
                AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")), // Alith
                AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0")), // Baltathar
                AccountId::from(hex!("798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc")), // Charleth
                AccountId::from(hex!("773539d4Ac0e786233D90A233654ccEE26a613D9")), // Dorothy
                AccountId::from(hex!("Ff64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB")), // Ethan
                AccountId::from(hex!("C0F0f4ab324C46e55D02D0033343B4Be8A55532d")), // Faith
            ],
            // Initial PoA authorities
            vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
            // Initial nominators
            vec![],
            // Ethereum chain ID
            SS58Prefix::get() as u64,
        ))
        .build()
}

// Local testnet config
pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::builder(WASM_BINARY.expect("WASM not available"), Default::default())
        .with_name("Local Testnet")
        .with_id("local_testnet")
        .with_chain_type(ChainType::Local)
        .with_properties(properties())
        .with_genesis_config_patch(testnet_genesis(
            // Initial PoA authorities
            // Sudo account (Alith)
            AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")),
            // Pre-funded accounts
            vec![
                AccountId::from(hex!("f24FF3a9CF04c71Dbc94D0b566f7A27B94566cac")), // Alith
                AccountId::from(hex!("3Cd0A705a2DC65e5b1E1205896BaA2be8A07c6e0")), // Baltathar
                AccountId::from(hex!("798d4Ba9baf0064Ec19eB4F0a1a45785ae9D6DFc")), // Charleth
                AccountId::from(hex!("773539d4Ac0e786233D90A233654ccEE26a613D9")), // Dorothy
                AccountId::from(hex!("Ff64d3F6efE2317EE2807d223a0Bdc4c0c49dfDB")), // Ethan
                AccountId::from(hex!("C0F0f4ab324C46e55D02D0033343B4Be8A55532d")), // Faith
            ],
            vec![authority_keys_from_seed("Alice"), authority_keys_from_seed("Bob")],
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
    initial_authorities: Vec<(AccountId, AccountId, AuraId, GrandpaId, ImOnlineId)>,
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
        "session": {
            "keys": initial_authorities
                .iter()
                .map(|x| (x.1.clone(), x.0.clone(), session_keys(x.2.clone(), x.3.clone(), x.4.clone())))
                .collect::<Vec<_>>(),
        },
        "staking": {
            "validatorCount": initial_authorities.len() as u32,
            "minimumValidatorCount": initial_authorities.len() as u32,
            "invulnerables": initial_authorities.iter().map(|x| x.0.clone()).collect::<Vec<_>>(),
            "slashRewardFraction": Perbill::from_percent(10),
            "stakers": stakers.clone(),
            "minValidatorBond": 75_000 * DOLLARS,
            "minNominatorBond": DOLLARS,
        },
        "evmChainId": { "chainId": chain_id },
        "evm": {
            "accounts": evm_accounts,
        },
    })
}
