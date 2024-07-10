#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use pallet_evm::PrecompileFailure;
use precompile_utils::prelude::*;
use sp_core::H160;
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_std::marker::PhantomData;

pub struct StakingPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> StakingPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_staking::Config,
    Runtime::AccountId: Into<H160>,
    Runtime::Lookup: StaticLookup<Source = Runtime::AccountId>,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
{
    #[precompile::public("activeEra()")]
    #[precompile::view]
    fn active_era(_: &mut impl PrecompileHandle) -> EvmResult<u32> {
        // TODO: record gas
        let era_info = pallet_staking::Pallet::<Runtime>::active_era()
            .ok_or_else(|| Self::custom_err("Unable to get active era"))?;
        Ok(era_info.index)
    }

    fn custom_err(reason: &'static str) -> PrecompileFailure {
        PrecompileFailure::Error { exit_status: evm::ExitError::Other(reason.into()) }
    }
}
