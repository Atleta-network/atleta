use crate as pallet_faucet;
use frame_support::parameter_types;
use frame_support::traits::{ConstU16, ConstU32, ConstU64, Everything};
use frame_system::pallet_prelude::*;
use sp_core::H256;
use sp_runtime::{
    traits::{BlakeTwo256, IdentityLookup},
    BuildStorage,
};

type Block = frame_system::mocking::MockBlock<Test>;
pub(crate) type AccountId = u32;
pub(crate) type Balance = u32;
type Nonce = u32;

// minutes * seconds / 6 seconds per block
pub const BLOCKS_PER_HOUR: BlockNumberFor<Test> = 60 * 60 / 6;

frame_support::construct_runtime!(
    pub enum Test
    {
        System: frame_system,
        Balances: pallet_balances,
        Faucet: pallet_faucet,
    }
);

impl frame_system::Config for Test {
    type BaseCallFilter = Everything;
    type BlockWeights = ();
    type BlockLength = ();
    type DbWeight = ();
    type RuntimeOrigin = RuntimeOrigin;
    type RuntimeCall = RuntimeCall;
    type Nonce = Nonce;
    type Hash = H256;
    type Hashing = BlakeTwo256;
    type AccountId = AccountId;
    type Lookup = IdentityLookup<Self::AccountId>;
    type Block = Block;
    type RuntimeEvent = RuntimeEvent;
    type BlockHashCount = ConstU64<250>;
    type Version = ();
    type PalletInfo = PalletInfo;
    type AccountData = pallet_balances::AccountData<Balance>;
    type OnNewAccount = ();
    type OnKilledAccount = ();
    type SystemWeightInfo = ();
    type SS58Prefix = ConstU16<42>;
    type OnSetCode = ();
    type MaxConsumers = ConstU32<16>;
    type RuntimeTask = ();
}

impl pallet_balances::Config for Test {
    type MaxLocks = ();
    type MaxReserves = ();
    type ReserveIdentifier = [u8; 8];
    type Balance = Balance;
    type RuntimeEvent = RuntimeEvent;
    type DustRemoval = ();
    type ExistentialDeposit = ConstU32<1>;
    type AccountStore = System;
    type WeightInfo = ();
    type FreezeIdentifier = ();
    type MaxFreezes = ();
    type MaxHolds = ();
    type RuntimeHoldReason = ();
    type RuntimeFreezeReason = ();
}

parameter_types! {
    pub const AccumulationPeriod: BlockNumberFor<Test> = BLOCKS_PER_HOUR * 24;
    pub const FaucetAmount: Balance = 1000;
}

impl pallet_faucet::Config for Test {
    type AccumulationPeriod = AccumulationPeriod;
    type FaucetAmount = FaucetAmount;
    type RuntimeEvent = RuntimeEvent;
    type WeightInfo = ();
}

pub const GENESIS_ACCOUNT: AccountId = 5;
pub const GENESIS_ACCOUNT_BALANCE: u32 = 1_000_000_000;

pub struct ExtBuilder {
    balances: Vec<(AccountId, u32)>,
    genesis_account: Option<AccountId>,
}

impl Default for ExtBuilder {
    fn default() -> Self {
        Self {
            balances: [(GENESIS_ACCOUNT, GENESIS_ACCOUNT_BALANCE)].to_vec(),
            genesis_account: Some(GENESIS_ACCOUNT),
        }
    }
}

impl ExtBuilder {
    pub fn build(self) -> sp_io::TestExternalities {
        let mut storage =
            frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into();

        let _ = pallet_balances::GenesisConfig::<Test> { balances: self.balances }
            .assimilate_storage(&mut storage);

        let _ = pallet_faucet::GenesisConfig::<Test> { genesis_account: self.genesis_account }
            .assimilate_storage(&mut storage);

        let ext = sp_io::TestExternalities::from(storage);

        ext
    }

    pub fn build_and_execute(self, test: impl FnOnce() -> ()) {
        let mut ext = self.build();
        ext.execute_with(test);
        ext.execute_with(|| System::set_block_number(1));
    }
}
