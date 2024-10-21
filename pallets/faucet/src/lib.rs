//! This pallet implements a straightforward faucet mechanism. It allows users to request funds exclusively for their own accounts.
//! The origin should be signed.
//!
//! Users are limited to requesting up to the `Config::FaucetAmount` within a `Config::AccumulationPeriod` period.
//!
//! Designed solely for use within test networks.
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::all)]
#![warn(missing_docs)]
#![allow(clippy::manual_inspect)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

use frame_support::{
    traits::{Currency, Get},
    PalletId,
};
use sp_runtime::traits::AccountIdConversion;

type BalanceOf<T> =
    <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::{TransactionValidity, *};
    use frame_support::traits::ExistenceRequirement;
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_balances::Config {
        /// The type of events defined by the pallet.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// The currency mechanism, used for Faucet.
        type Currency: Currency<Self::AccountId>;

        /// Estimate of resource consumption for pallet operations.
        type WeightInfo: WeightInfo;

        /// The faucet's pallet id, used for deriving its sovereign account ID.
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// The period during which the user can't request more than `Config::MaxAmount`.
        #[pallet::constant]
        type AccumulationPeriod: Get<BlockNumberFor<Self>>;

        /// Faucet amount.
        #[pallet::constant]
        type FaucetAmount: Get<BalanceOf<Self>>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Holding all requests by account.
    #[pallet::storage]
    #[pallet::getter(fn requests)]
    pub type Requests<T: Config> = StorageMap<
        _,
        Blake2_128Concat,
        T::AccountId,
        (BalanceOf<T>, BlockNumberFor<T>),
        ValueQuery,
    >;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        #[serde(skip)]
        _config: sp_std::marker::PhantomData<T>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            // Create faucet account.
            let account_id = <Pallet<T>>::account_id();
            let min = T::Currency::minimum_balance();

            if T::Currency::free_balance(&account_id) < min {
                let _ = T::Currency::make_free_balance_be(&account_id, min);
            }
        }
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// The requested funds sent to `who`. [who, amount]
        FundsSent {
            /// The account ID reaceved the funds.
            who: T::AccountId,
            /// The amount of funds.
            amount: BalanceOf<T>,
        },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// Request amount more than `Config::MaxAmount`.
        AmountTooHigh,
        /// More than allowed funds requested during `Config::AccumulationPeriod`.
        RequestLimitExceeded,
        /// No account to send funds.
        NoFaucetAccount,
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Request some funds.
        #[pallet::call_index(0)]
        #[pallet::weight(
        (<T as Config>::WeightInfo::request_funds(), DispatchClass::Normal, Pays::No)
        )]
        pub fn request_funds(
            origin: OriginFor<T>,
            who: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            ensure_none(origin)?;

            ensure!(amount <= T::FaucetAmount::get(), Error::<T>::AmountTooHigh);

            let (balance, timestamp) = Requests::<T>::get(&who);
            let now = frame_system::Pallet::<T>::block_number();
            let period = now - timestamp;

            let (total, now) = if period >= T::AccumulationPeriod::get() {
                (amount, now)
            } else {
                (balance + amount, timestamp)
            };

            ensure!(total <= T::FaucetAmount::get(), Error::<T>::RequestLimitExceeded);

            let account_id = Self::account_id();

            let _ = T::Currency::make_free_balance_be(&account_id, amount);
            let _ =
                T::Currency::transfer(&account_id, &who, amount, ExistenceRequirement::AllowDeath);

            Requests::<T>::insert(&who, (total, now));

            Self::deposit_event(Event::FundsSent { who, amount });

            Ok(())
        }
    }

    #[pallet::validate_unsigned]
    impl<T: Config> ValidateUnsigned for Pallet<T> {
        type Call = Call<T>;

        fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
            match call {
                Call::request_funds { who, amount } => ValidTransaction::with_tag_prefix("Faucet")
                    .and_provides((who, amount))
                    .propagate(true)
                    .build(),
                _ => InvalidTransaction::Call.into(),
            }
        }
    }
}

impl<T: Config> Pallet<T> {
    /// The account ID to transfer faucet amount to user.
    pub fn account_id() -> T::AccountId {
        T::PalletId::get().into_account_truncating()
    }
}
