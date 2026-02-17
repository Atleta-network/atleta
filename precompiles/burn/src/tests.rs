use frame_support::traits::fungible::{Inspect, Mutate};
use frame_support::traits::tokens::{Fortitude, Precision};
use sp_core::{H160, U256};

use crate::mock::*;

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
