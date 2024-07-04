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
use sp_std::{marker::PhantomData, vec::Vec};

type BalanceOf<Runtime> = <<Runtime as pallet_democracy::Config>::Currency as Currency<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

type TreasuryBalanceOf<Runtime> = <<Runtime as pallet_treasury::Config>::Currency as Currency<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

pub struct GovernanceFlowPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> GovernanceFlowPrecompile<Runtime>
where
    Runtime: pallet_evm::Config
        + pallet_democracy::Config
        + pallet_treasury::Config
        + pallet_preimage::Config,
    Runtime::AccountId: Into<H160>,
    Runtime::Hash: IsType<H256>,
    BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
    TreasuryBalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
    Runtime::Lookup: StaticLookup<Source = Runtime::AccountId>,
    Runtime::RuntimeCall: From<pallet_democracy::Call<Runtime>>
        + From<pallet_treasury::Call<Runtime>>
        + From<pallet_preimage::Call<Runtime>>,
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

    #[precompile::public("vote(uint32,bool,uint8,uint256)")]
    fn vote_standard(
        h: &mut impl PrecompileHandle,
        ref_index: u32,
        aye: bool,
        conviction: u8,
        balance: U256,
    ) -> EvmResult<()> {
        let conviction = pallet_democracy::Conviction::try_from(conviction)
            .map_err(|_| Self::custom_err("Unable to parse conviction"))?;
        let vote = pallet_democracy::Vote { aye, conviction };
        let balance = Self::u256_to_amount(balance)?;
        let vote = pallet_democracy::AccountVote::Standard { vote, balance };
        Self::_vote(h, ref_index, vote)
    }

    #[precompile::public("vote(uint32,uint256,uint256)")]
    fn vote_split(
        h: &mut impl PrecompileHandle,
        ref_index: u32,
        aye: U256,
        nay: U256,
    ) -> EvmResult<()> {
        let aye = Self::u256_to_amount(aye)?;
        let nay = Self::u256_to_amount(nay)?;
        let vote = pallet_democracy::AccountVote::Split { aye, nay };
        Self::_vote(h, ref_index, vote)
    }

    fn _vote(
        h: &mut impl PrecompileHandle,
        ref_index: pallet_democracy::ReferendumIndex,
        vote: pallet_democracy::AccountVote<BalanceOf<Runtime>>,
    ) -> EvmResult<()> {
        let call = pallet_democracy::Call::<Runtime>::vote { ref_index, vote };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("removeVote(uint32)")]
    fn remove_vote(h: &mut impl PrecompileHandle, index: u32) -> EvmResult<()> {
        let call = pallet_democracy::Call::<Runtime>::remove_vote { index };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

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

    #[precompile::public("notePreimage(uint8[])")]
    fn note_preimage(h: &mut impl PrecompileHandle, bytes: Vec<u8>) -> EvmResult<()> {
        let call = pallet_preimage::Call::<Runtime>::note_preimage { bytes };
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
