#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use precompile_utils::prelude::*;
use sp_core::Get;
use sp_std::marker::PhantomData;

pub struct BabePrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> BabePrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_babe::Config,
    Runtime::Moment: Into<u64>,
{
    #[precompile::public("epochDuration()")]
    #[precompile::view]
    fn epoch_duration(_: &mut impl PrecompileHandle) -> EvmResult<u64> {
        Ok(<Runtime as pallet_babe::Config>::EpochDuration::get())
    }

    #[precompile::public("expectedBlockTime()")]
    #[precompile::view]
    fn expected_block_time(_: &mut impl PrecompileHandle) -> EvmResult<u64> {
        Ok(<Runtime as pallet_babe::Config>::ExpectedBlockTime::get().into())
    }
}
