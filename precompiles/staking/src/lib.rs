#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    traits::{fungible::Inspect, Currency},
};
use pallet_evm::{AddressMapping, PrecompileFailure};
use pallet_nomination_pools::BondExtra;
use precompile_utils::prelude::*;
use sp_core::{Get, H160, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_std::{marker::PhantomData, vec::Vec};

pub struct StakingFlowPrecompile<Runtime>(PhantomData<Runtime>);

type BalanceOf<Runtime> = <<Runtime as pallet_nomination_pools::Config>::Currency as Inspect<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

type FaucetBalanceOf<Runtime> = <<Runtime as pallet_faucet::Config>::Currency as Currency<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

#[precompile_utils::precompile]
impl<Runtime> StakingFlowPrecompile<Runtime>
where
    Runtime: pallet_evm::Config
        + pallet_nomination_pools::Config
        + pallet_staking::Config
        + pallet_faucet::Config,
    Runtime::AccountId: Into<H160>,
    BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
    FaucetBalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
    <Runtime as pallet_faucet::Config>::Currency:
        frame_support::traits::fungible::Inspect<<Runtime as frame_system::Config>::AccountId>,
    Runtime::Lookup: StaticLookup<Source = Runtime::AccountId>,
    Runtime::RuntimeCall:
        From<pallet_nomination_pools::Call<Runtime>> + From<pallet_faucet::Call<Runtime>>,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
{
    #[precompile::public("joinPool(uint256,uint32)")]
    fn join_pool(h: &mut impl PrecompileHandle, amount: U256, pool_id: u32) -> EvmResult<()> {
        let amount = Self::u256_to_amount(amount)?;
        let call = pallet_nomination_pools::Call::<Runtime>::join { amount, pool_id };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("bondExtra(uint256)")]
    fn bond_extra(h: &mut impl PrecompileHandle, amount: U256) -> EvmResult<()> {
        let extra = if amount == U256::MAX {
            BondExtra::Rewards
        } else {
            let amount = Self::u256_to_amount(amount)?;
            BondExtra::FreeBalance(amount)
        };
        let call = pallet_nomination_pools::Call::<Runtime>::bond_extra { extra };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("claimPayout()")]
    fn claim_payout(h: &mut impl PrecompileHandle) -> EvmResult<()> {
        let call = pallet_nomination_pools::Call::<Runtime>::claim_payout {};
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("unbond(address,uint256)")]
    fn unbond(
        h: &mut impl PrecompileHandle,
        member_account: Address,
        unbonding_points: U256,
    ) -> EvmResult<()> {
        let member_account =
            Runtime::Lookup::lookup(Runtime::AddressMapping::into_account_id(member_account.0))
                .map_err(|_| Self::custom_err("Unable to lookup address"))?;
        let unbonding_points = Self::u256_to_amount(unbonding_points)?;

        let call =
            pallet_nomination_pools::Call::<Runtime>::unbond { member_account, unbonding_points };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("withdrawUnbonded(address,uint32)")]
    fn withdraw_unbonded(
        h: &mut impl PrecompileHandle,
        member_account: Address,
        num_slashing_spans: u32,
    ) -> EvmResult<()> {
        let member_account =
            Runtime::Lookup::lookup(Runtime::AddressMapping::into_account_id(member_account.0))
                .map_err(|_| Self::custom_err("Unable to lookup address"))?;

        let call = pallet_nomination_pools::Call::<Runtime>::withdraw_unbonded {
            member_account,
            num_slashing_spans,
        };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("pendingRewards(address)")]
    #[precompile::view]
    fn pending_rewards(_: &mut impl PrecompileHandle, who: Address) -> EvmResult<U256> {
        // TODO: record gas
        let who = Runtime::AddressMapping::into_account_id(who.0);
        let amount = pallet_nomination_pools::Pallet::<Runtime>::api_pending_rewards(who)
            .map(<_>::into)
            .unwrap_or_else(U256::zero);
        Ok(amount)
    }

    #[precompile::public("activeEra()")]
    #[precompile::view]
    fn active_era(_: &mut impl PrecompileHandle) -> EvmResult<u32> {
        // TODO: record gas
        let era_info = pallet_staking::Pallet::<Runtime>::active_era()
            .ok_or_else(|| Self::custom_err("Unable to get active era"))?;
        Ok(era_info.index)
    }

    #[precompile::public("bondedPools(uint32)")]
    #[precompile::view]
    // TODO: return value is to be discussed
    fn bonded_pools(_: &mut impl PrecompileHandle, pool_id: u32) -> EvmResult<(u32, U256)> {
        // TODO: record gas
        let bonded_pool = pallet_nomination_pools::BondedPool::<Runtime>::get(pool_id)
            .ok_or_else(|| Self::custom_err("Unable to get bonded pool"))?;
        let pallet_nomination_pools::BondedPoolInner { member_counter, points, .. } = *bonded_pool;
        Ok((member_counter, points.into()))
    }

    #[precompile::public("poolMembers(address)")]
    #[precompile::view]
    fn pool_members(
        _: &mut impl PrecompileHandle,
        address: Address,
    ) -> EvmResult<(u32, U256, Vec<(u32, U256)>)> {
        let address = Runtime::AddressMapping::into_account_id(address.0);
        let pallet_nomination_pools::PoolMember { pool_id, points, unbonding_eras, .. } =
            pallet_nomination_pools::PoolMembers::<Runtime>::get(address)
                .ok_or_else(|| Self::custom_err("Unable to get pool members"))?;

        Ok((
            pool_id,
            points.into(),
            unbonding_eras
                .into_iter()
                .map(|(era, points)| (era, points.into()))
                .collect::<Vec<_>>(),
        ))
    }

    #[precompile::public("requestFunds(address,uint256)")]
    fn request_funds(h: &mut impl PrecompileHandle, who: Address, amount: U256) -> EvmResult<()> {
        let who = Runtime::AddressMapping::into_account_id(who.0);
        let amount =
            amount.try_into().map_err(|_| RevertReason::value_is_too_large("amount type"))?;

        let call = pallet_faucet::Call::<Runtime>::request_funds { who, amount };
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
