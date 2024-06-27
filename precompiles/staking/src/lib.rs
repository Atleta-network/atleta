#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    traits::Currency,
    BoundedVec,
};
use pallet_evm::{AddressMapping, PrecompileFailure};
use pallet_staking::{RewardDestination, ValidatorPrefs};
use precompile_utils::prelude::*;
use sp_core::{Get, H160, U256};
use sp_runtime::{
    traits::{Dispatchable, StaticLookup},
    Perbill, Percent,
};
use sp_staking::{EraIndex, Page};
use sp_std::{convert::TryInto, marker::PhantomData, vec::Vec};

mod solidity_types;

type BalanceOf<Runtime> = <<Runtime as pallet_staking::Config>::Currency as Currency<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

pub struct StakingPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> StakingPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_staking::Config,
    Runtime::RuntimeCall: From<pallet_staking::Call<Runtime>>,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    Runtime::Lookup: StaticLookup<Source = Runtime::AccountId>,
    Runtime::AccountId: Into<H160>,
    BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
{
    #[precompile::public("bond(uint256,uint8,address)")]
    fn bond(
        handle: &mut impl PrecompileHandle,
        amount: U256,
        reward_destination: solidity_types::RewardDestinationKind,
        payee: Address,
    ) -> EvmResult<()> {
        let amount = Self::u256_to_amount(amount).in_field("amount")?;
        let payee = Runtime::AddressMapping::into_account_id(payee.0);

        let reward_destination = reward_destination.conv_with(payee.clone());

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call =
            pallet_staking::Call::<Runtime>::bond { value: amount, payee: reward_destination };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;

        Ok(())
    }

    #[precompile::public("bondExtra(uint256)")]
    fn bond_extra(handle: &mut impl PrecompileHandle, max_additional: U256) -> EvmResult<()> {
        let max_additional = Self::u256_to_amount(max_additional).in_field("max_additional")?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::bond_extra { max_additional };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("unbond(uint256)")]
    fn unbond(handle: &mut impl PrecompileHandle, value: U256) -> EvmResult<()> {
        let value = Self::u256_to_amount(value).in_field("value")?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::unbond { value };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("withdrawUnbonded(uint32)")]
    fn withdraw_unbonded(
        handle: &mut impl PrecompileHandle,
        num_slashing_spans: u32,
    ) -> EvmResult<()> {
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::withdraw_unbonded { num_slashing_spans };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("validate(uint32,bool)")]
    fn validate(
        handle: &mut impl PrecompileHandle,
        commission_perc: u32,
        blocked: bool,
    ) -> EvmResult<()> {
        let prefs = ValidatorPrefs { commission: Perbill::from_percent(commission_perc), blocked };

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::validate { prefs };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("nominate(address[])")]
    fn nominate(handle: &mut impl PrecompileHandle, targets: Vec<Address>) -> EvmResult<()> {
        let targets = targets
            .into_iter()
            .map(|addr| Runtime::AddressMapping::into_account_id(addr.0))
            .collect::<Vec<_>>();

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::nominate { targets };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("chill()")]
    fn chill(handle: &mut impl PrecompileHandle) -> EvmResult<()> {
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::chill {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("setPayee(uint8,address)")]
    fn set_payee(
        handle: &mut impl PrecompileHandle,
        reward_destination: solidity_types::RewardDestinationKind,
        payee: Address,
    ) -> EvmResult<()> {
        let payee = Runtime::AddressMapping::into_account_id(payee.0);
        let reward_destination = reward_destination.conv_with(payee.clone());

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::set_payee { payee: reward_destination };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("setController()")]
    fn set_controller(handle: &mut impl PrecompileHandle) -> EvmResult<()> {
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::set_controller {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("setValidatorCount(uint32)")]
    fn set_validator_count(handle: &mut impl PrecompileHandle, new: u32) -> EvmResult<()> {
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::set_validator_count { new };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("increaseValidatorCount(uint32)")]
    fn increase_validator_count(
        handle: &mut impl PrecompileHandle,
        additional: u32,
    ) -> EvmResult<()> {
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::increase_validator_count { additional };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("scaleValidatorCount(uint8)")]
    fn scale_validator_count(handle: &mut impl PrecompileHandle, factor: u8) -> EvmResult<()> {
        let factor = Percent::from_percent(factor);

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::scale_validator_count { factor };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("forceNoEras()")]
    fn force_no_eras(handle: &mut impl PrecompileHandle) -> EvmResult<()> {
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::force_no_eras {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("forceNewEra()")]
    fn force_new_era(handle: &mut impl PrecompileHandle) -> EvmResult<()> {
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::force_new_era {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("setInvulnerables(address[])")]
    fn set_invulnerables(
        handle: &mut impl PrecompileHandle,
        invulnerables: Vec<Address>,
    ) -> EvmResult<()> {
        let invulnerables = invulnerables
            .into_iter()
            .map(|addr| Runtime::AddressMapping::into_account_id(addr.0))
            .collect::<Vec<_>>();

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::set_invulnerables { invulnerables };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("forceUnstake(address,uint32)")]
    fn force_unstake(
        handle: &mut impl PrecompileHandle,
        stash: Address,
        num_slashing_spans: u32,
    ) -> EvmResult<()> {
        let stash = Runtime::AddressMapping::into_account_id(stash.0);

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::force_unstake { stash, num_slashing_spans };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("forceNewEraAlways()")]
    fn force_new_era_always(handle: &mut impl PrecompileHandle) -> EvmResult<()> {
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::force_new_era_always {};
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("cancelDeferredSlash(uint32,uint32[])")]
    fn cancel_deferred_slash(
        handle: &mut impl PrecompileHandle,
        era: EraIndex,
        slash_indices: Vec<u32>,
    ) -> EvmResult<()> {
        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::cancel_deferred_slash { era, slash_indices };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("payoutStakers(address,uint32)")]
    fn payout_stakers(
        handle: &mut impl PrecompileHandle,
        validator_stash: Address,
        era: EraIndex,
    ) -> EvmResult<()> {
        let validator_stash = Runtime::AddressMapping::into_account_id(validator_stash.0);

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::payout_stakers { validator_stash, era };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("rebond(uint256)")]
    fn rebond(handle: &mut impl PrecompileHandle, value: U256) -> EvmResult<()> {
        let value = Self::u256_to_amount(value)?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::rebond { value };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("reapStash(address,uint32)")]
    fn reap_stash(
        handle: &mut impl PrecompileHandle,
        stash: Address,
        num_slashing_spans: u32,
    ) -> EvmResult<()> {
        let stash = Runtime::AddressMapping::into_account_id(stash.0);

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::reap_stash { stash, num_slashing_spans };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("kick(address[])")]
    fn kick(handle: &mut impl PrecompileHandle, who: Vec<Address>) -> EvmResult<()> {
        let who = who
            .into_iter()
            .map(|addr| Runtime::AddressMapping::into_account_id(addr.0))
            .map(Runtime::Lookup::lookup)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| PrecompileFailure::Error {
                exit_status: evm::ExitError::Other("Unable to lookup some address".into()),
            })?;

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::kick { who };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    // NOTE
    // `set_staking_configs` call is TODO if we need to support all root origin calls
    // It has a massive signature and I'm not sure if we should even make this available to EVM

    #[precompile::public("chillOther(address)")]
    fn chill_other(handle: &mut impl PrecompileHandle, stash: Address) -> EvmResult<()> {
        let stash = Runtime::AddressMapping::into_account_id(stash.0);

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::chill_other { stash };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("forceApplyMinCommission(address)")]
    fn force_apply_min_commission(
        handle: &mut impl PrecompileHandle,
        validator_stash: Address,
    ) -> EvmResult<()> {
        let validator_stash = Runtime::AddressMapping::into_account_id(validator_stash.0);

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::force_apply_min_commission { validator_stash };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("setMinCommission(uint32)")]
    fn set_min_commission(handle: &mut impl PrecompileHandle, new: u32) -> EvmResult<()> {
        let new = Perbill::from_percent(new);

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::set_min_commission { new };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("payoutStakersByPage(address,uint32,uint32)")]
    fn payout_stakers_by_page(
        handle: &mut impl PrecompileHandle,
        validator_stash: Address,
        era: EraIndex,
        page: Page,
    ) -> EvmResult<()> {
        let validator_stash = Runtime::AddressMapping::into_account_id(validator_stash.0);

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call =
            pallet_staking::Call::<Runtime>::payout_stakers_by_page { validator_stash, era, page };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("updatePayee(address)")]
    fn update_payee(handle: &mut impl PrecompileHandle, controller: Address) -> EvmResult<()> {
        let controller = Runtime::AddressMapping::into_account_id(controller.0);

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::update_payee { controller };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    #[precompile::public("deprecateControllerBatch(address[])")]
    fn deprecate_controller_batch(
        handle: &mut impl PrecompileHandle,
        controllers: Vec<Address>,
    ) -> EvmResult<()> {
        let controllers = {
            let mut addrs = BoundedVec::new(); // Size is elided from call type
            for addr in controllers {
                let addr = Runtime::AddressMapping::into_account_id(addr.0);
                addrs.try_push(addr).map_err(|_| PrecompileFailure::Error {
                    exit_status: evm::ExitError::Other(
                        "Controllers size exceed allowed batch size".into(),
                    ),
                })?;
            }
            addrs
        };

        let origin = Runtime::AddressMapping::into_account_id(handle.context().caller);
        let call = pallet_staking::Call::<Runtime>::deprecate_controller_batch { controllers };
        RuntimeHelper::<Runtime>::try_dispatch(handle, Some(origin).into(), call)?;
        Ok(())
    }

    // NOTE:
    // optional argments, empty or single element array is expected
    // except last one which is `Option<Vec<_>>` ~ `Vec<[]`
    #[precompile::public("restoreLedger(address,address[],amount[],uint256[])")]
    fn restore_ledger(
        handle: &mut impl PrecompileHandle,
        stash: Address,
        maybe_controller: Vec<Address>,
        maybe_total: Vec<U256>,
        maybe_unlocking: Vec<U256>,
    ) -> EvmResult<()> {
        let stash = Runtime::AddressMapping::into_account_id(stash.0);
        let maybe_controller = Self::try_vec_to_opt(maybe_controller)?
            .map(|addr| Runtime::AddressMapping::into_account_id(addr.0));
        let maybe_total =
            Self::try_vec_to_opt(maybe_total)?.map(Self::u256_to_amount).transpose()?;
        Ok(())
    }

    fn u256_to_amount(value: U256) -> MayRevert<BalanceOf<Runtime>> {
        value
            .try_into()
            .map_err(|_| RevertReason::value_is_too_large("balance type").into())
    }

    fn try_vec_to_opt<T>(mut xs: Vec<T>) -> Result<Option<T>, PrecompileFailure> {
        match xs.pop() {
            None => Ok(None),
            Some(x) if xs.is_empty() => Ok(Some(x)),
            _ => {
                Err(Self::custom_err("Only empty or single element list is equivalent to optional"))
            },
        }
    }

    fn custom_err(s: &'static str) -> PrecompileFailure {
        PrecompileFailure::Error { exit_status: evm::ExitError::Other(s.into()) }
    }
}
