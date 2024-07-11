#![cfg_attr(not(feature = "std"), no_std)]
#![allow(missing_docs)]

use fp_evm::PrecompileHandle;
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    traits::Currency,
};
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{H160, U256};
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_std::marker::PhantomData;

pub struct FaucetPrecompile<Runtime>(PhantomData<Runtime>);

type BalanceOf<Runtime> = <<Runtime as pallet_faucet::Config>::Currency as Currency<
    <Runtime as frame_system::Config>::AccountId,
>>::Balance;

#[precompile_utils::precompile]
impl<Runtime> FaucetPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_faucet::Config,
    Runtime::AccountId: Into<H160>,
    BalanceOf<Runtime>: TryFrom<U256> + Into<U256> + solidity::Codec,
    <Runtime as pallet_faucet::Config>::Currency:
        frame_support::traits::fungible::Inspect<<Runtime as frame_system::Config>::AccountId>,
    Runtime::Lookup: StaticLookup<Source = Runtime::AccountId>,
    Runtime::RuntimeCall: From<pallet_faucet::Call<Runtime>>,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
{
    #[precompile::public("requestFunds(address,uint256)")]
    fn request_funds(h: &mut impl PrecompileHandle, who: Address, amount: U256) -> EvmResult<()> {
        let who = Runtime::AddressMapping::into_account_id(who.0);
        let amount = Self::u256_to_amount(amount)?;

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
}
