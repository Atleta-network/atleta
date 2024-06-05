#![cfg_attr(not(feature = "std"), no_std)]

use frame_support::{
    crypto::ecdsa::ECDSAExt, dispatch::GetDispatchInfo, traits::UnfilteredDispatchable,
};
use sp_core::{
    ecdsa::{Public, Signature},
    H160, H256,
};
use sp_io::crypto::secp256k1_ecdsa_recover_compressed;
use sp_std::{boxed::Box, vec::Vec};

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

pub mod weights;
pub use weights::WeightInfo;

mod eip712;
use eip712::{Domain, Payload, TypedData};

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::{pallet_prelude::*, traits::Get};
    use frame_system::{pallet_prelude::*, Account, RawOrigin};

    #[pallet::config(with_default)]
    pub trait Config: frame_system::Config + pallet_evm_chain_id::Config {
        type Sender: Parameter + Into<sp_core::H160> + Into<Self::AccountId>;
        type Nonce: Parameter + Into<sp_core::U256> + PartialEq;

        #[pallet::no_default_bounds]
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        #[pallet::no_default_bounds]
        type RuntimeCall: Parameter
            + UnfilteredDispatchable<RuntimeOrigin = Self::RuntimeOrigin>
            + GetDispatchInfo;

        type WeightInfo: WeightInfo;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight({
            let dispatch_info = call.get_dispatch_info();
            (dispatch_info.weight, dispatch_info.class)
        })]
        pub fn signed_call(
            _: OriginFor<T>,
            sender: <T as Config>::Sender,
            nonce: <T as Config>::Nonce, // sender nonce, part of signed data to prevent replay attack
            signature: Vec<u8>,          // 'eth_signTypedData_v4' result
            call: Box<<T as Config>::RuntimeCall>,
        ) -> DispatchResultWithPostInfo {
            // TODO: mess with the nonce: synch check with `validate_unsigned` and increment
            let chain_id = pallet_evm_chain_id::Pallet::<T>::get();

            {
                let sender: sp_core::H160 = sender.clone().into();
                let call = call.encode();
                let nonce: sp_core::U256 = nonce.into();

                let domain = Domain {
                    name: b"ATLA".into(),
                    version: b"1".into(),
                    chain_id: chain_id.into(),
                    verifying_contract: sp_core::H160::zero(),
                };

                let payload = Payload { sender, nonce, call };
                let typed_data = TypedData::new(domain, payload);

                let hash = typed_data.message_hash();

                let signature =
                    parse_signature(&signature).map_err(|_| Error::<T>::BadSignatureFormat)?;

                let origin = recover_signer_address(signature, hash)
                    .map_err(|_| Error::<T>::EcdsaRecoverErr)?;

                frame_support::ensure!(sender == origin, Error::<T>::SignerMismath);
            }

            let signer: <T as frame_system::Config>::AccountId = sender.into();
            let result = call.dispatch_bypass_filter(RawOrigin::Signed(signer.clone()).into());
            Self::deposit_event(Event::Authorized {
                signed_by: signer,
                call_result: result.map(|_| ()).map_err(|e| e.error),
            });

            Ok(Pays::No.into()) // TODO: take the fee from account
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        Authorized { signed_by: T::AccountId, call_result: DispatchResult },
    }

    #[pallet::error]
    pub enum Error<T> {
        BadSignatureFormat,
        EcdsaRecoverErr,
        SignerMismath,
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T>
    where
        <T as Config>::Nonce: PartialEq<<T as frame_system::Config>::Nonce>,
    {
        type Call = Call<T>;

        fn validate_unsigned(_: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::signed_call { sender, nonce, signature, call } => {
                    let signer: T::AccountId = sender.clone().into();

                    if *nonce == Account::<T>::get(signer).nonce {
                        ValidTransaction::with_tag_prefix("Metamask")
                            .and_provides((sender, nonce, signature, call))
                            .propagate(true)
                            .build()
                    } else {
                        InvalidTransaction::Call.into()
                    }
                },
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}

#[derive(Debug)]
pub struct SignatureParseError;

pub fn parse_signature(hex: &[u8]) -> Result<Signature, SignatureParseError> {
    use sp_std::str;

    if hex.len() != 132 {
        return Err(SignatureParseError);
    }
    let sh = match hex.strip_prefix(b"0x") {
        Some(sh) if sh.len() == 130 => sh,
        _ => return Err(SignatureParseError),
    };

    let mut bytes = [0u8; 65]; // r: 32, s: 32, v: 1
    for (i, chunk) in sh.chunks(2).enumerate() {
        let s = str::from_utf8(chunk).map_err(|_| SignatureParseError)?;
        bytes[i] = u8::from_str_radix(s, 16).map_err(|_| SignatureParseError)?;
    }

    Ok(Signature::from_raw(bytes))
}

#[derive(Debug)]
pub struct SignerRecoverError;

pub fn recover_signer_address(
    signature: Signature,
    hash: H256,
) -> Result<H160, SignerRecoverError> {
    secp256k1_ecdsa_recover_compressed(signature.as_ref(), hash.as_fixed_bytes())
        .map(Public)
        .and_then(|pubkey| {
            pubkey
                .to_eth_address()
                .map(H160)
                .map_err(|_| panic!("Public to H160 identity conversion must exists"))
        })
        .map_err(|_| SignerRecoverError)
}
