// TODO: use substrate benchmark cli

use core::marker::PhantomData;
use frame_support::weights::Weight;

pub trait WeightInfo {
    fn signed_call() -> Weight;
}

pub struct MetamaskWeight<T>(PhantomData<T>);

impl<T: frame_system::Config> WeightInfo for MetamaskWeight<T> {
    fn signed_call() -> Weight {
        Weight::zero()
    }
}

impl WeightInfo for () {
    fn signed_call() -> Weight {
        Weight::zero()
    }
}
