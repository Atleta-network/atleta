use crate::{mock::*, Error};
use frame_support::{assert_noop, assert_ok, traits::Currency};

#[test]
fn faucet_works_with_initial_balance() {
    ExtBuilder::default().with_initial_balance(200).build_and_execute(|| {
        let balance = 200;
        let receiver = 1;

        let faucet_account = Faucet::account_id();
        assert_eq!(Balances::free_balance(faucet_account), 200);

        assert_eq!(Balances::free_balance(receiver), 0);
        assert_ok!(Faucet::request_funds(RuntimeOrigin::none(), receiver, balance));
        assert_eq!(Balances::free_balance(receiver), balance);

        assert_eq!(Balances::free_balance(faucet_account), 0);
    });
}

#[test]
fn faucet_fails_without_initial_balance() {
    ExtBuilder::default().with_initial_balance(0).build_and_execute(|| {
        let receiver = 1;

        let min_balance = Balances::minimum_balance();

        let faucet_account = Faucet::account_id();
        assert_eq!(Balances::free_balance(faucet_account), min_balance);

        let request_amount = 200;
        assert_noop!(
            Faucet::request_funds(RuntimeOrigin::none(), receiver, request_amount),
            Error::<Test>::NoAmountToTransfer
        );

        assert_eq!(Balances::free_balance(faucet_account), min_balance);
    });
}

#[test]
fn refill_faucet_works() {
    ExtBuilder::default().with_initial_balance(0).build_and_execute(|| {
        let faucet_account = Faucet::account_id();
        let sender = 1;
        let refill_amount = 500;
        let min_balance = Balances::minimum_balance();

        assert_ok!(Balances::force_set_balance(RuntimeOrigin::root(), sender, refill_amount));
        let sender_initial_balance = Balances::free_balance(sender);
        assert!(sender_initial_balance >= refill_amount);

        assert_eq!(Balances::free_balance(faucet_account), Balances::minimum_balance());
        assert_ok!(Faucet::refill_faucet(
            RuntimeOrigin::signed(sender),
            refill_amount - min_balance
        ));

        assert_eq!(Balances::free_balance(faucet_account), refill_amount);

        assert_eq!(Balances::free_balance(sender), min_balance);
    });
}

#[test]
fn request_fails_with_excess_in_period() {
    ExtBuilder::default().with_initial_balance(1000).build_and_execute(|| {
        let receiver = 1;

        assert_ok!(Faucet::request_funds(RuntimeOrigin::none(), receiver, 600));
        assert_eq!(Balances::free_balance(receiver), 600);

        assert_noop!(
            Faucet::request_funds(RuntimeOrigin::none(), receiver, 500),
            Error::<Test>::RequestLimitExceeded
        );
    });
}

#[test]
fn faucet_fails_with_insufficient_faucet_balance() {
    ExtBuilder::default().with_initial_balance(300).build_and_execute(|| {
        let receiver = 1;
        let request_amount = 500;

        assert_noop!(
            Faucet::request_funds(RuntimeOrigin::none(), receiver, request_amount),
            Error::<Test>::NoAmountToTransfer
        );

        let faucet_account = Faucet::account_id();
        assert_eq!(Balances::free_balance(faucet_account), 300);
    });
}

#[test]
fn request_works_after_accumulation_period() {
    ExtBuilder::default().with_initial_balance(1000).build_and_execute(|| {
        let receiver = 1;

        assert_ok!(Faucet::request_funds(RuntimeOrigin::none(), receiver, 500));
        assert_eq!(Balances::free_balance(receiver), 500);

        System::set_block_number(BLOCKS_PER_HOUR * 24);

        assert_ok!(Faucet::request_funds(RuntimeOrigin::none(), receiver, 500));
        assert_eq!(Balances::free_balance(receiver), 1000);
    });
}
