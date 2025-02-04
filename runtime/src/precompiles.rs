use frame_support::dispatch::{GetDispatchInfo, Pays};
use sp_std::marker::PhantomData;

use pallet_evm::ExitError;
use pallet_evm_precompile_dispatch::{Dispatch, DispatchValidateT};
use pallet_evm_precompile_modexp::Modexp;
use pallet_evm_precompile_sha3fips::Sha3FIPS256;
use pallet_evm_precompile_simple::{ECRecover, ECRecoverPublicKey, Identity, Ripemd160, Sha256};

use pallet_evm_precompile_babe::BabePrecompile;
use pallet_evm_precompile_governance::GovernancePrecompile;
use pallet_evm_precompile_nomination_pools::NominationPoolsPrecompile;
use pallet_evm_precompile_preimage::PreimagePrecompile;
use pallet_evm_precompile_staking::StakingPrecompile;
use pallet_evm_precompile_treasury::TreasuryPrecompile;
use pallet_evm_precompile_utility::BatchPrecompile;

use frame_support::traits::Contains;
use precompile_utils::precompile_set::*;

use crate::*;

/// Precompile checks for ethereum spec precompiles
/// We allow DELEGATECALL to stay compliant with Ethereum behavior.
type EthereumPrecompilesChecks = (AcceptDelegateCall, CallableByContract, CallableByPrecompile);

/// Filter that only allows whitelisted runtime call to pass through dispatch precompile
pub struct WhitelistedCalls;

impl Contains<RuntimeCall> for WhitelistedCalls {
    fn contains(t: &RuntimeCall) -> bool {
        match t {
            RuntimeCall::Utility(pallet_utility::Call::batch { calls })
            | RuntimeCall::Utility(pallet_utility::Call::batch_all { calls }) => {
                calls.iter().all(WhitelistedCalls::contains)
            },
            RuntimeCall::Democracy(..) => true,
            RuntimeCall::Staking(..) => true,
            RuntimeCall::Elections(..) => true,
            RuntimeCall::Preimage(..) => true,
            RuntimeCall::NominationPools(..) => true,
            RuntimeCall::Treasury(..) => true,
            _ => false,
        }
    }
}

#[precompile_utils::precompile_name_from_address]
type AtletaPrecompilesAt<R> = (
    // Ethereum precompiles:
    PrecompileAt<AddressU64<1>, ECRecover, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<2>, Sha256, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<3>, Ripemd160, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<4>, Identity, EthereumPrecompilesChecks>,
    PrecompileAt<AddressU64<5>, Modexp, EthereumPrecompilesChecks>,
    // Non-Frontier specific nor Ethereum precompiles:
    PrecompileAt<AddressU64<1024>, Sha3FIPS256, (CallableByContract, CallableByPrecompile)>,
    PrecompileAt<AddressU64<1025>, ECRecoverPublicKey, (CallableByContract, CallableByPrecompile)>,
    PrecompileAt<
        AddressU64<1026>,
        Dispatch<R, DispatchFilterValidate<RuntimeCall, WhitelistedCalls>>,
        // Not callable from smart contract nor precompiles, only EOA accounts
        (),
    >,
    // Atleta Precompiles
    PrecompileAt<
        AddressU64<2001>,
        GovernancePrecompile<R>,
        (CallableByContract, CallableByPrecompile),
    >,
    PrecompileAt<
        AddressU64<2002>,
        TreasuryPrecompile<R>,
        (CallableByContract, CallableByPrecompile),
    >,
    PrecompileAt<
        AddressU64<2003>,
        PreimagePrecompile<R>,
        (CallableByContract, CallableByPrecompile),
    >,
    PrecompileAt<
        AddressU64<2004>,
        StakingPrecompile<R>,
        (CallableByContract, CallableByPrecompile),
    >,
    PrecompileAt<
        AddressU64<2006>,
        NominationPoolsPrecompile<R>,
        (CallableByContract, CallableByPrecompile),
    >,
    PrecompileAt<AddressU64<2007>, BabePrecompile<R>, (CallableByContract, CallableByPrecompile)>,
    PrecompileAt<
        AddressU64<2008>,
        BatchPrecompile<R>,
        (SubcallWithMaxNesting<2>, CallableByPrecompile<OnlyFrom<AddressU64<2056>>>),
    >,
);

/// The PrecompileSet installed in the Atleta runtime.
pub type AtletaPrecompiles<R> = PrecompileSetBuilder<
    R,
    (
        // Skip precompiles if out of range.
        PrecompilesInRangeInclusive<(AddressU64<1>, AddressU64<5094>), AtletaPrecompilesAt<R>>,
    ),
>;

/// Struct that allows only calls based on `Filter` to pass through.
pub struct DispatchFilterValidate<RuntimeCall, Filter: Contains<RuntimeCall>>(
    PhantomData<(RuntimeCall, Filter)>,
);

impl<AccountId, RuntimeCall: GetDispatchInfo, Filter: Contains<RuntimeCall>>
    DispatchValidateT<AccountId, RuntimeCall> for DispatchFilterValidate<RuntimeCall, Filter>
{
    fn validate_before_dispatch(
        _origin: &AccountId,
        call: &RuntimeCall,
    ) -> Option<fp_evm::PrecompileFailure> {
        let info = call.get_dispatch_info();
        let paid_normal_call = info.pays_fee == Pays::Yes && info.class == DispatchClass::Normal;
        if !paid_normal_call {
            return Some(fp_evm::PrecompileFailure::Error {
                exit_status: ExitError::Other("invalid call".into()),
            });
        }
        if Filter::contains(call) {
            None
        } else {
            Some(fp_evm::PrecompileFailure::Error {
                exit_status: ExitError::Other("call filtered out".into()),
            })
        }
    }
}
