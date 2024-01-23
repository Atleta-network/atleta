//! This pallet is essentially a wrapper around `pallet_staking` from Substrate, with some minor
//! adaptations to our needs.

#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::WeightInfo as ThisWeightInfo;

// The speculative number of spans are used as an input of the weight annotation of
// [`Call::unbond`], as the post dipatch weight may depend on the number of slashing span on the
// account which is not provided as an input. The value set should be conservative but sensible.
pub(crate) const SPECULATIVE_NUM_SPANS: u32 = 32;

#[frame_support::pallet]
pub mod pallet {
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use pallet_staking::{BalanceOf, RewardDestination, WeightInfo};

    const STORAGE_VERSION: StorageVersion = StorageVersion::new(1);

    #[pallet::pallet]
    #[pallet::storage_version(STORAGE_VERSION)]
    pub struct Pallet<T>(_);

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_staking::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Type representing the weight of this pallet
        type WeightInfo: ThisWeightInfo + pallet_staking::WeightInfo;
    }

    // Storage items

	/// The minimum active bond to become and maintain the role of a nominator.
	#[pallet::storage]
	pub type MaxNominatorBond<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

	/// The minimum active bond to become and maintain the role of a validator.
	#[pallet::storage]
	pub type MaxValidatorBond<T: Config> = StorageValue<_, BalanceOf<T>, ValueQuery>;

    // Pallets use events to inform users when important changes are made.
    // https://docs.substrate.io/main-docs/build/events-errors/
    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        SomethingStored { something: u32, who: T::AccountId },
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// The bond is higher than allowed.
        BondTooHigh,
    }

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::call_index(0)]
        #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::bond())]
        pub fn bond(
            origin: OriginFor<T>,
            #[pallet::compact] value: BalanceOf<T>,
            payee: RewardDestination<T::AccountId>,
        ) -> DispatchResult {
            pallet_staking::Pallet::<T>::bond(origin, value, payee)
        }

        // TODO
        // #[pallet::call_index(1)]
        // #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::bond_extra())]
        // pub fn bond_extra(
        //     origin: OriginFor<T>,
        //     #[pallet::compact] max_additional: BalanceOf<T>,
        // ) -> DispatchResult {
        //     pallet_staking::Pallet::<T>::bond_extra(origin, max_additional)
        // }
        //
        // #[pallet::call_index(2)]
        // #[pallet::weight(
        //     <T as pallet_staking::Config>::WeightInfo::withdraw_unbonded_kill(SPECULATIVE_NUM_SPANS).saturating_add(<T as pallet_staking::Config>::WeightInfo::unbond()))
        // ]
        // pub fn unbond(
        //     origin: OriginFor<T>,
        //     #[pallet::compact] value: BalanceOf<T>,
        // ) -> DispatchResultWithPostInfo {
        //     pallet_staking::Pallet::<T>::unbond(origin, value)
        // }
        //
        // #[pallet::call_index(3)]
        // #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::withdraw_unbonded_kill(*num_slashing_spans))]
        // pub fn withdraw_unbonded(
        //     origin: OriginFor<T>,
        //     num_slashing_spans: u32,
        // ) -> DispatchResultWithPostInfo {
        //     pallet_staking::Pallet::<T>::withdraw_unbonded(origin, num_slashing_spans)
        // }
    }

    #[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub pallet_staking: pallet_staking::GenesisConfig<T>,
		pub max_nominator_bond: BalanceOf<T>,
		pub max_validator_bond: BalanceOf<T>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			MaxNominatorBond::<T>::put(self.max_nominator_bond);
			MaxValidatorBond::<T>::put(self.max_validator_bond);
		}
	}
}
