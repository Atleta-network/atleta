#![cfg_attr(not(feature = "std"), no_std)]
#![allow(missing_docs)]

use fp_evm::PrecompileHandle;
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    traits::fungible::Inspect,
};
use pallet_evm::{AddressMapping, PrecompileFailure};
use pallet_nomination_pools::BondExtra;
use precompile_utils::prelude::*;
use sp_core::{Decode, H160, U256};
use sp_runtime::{
    traits::{Dispatchable, StaticLookup},
    Perbill,
};
use sp_std::{marker::PhantomData, vec::Vec};

pub struct NominationPoolsPrecompile<Runtime>(PhantomData<Runtime>);

type BalanceOf<Runtime> = <<Runtime as pallet_nomination_pools::Config>::Currency as Inspect<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

#[precompile_utils::precompile]
impl<Runtime> NominationPoolsPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_nomination_pools::Config,
    Runtime::AccountId: Into<H160>,
    BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
    Runtime::Lookup: StaticLookup<Source = Runtime::AccountId>,
    Runtime::RuntimeCall: From<pallet_nomination_pools::Call<Runtime>>,
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
    #[allow(clippy::type_complexity)]
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

    #[precompile::public("create(uint256,address,address,address)")]
    fn create(
        h: &mut impl PrecompileHandle,
        amount: U256,
        root: Address,
        nominator: Address,
        bouncer: Address,
    ) -> EvmResult<()> {
        let amount = Self::u256_to_amount(amount)?;
        let root = Runtime::Lookup::lookup(Runtime::AddressMapping::into_account_id(root.0))
            .map_err(|_| Self::custom_err("Unable to lookup root address"))?;
        let nominator =
            Runtime::Lookup::lookup(Runtime::AddressMapping::into_account_id(nominator.0))
                .map_err(|_| Self::custom_err("Unable to lookup nominator address"))?;
        let bouncer = Runtime::Lookup::lookup(Runtime::AddressMapping::into_account_id(bouncer.0))
            .map_err(|_| Self::custom_err("Unable to lookup bouncer address"))?;

        let call =
            pallet_nomination_pools::Call::<Runtime>::create { amount, root, nominator, bouncer };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("setMetadata(uint32,uint8[])")]
    fn set_metadata(
        h: &mut impl PrecompileHandle,
        pool_id: u32,
        metadata: Vec<u8>,
    ) -> EvmResult<()> {
        let call = pallet_nomination_pools::Call::<Runtime>::set_metadata { pool_id, metadata };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("setCommission(uint32,uint32,address)")]
    fn set_commission(
        h: &mut impl PrecompileHandle,
        pool_id: u32,
        commission: u32,
        payee: Address,
    ) -> EvmResult<()> {
        let commission = Perbill::from_percent(commission);
        let payee = Runtime::AddressMapping::into_account_id(payee.0);

        let call = pallet_nomination_pools::Call::<Runtime>::set_commission {
            pool_id,
            new_commission: Some((commission, payee)),
        };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("setCommission(uint32)")]
    fn set_commission_none(h: &mut impl PrecompileHandle, pool_id: u32) -> EvmResult<()> {
        let call = pallet_nomination_pools::Call::<Runtime>::set_commission {
            pool_id,
            new_commission: None,
        };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("chill(uint32)")]
    fn chill(h: &mut impl PrecompileHandle, pool_id: u32) -> EvmResult<()> {
        let call = pallet_nomination_pools::Call::<Runtime>::chill { pool_id };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("setState(uint32,uint8)")]
    fn set_state(h: &mut impl PrecompileHandle, pool_id: u32, pool_state: u8) -> EvmResult<()> {
        let state = pallet_nomination_pools::PoolState::decode(&mut &[pool_state][..])
            .map_err(|_| Self::custom_err("Unable to decode PoolState variant"))?;

        let call = pallet_nomination_pools::Call::<Runtime>::set_state { pool_id, state };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("setClaimPermission(uint8)")]
    fn set_claim_permission(h: &mut impl PrecompileHandle, permission: u8) -> EvmResult<()> {
        let permission =
            pallet_nomination_pools::ClaimPermission::decode(&mut &[permission][..])
                .map_err(|_| Self::custom_err("Unable to decode ClaimPermission variant"))?;

        let call = pallet_nomination_pools::Call::<Runtime>::set_claim_permission { permission };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("updateRoles(uint32,uint8[])")]
    fn update_roles(
        h: &mut impl PrecompileHandle,
        pool_id: u32,
        encoded_roles: Vec<u8>,
    ) -> EvmResult<()> {
        use pallet_nomination_pools::ConfigOp;

        fn conv<T, U>(op: ConfigOp<T>, f: impl FnOnce(T) -> U) -> ConfigOp<U>
        where
            T: sp_std::fmt::Debug + parity_scale_codec::Codec,
            U: sp_std::fmt::Debug + parity_scale_codec::Codec,
        {
            match op {
                ConfigOp::Noop => ConfigOp::Noop,
                ConfigOp::Set(val) => ConfigOp::Set(f(val)),
                ConfigOp::Remove => ConfigOp::Remove,
            }
        }

        let (new_root, new_nominator, new_bouncer) = <(
            ConfigOp<[u8; 20]>,
            ConfigOp<[u8; 20]>,
            ConfigOp<[u8; 20]>,
        )>::decode(&mut &encoded_roles[..])
        .map_err(|_| Self::custom_err("Unable to decode Roles custom tuple"))
        .map(|(new_root, new_nominator, new_bouncer)| {
            let new_root =
                conv(new_root, |bytes| Runtime::AddressMapping::into_account_id(bytes.into()));
            let new_nominator =
                conv(new_nominator, |bytes| Runtime::AddressMapping::into_account_id(bytes.into()));
            let new_bouncer =
                conv(new_bouncer, |bytes| Runtime::AddressMapping::into_account_id(bytes.into()));

            (new_root, new_nominator, new_bouncer)
        })?;

        let call = pallet_nomination_pools::Call::<Runtime>::update_roles {
            pool_id,
            new_root,
            new_nominator,
            new_bouncer,
        };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("claimCommission(uint32)")]
    fn claim_commission(h: &mut impl PrecompileHandle, pool_id: u32) -> EvmResult<()> {
        let call = pallet_nomination_pools::Call::<Runtime>::claim_commission { pool_id };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("setCommissionChangeRate(uint32,uint32,uint32)")]
    fn set_commission_change_rate(
        h: &mut impl PrecompileHandle,
        pool_id: u32,
        max_increase: u32,
        min_delay: u32,
    ) -> EvmResult<()> {
        let max_increase = Perbill::from_percent(max_increase);

        let call = pallet_nomination_pools::Call::<Runtime>::set_commission_change_rate {
            pool_id,
            change_rate: pallet_nomination_pools::CommissionChangeRate {
                max_increase,
                min_delay: min_delay.into(),
            },
        };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("setCommissionMax(uint32,uint32)")]
    fn set_commission_max(
        h: &mut impl PrecompileHandle,
        pool_id: u32,
        max_commission: u32,
    ) -> EvmResult<()> {
        let max_commission = Perbill::from_percent(max_commission);

        let call = pallet_nomination_pools::Call::<Runtime>::set_commission_max {
            pool_id,
            max_commission,
        };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("setCommissionClaimPermissionNone(uint32)")]
    fn set_commission_claim_permission_none(
        h: &mut impl PrecompileHandle,
        pool_id: u32,
    ) -> EvmResult<()> {
        let call = pallet_nomination_pools::Call::<Runtime>::set_commission_claim_permission {
            pool_id,
            permission: None,
        };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("setCommissionClaimPermissionless(uint32)")]
    fn set_commission_claim_permissionless(
        h: &mut impl PrecompileHandle,
        pool_id: u32,
    ) -> EvmResult<()> {
        let call = pallet_nomination_pools::Call::<Runtime>::set_commission_claim_permission {
            pool_id,
            permission: Some(pallet_nomination_pools::CommissionClaimPermission::Permissionless),
        };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("setCommissionClaimPermissionAddress(uint32,address)")]
    fn set_commission_claim_permission_address(
        h: &mut impl PrecompileHandle,
        pool_id: u32,
        account: Address,
    ) -> EvmResult<()> {
        let account = Runtime::AddressMapping::into_account_id(account.0);

        let call = pallet_nomination_pools::Call::<Runtime>::set_commission_claim_permission {
            pool_id,
            permission: Some(pallet_nomination_pools::CommissionClaimPermission::Account(account)),
        };
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
