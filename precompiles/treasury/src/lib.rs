#![cfg_attr(not(feature = "std"), no_std)]
#![allow(missing_docs)]

use fp_evm::PrecompileHandle;
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    traits::{Currency, IsType},
};
use pallet_evm::{AddressMapping, PrecompileFailure};
use precompile_utils::prelude::*;
use sp_core::{H160, H256, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_std::{marker::PhantomData, vec::Vec};

type BalanceOf<Runtime> = <<Runtime as pallet_treasury::Config>::Currency as Currency<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

pub struct TreasuryPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> TreasuryPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_treasury::Config,
    Runtime::AccountId: Into<H160>,
    Runtime::Hash: IsType<H256>,
    BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
    Runtime::Lookup: StaticLookup<Source = Runtime::AccountId>,
    Runtime::RuntimeCall: From<pallet_treasury::Call<Runtime>>,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
{
    #[precompile::public("proposeSpend(uint256,address)")]
    fn propose_spend(
        h: &mut impl PrecompileHandle,
        value: U256,
        beneficiary: Address,
    ) -> EvmResult<()> {
        let value =
            value.try_into().map_err(|_| RevertReason::value_is_too_large("amount type"))?;
        let beneficiary =
            Runtime::Lookup::lookup(Runtime::AddressMapping::into_account_id(beneficiary.0))
                .map_err(|_| Self::custom_err("Unable to lookup address"))?;

        let call = pallet_treasury::Call::<Runtime>::propose_spend { value, beneficiary };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("proposalCount()")]
    #[precompile::view]
    fn proposal_count(handle: &mut impl PrecompileHandle) -> EvmResult<u32> {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

        Ok(pallet_treasury::Pallet::<Runtime>::proposal_count())
    }

    #[precompile::public("approvals()")]
    #[precompile::view]
    fn approvals(handle: &mut impl PrecompileHandle) -> EvmResult<Vec<u32>> {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

        let approvals: Vec<u32> =
            pallet_treasury::Approvals::<Runtime>::get().into_iter().collect();
        Ok(approvals)
    }

    fn custom_err(reason: &'static str) -> PrecompileFailure {
        PrecompileFailure::Error { exit_status: evm::ExitError::Other(reason.into()) }
    }
}
