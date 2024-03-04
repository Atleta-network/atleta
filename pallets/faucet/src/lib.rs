//! This pallet implements a straightforward faucet mechanism. It allows users to request funds exclusively for their own accounts.
//! The origin should be signed.
//!
//! Users are limited to requesting up to the `Config::FaucetAmount` within a `Config::AccumulationPeriod` period.
//!
//! Designed solely for use within test networks.
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(clippy::all)]
#![warn(missing_docs)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

pub mod weights;
pub use weights::*;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::traits::ExistenceRequirement;
    use frame_support::{pallet_prelude::*, traits::Currency};
    use frame_system::pallet_prelude::*;

    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_balances::Config {
        /// The type of events defined by the pallet.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

        /// Estimate of resource consumption for pallet operations.
        type WeightInfo: WeightInfo;

        /// The period during which the user can't request more than `Config::MaxAmount`.
        #[pallet::constant]
        type AccumulationPeriod: Get<BlockNumberFor<Self>>;

        /// Faucet amount.
        #[pallet::constant]
        type FaucetAmount: Get<Self::Balance>;
    }

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Holding all requests by account.
    #[pallet::storage]
    #[pallet::getter(fn requests)]
    pub type Requests<T: Config> =
        StorageMap<_, Blake2_128Concat, T::AccountId, (T::Balance, BlockNumberFor<T>), ValueQuery>;

    /// Holding genesis account for funds sending.
    #[pallet::storage]
    #[pallet::getter(fn genesis_account)]
    pub type GenesisAccount<T: Config> = StorageValue<_, Option<T::AccountId>, ValueQuery>;

    #[pallet::genesis_config]
    #[derive(frame_support::DefaultNoBound)]
    pub struct GenesisConfig<T: Config> {
        /// Account for funds sending.
        pub genesis_account: Option<T::AccountId>,
    }

    #[pallet::genesis_build]
    impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
        fn build(&self) {
            <GenesisAccount<T>>::put(self.genesis_account.clone());
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
            amount: T::Balance,
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
        pub fn request_funds(origin: OriginFor<T>, amount: T::Balance) -> DispatchResult {
            let who = ensure_signed(origin)?;

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

            let genesis_account = Self::genesis_account().ok_or(Error::<T>::NoFaucetAccount)?;

            let _ = pallet_balances::Pallet::<T>::transfer(
                &genesis_account,
                &who,
                amount,
                ExistenceRequirement::KeepAlive,
            );

            Requests::<T>::insert(&who, (total, now));

            Self::deposit_event(Event::FundsSent { who, amount });

            Ok(())
        }
    }
}
