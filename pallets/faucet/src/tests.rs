use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok};

#[test]
fn faucet_works() {
    ExtBuilder::default().build_and_execute(|| {
        let balance = 100;
        let receiver = 1;
        assert_eq!(Balances::free_balance(&receiver), 0);
        assert_ok!(Faucet::request_funds(RuntimeOrigin::signed(1), balance));
        assert_eq!(Balances::free_balance(&receiver), balance);
        assert_eq!(Balances::free_balance(&GENESIS_ACCOUNT), GENESIS_ACCOUNT_BALANCE - balance);
    })
}

#[test]
fn faucet_fail_send_more_than_max() {
    ExtBuilder::default().build_and_execute(|| {
        let balance = 1000 + 5;
        assert_noop!(
            Faucet::request_funds(RuntimeOrigin::signed(1), balance),
            Error::<Test>::AmountTooHigh
        );
    });
}

#[test]
fn faucet_fail_exceed_max_amount_during_period() {
    ExtBuilder::default().build_and_execute(|| {
        assert_ok!(Faucet::request_funds(RuntimeOrigin::signed(1), 100));
        assert_eq!(Balances::free_balance(1), 100);

        System::set_block_number(BLOCKS_PER_HOUR * 7);

        assert_ok!(Faucet::request_funds(RuntimeOrigin::signed(1), 200));
        assert_eq!(Balances::free_balance(1), 300);

        System::set_block_number(BLOCKS_PER_HOUR * 20);

        assert_ok!(Faucet::request_funds(RuntimeOrigin::signed(1), 500));
        assert_eq!(Balances::free_balance(1), 800);

        System::set_block_number(BLOCKS_PER_HOUR * 23);

        assert_noop!(
            Faucet::request_funds(RuntimeOrigin::signed(1), 210),
            Error::<Test>::RequestLimitExceeded
        );

        System::set_block_number(BLOCKS_PER_HOUR * 24 - 1);

        assert_noop!(
            Faucet::request_funds(RuntimeOrigin::signed(1), 210),
            Error::<Test>::RequestLimitExceeded
        );

        System::set_block_number(BLOCKS_PER_HOUR * 24);

        assert_ok!(Faucet::request_funds(RuntimeOrigin::signed(1), 210));
        assert_eq!(Balances::free_balance(1), 1010);
    });
}
