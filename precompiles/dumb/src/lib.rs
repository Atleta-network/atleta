#![cfg_attr(not(feature = "std"), no_std)]

use fp_evm::PrecompileHandle;
use precompile_utils::prelude::*;
use sp_core::U256;
use sp_std::marker::PhantomData;

pub struct DumbPrecompile<Runtime>(PhantomData<Runtime>);

#[precompile_utils::precompile]
impl<Runtime> DumbPrecompile<Runtime>
where
    Runtime: pallet_evm::Config,
{
    #[precompile::public("available(address)")]
    #[precompile::view]
    fn available(handle: &mut impl PrecompileHandle, _address: Address) -> EvmResult<U256> {
        handle.record_db_read::<Runtime>(42)?;
        Ok(U256::zero())
    }
}
