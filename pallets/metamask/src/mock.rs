#![allow(unused_imports, dead_code)]

use super::*;
use crate as metamask;
use frame_support::derive_impl;

#[frame_support::pallet(dev_mode)]
pub mod logger {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config {
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(Weight::from(42_000))]
        pub fn log(origin: OriginFor<T>, data: Vec<u8>) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            Self::deposit_event(Event::Log { signed_by: sender, data });
            Ok(())
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Log { signed_by: T::AccountId, data: Vec<u8> },
    }
}

frame_support::construct_runtime!(
    pub enum TestRuntime {
        System: frame_system,
        EVMChainId: pallet_evm_chain_id,
        Logger: logger,
        Metamask: metamask,
    }
);

type Block = frame_system::mocking::MockBlock<TestRuntime>;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig as frame_system::pallet::DefaultConfig)]
impl frame_system::Config for TestRuntime {
    type Block = Block;
    type AccountId = sp_core::H160;
    type Lookup = sp_runtime::traits::IdentityLookup<sp_core::H160>;
}

impl pallet_evm_chain_id::Config for TestRuntime {}

impl logger::Config for TestRuntime {
    type RuntimeEvent = RuntimeEvent;
}

impl Config for TestRuntime {
    type Sender = <Self as frame_system::Config>::AccountId;
    type Nonce = <Self as frame_system::Config>::Nonce;

    type RuntimeEvent = RuntimeEvent;
    type RuntimeCall = RuntimeCall;
    type WeightInfo = ();
}

pub type MetamaskCall = metamask::Call<TestRuntime>;
pub type LoggerCall = logger::Call<TestRuntime>;

pub fn new_test_ext() -> sp_io::TestExternalities {
    // let t = frame_system::GenesisConfig::<TestRuntime>::default().build_storage().unwrap();
    let mut ext = sp_io::TestExternalities::new_empty();
    ext.execute_with(|| System::set_block_number(1));
    ext
}
