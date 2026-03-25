#![cfg_attr(not(feature = "std"), no_std)]
#![allow(missing_docs)]

use fp_evm::PrecompileHandle;
use frame_support::{
    dispatch::{GetDispatchInfo, PostDispatchInfo},
    traits::{IsType, QueryPreimage},
};
use pallet_evm::AddressMapping;
use precompile_utils::prelude::*;
use sp_core::{H160, H256};
use sp_runtime::traits::{Dispatchable, StaticLookup};
use sp_std::{marker::PhantomData, vec::Vec};

pub struct PreimagePrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> PreimagePrecompile<Runtime>
where
    Runtime: pallet_evm::Config + pallet_preimage::Config,
    Runtime::AccountId: Into<H160>,
    Runtime::Hash: IsType<H256>,
    Runtime::Lookup: StaticLookup<Source = Runtime::AccountId>,
    Runtime::RuntimeCall: From<pallet_preimage::Call<Runtime>>,
    <Runtime::RuntimeCall as Dispatchable>::RuntimeOrigin: From<Option<Runtime::AccountId>>,
    Runtime::RuntimeCall: Dispatchable<PostInfo = PostDispatchInfo> + GetDispatchInfo,
{
    #[precompile::public("notePreimage(uint8[])")]
    fn note_preimage(h: &mut impl PrecompileHandle, bytes: Vec<u8>) -> EvmResult<()> {
        let call = pallet_preimage::Call::<Runtime>::note_preimage { bytes };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("unnotePreimage(bytes32)")]
    fn unnote_preimage(h: &mut impl PrecompileHandle, hash: H256) -> EvmResult<()> {
        let call = pallet_preimage::Call::<Runtime>::unnote_preimage { hash: hash.into() };
        let origin = Some(Runtime::AddressMapping::into_account_id(h.context().caller));
        RuntimeHelper::<Runtime>::try_dispatch(h, origin.into(), call)?;
        Ok(())
    }

    #[precompile::public("preimageStatus(bytes32)")]
    #[precompile::view]
    fn preimage_status(handle: &mut impl PrecompileHandle, hash: H256) -> EvmResult<bool> {
        handle.record_cost(RuntimeHelper::<Runtime>::db_read_gas_cost())?;

        Ok(<pallet_preimage::Pallet<Runtime> as QueryPreimage>::is_requested(&hash.into()))
    }
}
