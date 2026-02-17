//! Native token burn precompile at address `0x7D5` (2005).
//! Allows Solidity contracts to burn native ATLA from `msg.sender`,
//! atomically reducing `TotalIssuance`. Only self-burn is supported.

#![cfg_attr(not(feature = "std"), no_std)]
#![allow(missing_docs)]

extern crate alloc;

use alloc::format;

use fp_evm::PrecompileHandle;
use frame_support::traits::{
    fungible::{Inspect, Mutate},
    tokens::{Fortitude, Precision},
};
use pallet_evm::{AddressMapping, PrecompileFailure};
use precompile_utils::prelude::*;
use sp_core::{H160, U256};
use sp_std::marker::PhantomData;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub const SELECTOR_LOG_BURNED: [u8; 32] = keccak256!("Burned(address,uint256)");

pub struct BurnPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> BurnPrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_balances::Config,
    Runtime::AccountId: Into<H160>,
    <Runtime as pallet_balances::Config>::Balance: TryFrom<U256> + Into<U256>,
{
    /// `burn(uint256)` — burns caller's tokens via `fungible::Mutate::burn_from`.
    /// Exact (revert if insufficient), Polite (respect locks).
    #[precompile::public("burn(uint256)")]
    fn burn(handle: &mut impl PrecompileHandle, amount: U256) -> EvmResult<bool> {
        // 1 read (account) + 2 writes (balance + TotalIssuance)
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;
        handle.record_cost(RuntimeHelper::<Runtime>::db_write_gas_cost())?;

        let caller = Runtime::AddressMapping::into_account_id(handle.context().caller);

        let amount: <Runtime as pallet_balances::Config>::Balance =
            amount.try_into().map_err(|_| RevertReason::value_is_too_large("amount type"))?;

        <pallet_balances::Pallet<Runtime> as Mutate<Runtime::AccountId>>::burn_from(
            &caller,
            amount,
            Precision::Exact,
            Fortitude::Polite,
        )
        .map_err(|e| PrecompileFailure::Error {
            exit_status: evm::ExitError::Other(format!("burn failed: {:?}", e).into()),
        })?;

        let event = log2(
            handle.context().address,
            SELECTOR_LOG_BURNED,
            handle.context().caller,
            solidity::encode_event_data(Into::<U256>::into(amount)),
        );
        event.record(handle)?;

        Ok(true)
    }

    /// `totalIssuance()` — view function returning current total supply.
    #[precompile::public("totalIssuance()")]
    #[precompile::view]
    fn total_issuance(handle: &mut impl PrecompileHandle) -> EvmResult<U256> {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

        let issuance =
            <pallet_balances::Pallet<Runtime> as Inspect<Runtime::AccountId>>::total_issuance();

        Ok(issuance.into())
    }
}
