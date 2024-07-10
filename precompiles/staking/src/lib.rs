#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use frame_support::dispatch::{GetDispatchInfo, PostDispatchInfo};
use pallet_evm::{AddressMapping, PrecompileFailure};
use precompile_utils::prelude::*;
use sp_core::{Get, H160, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_std::{marker::PhantomData, vec::Vec};

pub struct StakingPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> StakingPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_staking::Config,
    Runtime::AccountId: Into<H160>,
    <Runtime as pallet_staking::Config>::CurrencyBalance: Into<U256>,
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

    #[precompile::public("sessionsPerEra()")]
    #[precompile::view]
    fn sessions_per_era(_: &mut impl PrecompileHandle) -> EvmResult<u32> {
        Ok(<Runtime as pallet_staking::Config>::SessionsPerEra::get())
    }

    #[precompile::public("slashingSpans(address)")]
    fn slashing_spans(
        _: &mut impl PrecompileHandle,
        address: Address,
    ) -> EvmResult<(u32, u32, u32, Vec<u32>)> {
        let addr = Runtime::AddressMapping::into_account_id(address.0);
        let pallet_staking::slashing::SlashingSpans { .. } =
            pallet_staking::SlashingSpans::<Runtime>::get(addr)
                .ok_or_else(|| Self::custom_err("Unable to get slashing spans"))?;
        // XXX: SlashingSpans fields are private
        unimplemented!()
    }

    #[precompile::public("erasTotalStake(uint32)")]
    #[precompile::view]
    fn eras_total_stake(_: &mut impl PrecompileHandle, era: u32) -> EvmResult<U256> {
        let total = pallet_staking::ErasTotalStake::<Runtime>::get(era);
        Ok(total.into())
    }

    #[precompile::public("erasValidatorReward(uint32)")]
    fn eras_validator_reward(_: &mut impl PrecompileHandle, era: u32) -> EvmResult<U256> {
        let reward = pallet_staking::ErasValidatorReward::<Runtime>::get(era)
            .ok_or_else(|| Self::custom_err("Unable to get eras validator reward"))?;
        Ok(reward.into())
    }

    fn custom_err(reason: &'static str) -> PrecompileFailure {
        PrecompileFailure::Error { exit_status: evm::ExitError::Other(reason.into()) }
    }
}
