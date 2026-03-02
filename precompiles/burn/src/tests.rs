use frame_support::traits::fungible::{Inspect, Mutate};
use frame_support::traits::tokens::{Fortitude, Precision};
use frame_support::weights::Weight;
use pallet_evm::Runner;
use sp_core::{keccak_256, H160, H256, U256};

use crate::mock::*;

struct PrecompileTesterExt;

impl PrecompileTesterExt {
    const GAS_LIMIT: u64 = 1_000_000;

    fn selector(signature: &[u8]) -> [u8; 4] {
        let hash = keccak_256(signature);
        [hash[0], hash[1], hash[2], hash[3]]
    }

    fn encode_burn(amount: U256) -> Vec<u8> {
        let mut calldata = Vec::with_capacity(36);
        calldata.extend_from_slice(&Self::selector(b"burn(uint256)"));
        let mut encoded_amount = [0u8; 32];
        amount.to_big_endian(&mut encoded_amount);
        calldata.extend_from_slice(&encoded_amount);
        calldata
    }

    fn encode_total_issuance() -> Vec<u8> {
        Self::selector(b"totalIssuance()").to_vec()
    }

    fn call(caller: H160, input: Vec<u8>) -> pallet_evm::CallInfo {
        let config = <Runtime as pallet_evm::Config>::config().clone();

        <Runtime as pallet_evm::Config>::Runner::call(
            caller,
            PRECOMPILE_ADDRESS,
            input,
            U256::zero(),
            Self::GAS_LIMIT,
            None,
            None,
            None,
            Vec::new(),
            false,
            true,
            Some(Weight::MAX),
            Some(0),
            &config,
        )
        .expect("EVM call should execute")
    }

    fn succeeded(reason: &evm::ExitReason) -> bool {
        matches!(reason, evm::ExitReason::Succeed(_))
    }

    fn decode_u256(output: &[u8]) -> U256 {
        assert_eq!(output.len(), 32);
        U256::from_big_endian(output)
    }

    fn topic_for_address(address: H160) -> H256 {
        let mut topic = [0u8; 32];
        topic[12..].copy_from_slice(address.as_bytes());
        H256(topic)
    }
}

fn total_issuance() -> Balance {
    <pallet_balances::Pallet<Runtime> as Inspect<AccountId>>::total_issuance()
}

fn free_balance(who: &AccountId) -> Balance {
    <pallet_balances::Pallet<Runtime> as Inspect<AccountId>>::balance(who)
}

#[test]
fn burn_reduces_balance_and_total_issuance() {
    ExtBuilder::default().build().execute_with(|| {
        let burn_amount: Balance = 1_000_000_000_000_000_000_000;
        let initial_issuance = total_issuance();
        let initial_balance = free_balance(&ALICE);

        <pallet_balances::Pallet<Runtime> as Mutate<AccountId>>::burn_from(
            &ALICE,
            burn_amount,
            Precision::Exact,
            Fortitude::Polite,
        )
        .expect("burn should succeed");

        assert_eq!(free_balance(&ALICE), initial_balance - burn_amount);
        assert_eq!(total_issuance(), initial_issuance - burn_amount);
    });
}

#[test]
fn burn_full_balance_zeroes_account() {
    ExtBuilder::default().build().execute_with(|| {
        let full_balance = free_balance(&ALICE);
        let initial_issuance = total_issuance();

        <pallet_balances::Pallet<Runtime> as Mutate<AccountId>>::burn_from(
            &ALICE,
            full_balance,
            Precision::Exact,
            Fortitude::Polite,
        )
        .expect("burn should succeed");

        assert_eq!(free_balance(&ALICE), 0);
        assert_eq!(total_issuance(), initial_issuance - full_balance);
    });
}

#[test]
fn cannot_burn_more_than_available_balance() {
    ExtBuilder::default().build().execute_with(|| {
        let too_much = free_balance(&ALICE) + 1;

        let result = <pallet_balances::Pallet<Runtime> as Mutate<AccountId>>::burn_from(
            &ALICE,
            too_much,
            Precision::Exact,
            Fortitude::Polite,
        );

        assert!(result.is_err());
    });
}

#[test]
fn burn_does_not_affect_other_accounts() {
    ExtBuilder::default().build().execute_with(|| {
        let burn_amount: Balance = 500_000_000_000_000_000_000;
        let bob_balance_before = free_balance(&BOB);

        <pallet_balances::Pallet<Runtime> as Mutate<AccountId>>::burn_from(
            &ALICE,
            burn_amount,
            Precision::Exact,
            Fortitude::Polite,
        )
        .expect("burn should succeed");

        assert_eq!(free_balance(&BOB), bob_balance_before);
    });
}

#[test]
fn total_issuance_reflects_all_balances() {
    ExtBuilder::default().build().execute_with(|| {
        assert_eq!(total_issuance(), INITIAL_BALANCE * 2);
    });
}

#[test]
fn u256_to_u128_overflow_is_caught() {
    let overflow_value = U256::from(u128::MAX) + U256::from(1u64);
    let result: Result<u128, _> = overflow_value.try_into();
    assert!(result.is_err());
}

#[test]
fn burn_zero_is_noop() {
    ExtBuilder::default().build().execute_with(|| {
        let initial_issuance = total_issuance();
        let initial_balance = free_balance(&ALICE);

        let result = <pallet_balances::Pallet<Runtime> as Mutate<AccountId>>::burn_from(
            &ALICE,
            0,
            Precision::Exact,
            Fortitude::Polite,
        );

        assert!(result.is_ok());
        assert_eq!(free_balance(&ALICE), initial_balance);
        assert_eq!(total_issuance(), initial_issuance);
    });
}

#[test]
fn burn_from_nonexistent_account_fails() {
    ExtBuilder::default().build().execute_with(|| {
        let unknown = H160([0xCC; 20]);

        let result = <pallet_balances::Pallet<Runtime> as Mutate<AccountId>>::burn_from(
            &unknown,
            1_000_000_000_000_000_000,
            Precision::Exact,
            Fortitude::Polite,
        );

        assert!(result.is_err());
    });
}

#[test]
fn selector_log_burned_is_correct() {
    use sp_core::keccak_256;
    let expected = keccak_256(b"Burned(address,uint256)");
    assert_eq!(crate::SELECTOR_LOG_BURNED, expected);
}

#[test]
fn test_burn_via_precompile() {
    ExtBuilder::default().build().execute_with(|| {
        let burn_amount: Balance = 1_000_000_000_000_000_000_000;
        let initial_balance = free_balance(&ALICE);
        let initial_issuance = total_issuance();

        let result = PrecompileTesterExt::call(
            ALICE,
            PrecompileTesterExt::encode_burn(U256::from(burn_amount)),
        );

        assert!(PrecompileTesterExt::succeeded(&result.exit_reason));
        assert_eq!(result.value.len(), 32);
        assert_eq!(result.value[31], 1);
        assert_eq!(free_balance(&ALICE), initial_balance - burn_amount);
        assert_eq!(total_issuance(), initial_issuance - burn_amount);
    });
}

#[test]
fn test_total_issuance_via_precompile() {
    ExtBuilder::default().build().execute_with(|| {
        let result = PrecompileTesterExt::call(ALICE, PrecompileTesterExt::encode_total_issuance());

        assert!(PrecompileTesterExt::succeeded(&result.exit_reason));
        assert_eq!(PrecompileTesterExt::decode_u256(&result.value), U256::from(total_issuance()));
    });
}

#[test]
fn test_burn_emits_log() {
    ExtBuilder::default().build().execute_with(|| {
        let burn_amount: Balance = 500_000_000_000_000_000_000;

        let result = PrecompileTesterExt::call(
            ALICE,
            PrecompileTesterExt::encode_burn(U256::from(burn_amount)),
        );

        assert!(PrecompileTesterExt::succeeded(&result.exit_reason));
        assert_eq!(result.logs.len(), 1);

        let log = &result.logs[0];
        let mut encoded_amount = [0u8; 32];
        U256::from(burn_amount).to_big_endian(&mut encoded_amount);

        assert_eq!(log.address, PRECOMPILE_ADDRESS);
        assert_eq!(log.topics.len(), 2);
        assert_eq!(log.topics[0], H256::from(crate::SELECTOR_LOG_BURNED));
        assert_eq!(log.topics[1], PrecompileTesterExt::topic_for_address(ALICE));
        assert_eq!(log.data, encoded_amount.to_vec());
    });
}

#[test]
fn test_burn_insufficient_balance_reverts() {
    ExtBuilder::default().build().execute_with(|| {
        let initial_balance = free_balance(&ALICE);
        let initial_issuance = total_issuance();
        let too_much = U256::from(initial_balance) + U256::one();

        let result = PrecompileTesterExt::call(ALICE, PrecompileTesterExt::encode_burn(too_much));

        assert!(!PrecompileTesterExt::succeeded(&result.exit_reason));
        assert_eq!(free_balance(&ALICE), initial_balance);
        assert_eq!(total_issuance(), initial_issuance);
    });
}
