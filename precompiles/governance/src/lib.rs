#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    traits::{Bounded, BoundedInline, Currency, IsType},
};
use pallet_evm::{AddressMapping, PrecompileFailure};
use precompile_utils::prelude::*;
use sp_core::{H160, H256, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_std::marker::PhantomData;

type BalanceOf<Runtime> = <<Runtime as pallet_democracy::Config>::Currency as Currency<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

pub struct GovernanceFlowPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> GovernanceFlowPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_democracy::Config,
    Runtime::AccountId: Into<H160>,
    Runtime::Hash: IsType<H256>,
    BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
    Runtime::Lookup: StaticLookup<Source = Runtime::AccountId>,
    Runtime::RuntimeCall: From<pallet_democracy::Call<Runtime>>,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
{
    #[precompile::public("propose(uint8[],uint256)")]
    fn propose_inline(
        h: &mut impl PrecompileHandle,
        bounded_call: Vec<u8>,
        value: U256,
    ) -> EvmResult<()> {
        let bounded_call = BoundedInline::try_from(bounded_call)
            .map_err(|_| Self::custom_err("Unable to parse bounded call"))?;
        let value = Self::u256_to_amount(value)?;
        Self::_propose(h, Bounded::Inline(bounded_call), value)
    }

    #[precompile::public("propose(bytes32,uint256)")]
    fn propose_lookup(
        h: &mut impl PrecompileHandle,
        proposal_hash: H256,
        value: U256,
    ) -> EvmResult<()> {
        let hash = proposal_hash;
        let len = hash.0.len() as u32;
        let value = Self::u256_to_amount(value)?;
        Self::_propose(h, Bounded::Lookup { hash: hash.into(), len }, value)
    }

    fn _propose(
        h: &mut impl PrecompileHandle,
        proposal: pallet_democracy::BoundedCallOf<Runtime>,
        value: BalanceOf<Runtime>,
    ) -> EvmResult<()> {
        let call = pallet_democracy::Call::<Runtime>::propose { proposal, value };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    fn u256_to_amount(value: U256) -> MayRevert<BalanceOf<Runtime>> {
        value
            .try_into()
            .map_err(|_| RevertReason::value_is_too_large("amount type").into())
    }

    fn custom_err(reason: &'static str) -> PrecompileFailure {
        PrecompileFailure::Error { exit_status: evm::ExitError::Other(reason.into()) }
    }
}
