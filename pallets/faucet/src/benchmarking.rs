//! Benchmarking setup for pallet-faucet.
#![cfg(feature = "runtime-benchmarks")]
use super::*;

#[allow(unused)]
use crate::Pallet as Faucet;
use frame_benchmarking::v2::*;
use frame_system::RawOrigin;

#[benchmarks]
mod benchmarks {
    use super::*;

    #[benchmark]
    fn request_funds() {
        let amount = 100u32.into();
        let caller: T::AccountId = whitelisted_caller();
        #[extrinsic_call]
        request_funds(RawOrigin::Signed(caller.clone()), amount);

        assert!(Requests::<T>::contains_key(&caller));
    }

    impl_benchmark_test_suite!(Faucet, crate::mock::new_test_ext(), crate::mock::Test);
}
