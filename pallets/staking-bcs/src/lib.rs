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
    use pallet_staking::{BalanceOf, RewardDestination, WeightInfo, AccountIdLookupOf};

    #[pallet::pallet]
    pub struct Pallet<T>(_);

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config + pallet_staking::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
        /// Type representing the weight of this pallet
        type WeightInfo: ThisWeightInfo + pallet_staking::WeightInfo;
    }

    // The pallet's runtime storage items.
    // https://docs.substrate.io/main-docs/build/runtime-storage/
    #[pallet::storage]
    #[pallet::getter(fn something)]
    // Learn more about declaring storage items:
    // https://docs.substrate.io/main-docs/build/runtime-storage/#declaring-storage-items
    pub type Something<T> = StorageValue<_, u32>;

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
        /// Error names should be descriptive.
        NoneValue,
        /// Errors should have helpful documentation associated with them.
        StorageOverflow,
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
            pallet_staking::Pallet::<T>::bond(origin, value, payee).into()
        }

        #[pallet::call_index(1)]
        #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::bond_extra())]
        pub fn bond_extra(
            origin: OriginFor<T>,
            #[pallet::compact] max_additional: BalanceOf<T>,
        ) -> DispatchResult {
            pallet_staking::Pallet::<T>::bond_extra(origin, max_additional)
        }

        #[pallet::call_index(2)]
        #[pallet::weight(
            <T as pallet_staking::Config>::WeightInfo::withdraw_unbonded_kill(SPECULATIVE_NUM_SPANS).saturating_add(<T as pallet_staking::Config>::WeightInfo::unbond()))
        ]
        pub fn unbond(
            origin: OriginFor<T>,
            #[pallet::compact] value: BalanceOf<T>,
        ) -> DispatchResultWithPostInfo {
            pallet_staking::Pallet::<T>::unbond(origin, value)
        }

        #[pallet::call_index(3)]
        #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::withdraw_unbonded_kill(*num_slashing_spans))]
        pub fn withdraw_unbonded(
            origin: OriginFor<T>,
            num_slashing_spans: u32,
        ) -> DispatchResultWithPostInfo {
            pallet_staking::Pallet::<T>::withdraw_unbonded(origin, num_slashing_spans)
        }

        #[pallet::call_index(4)]
        #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::validate())]
        pub fn validate(origin: OriginFor<T>, prefs: ValidatorPrefs) -> DispatchResult {
            pallet_staking::Pallet::<T>::validate(origin, prefs)
        }

        #[pallet::call_index(5)]
        #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::nominate(targets.len() as u32))]
        pub fn nominate(
            origin: OriginFor<T>,
            targets: Vec<AccountIdLookupOf<T>>,
        ) -> DispatchResult {
            pallet_staking::Pallet::<T>::nominate(origin, targets)
        }

        #[pallet::call_index(6)]
        #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::chill())]
        pub fn chill(origin: OriginFor<T>) -> DispatchResult {
            pallet_staking::Pallet::<T>::chill(origin)
        }

        //       /// (Re-)set the payment target for a controller.
        //       ///
        //       /// Effects will be felt instantly (as soon as this function is completed successfully).
        //       ///
        //       /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        //       ///
        //       /// ## Complexity
        //       /// - O(1)
        //       /// - Independent of the arguments. Insignificant complexity.
        //       /// - Contains a limited number of reads.
        //       /// - Writes are limited to the `origin` account key.
        //       /// ---------
        //       #[pallet::call_index(7)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::set_payee())]
        //       pub fn set_payee(
        //           origin: OriginFor<T>,
        //           payee: RewardDestination<T::AccountId>,
        //       ) -> DispatchResult {
        //           let controller = ensure_signed(origin)?;
        //           let ledger = Self::ledger(Controller(controller.clone()))?;
        //
        //           ensure!(
        //               (payee != {
        //                   #[allow(deprecated)]
        //                   RewardDestination::Controller
        //               }),
        //               Error::<T>::ControllerDeprecated
        //           );
        //
        //           let _ = ledger
        //               .set_payee(payee)
        //               .defensive_proof("ledger was retrieved from storage, thus its bonded; qed.")?;
        //
        //           Ok(())
        //       }
        //
        //       /// (Re-)sets the controller of a stash to the stash itself. This function previously
        //       /// accepted a `controller` argument to set the controller to an account other than the
        //       /// stash itself. This functionality has now been removed, now only setting the controller
        //       /// to the stash, if it is not already.
        //       ///
        //       /// Effects will be felt instantly (as soon as this function is completed successfully).
        //       ///
        //       /// The dispatch origin for this call must be _Signed_ by the stash, not the controller.
        //       ///
        //       /// ## Complexity
        //       /// O(1)
        //       /// - Independent of the arguments. Insignificant complexity.
        //       /// - Contains a limited number of reads.
        //       /// - Writes are limited to the `origin` account key.
        //       #[pallet::call_index(8)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::set_controller())]
        //       pub fn set_controller(origin: OriginFor<T>) -> DispatchResult {
        //           let stash = ensure_signed(origin)?;
        //
        //           // The bonded map and ledger are mutated directly as this extrinsic is related to a
        //           // (temporary) passive migration.
        //           Self::ledger(StakingAccount::Stash(stash.clone())).map(|ledger| {
        // 		let controller = ledger.controller()
        //                   .defensive_proof("Ledger's controller field didn't exist. The controller should have been fetched using StakingLedger.")
        //                   .ok_or(Error::<T>::NotController)?;
        //
        // 		if controller == stash {
        // 			// Stash is already its own controller.
        // 			return Err(Error::<T>::AlreadyPaired.into())
        // 		}
        // 		<Ledger<T>>::remove(controller);
        // 		<Bonded<T>>::insert(&stash, &stash);
        // 		<Ledger<T>>::insert(&stash, ledger);
        // 		Ok(())
        // 	})?
        //       }
        //
        //       /// Sets the ideal number of validators.
        //       ///
        //       /// The dispatch origin must be Root.
        //       ///
        //       /// ## Complexity
        //       /// O(1)
        //       #[pallet::call_index(9)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::set_validator_count())]
        //       pub fn set_validator_count(
        //           origin: OriginFor<T>,
        //           #[pallet::compact] new: u32,
        //       ) -> DispatchResult {
        //           ensure_root(origin)?;
        //           // ensure new validator count does not exceed maximum winners
        //           // support by election provider.
        //           ensure!(
        //               new <= <T::ElectionProvider as ElectionProviderBase>::MaxWinners::get(),
        //               Error::<T>::TooManyValidators
        //           );
        //           ValidatorCount::<T>::put(new);
        //           Ok(())
        //       }
        //
        //       /// Increments the ideal number of validators upto maximum of
        //       /// `ElectionProviderBase::MaxWinners`.
        //       ///
        //       /// The dispatch origin must be Root.
        //       ///
        //       /// ## Complexity
        //       /// Same as [`Self::set_validator_count`].
        //       #[pallet::call_index(10)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::set_validator_count())]
        //       pub fn increase_validator_count(
        //           origin: OriginFor<T>,
        //           #[pallet::compact] additional: u32,
        //       ) -> DispatchResult {
        //           ensure_root(origin)?;
        //           let old = ValidatorCount::<T>::get();
        //           let new = old.checked_add(additional).ok_or(ArithmeticError::Overflow)?;
        //           ensure!(
        //               new <= <T::ElectionProvider as ElectionProviderBase>::MaxWinners::get(),
        //               Error::<T>::TooManyValidators
        //           );
        //
        //           ValidatorCount::<T>::put(new);
        //           Ok(())
        //       }
        //
        //       /// Scale up the ideal number of validators by a factor upto maximum of
        //       /// `ElectionProviderBase::MaxWinners`.
        //       ///
        //       /// The dispatch origin must be Root.
        //       ///
        //       /// ## Complexity
        //       /// Same as [`Self::set_validator_count`].
        //       #[pallet::call_index(11)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::set_validator_count())]
        //       pub fn scale_validator_count(origin: OriginFor<T>, factor: Percent) -> DispatchResult {
        //           ensure_root(origin)?;
        //           let old = ValidatorCount::<T>::get();
        //           let new = old.checked_add(factor.mul_floor(old)).ok_or(ArithmeticError::Overflow)?;
        //
        //           ensure!(
        //               new <= <T::ElectionProvider as ElectionProviderBase>::MaxWinners::get(),
        //               Error::<T>::TooManyValidators
        //           );
        //
        //           ValidatorCount::<T>::put(new);
        //           Ok(())
        //       }
        //
        //       /// Force there to be no new eras indefinitely.
        //       ///
        //       /// The dispatch origin must be Root.
        //       ///
        //       /// # Warning
        //       ///
        //       /// The election process starts multiple blocks before the end of the era.
        //       /// Thus the election process may be ongoing when this is called. In this case the
        //       /// election will continue until the next era is triggered.
        //       ///
        //       /// ## Complexity
        //       /// - No arguments.
        //       /// - Weight: O(1)
        //       #[pallet::call_index(12)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::force_no_eras())]
        //       pub fn force_no_eras(origin: OriginFor<T>) -> DispatchResult {
        //           ensure_root(origin)?;
        //           Self::set_force_era(Forcing::ForceNone);
        //           Ok(())
        //       }
        //
        //       /// Force there to be a new era at the end of the next session. After this, it will be
        //       /// reset to normal (non-forced) behaviour.
        //       ///
        //       /// The dispatch origin must be Root.
        //       ///
        //       /// # Warning
        //       ///
        //       /// The election process starts multiple blocks before the end of the era.
        //       /// If this is called just before a new era is triggered, the election process may not
        //       /// have enough blocks to get a result.
        //       ///
        //       /// ## Complexity
        //       /// - No arguments.
        //       /// - Weight: O(1)
        //       #[pallet::call_index(13)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::force_new_era())]
        //       pub fn force_new_era(origin: OriginFor<T>) -> DispatchResult {
        //           ensure_root(origin)?;
        //           Self::set_force_era(Forcing::ForceNew);
        //           Ok(())
        //       }
        //
        //       /// Set the validators who cannot be slashed (if any).
        //       ///
        //       /// The dispatch origin must be Root.
        //       #[pallet::call_index(14)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::set_invulnerables(invulnerables.len() as u32))]
        //       pub fn set_invulnerables(
        //           origin: OriginFor<T>,
        //           invulnerables: Vec<T::AccountId>,
        //       ) -> DispatchResult {
        //           ensure_root(origin)?;
        //           <Invulnerables<T>>::put(invulnerables);
        //           Ok(())
        //       }
        //
        //       /// Force a current staker to become completely unstaked, immediately.
        //       ///
        //       /// The dispatch origin must be Root.
        //       ///
        //       /// ## Parameters
        //       ///
        //       /// - `num_slashing_spans`: Refer to comments on [`Call::withdraw_unbonded`] for more
        //       /// details.
        //       #[pallet::call_index(15)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::force_unstake(*num_slashing_spans))]
        //       pub fn force_unstake(
        //           origin: OriginFor<T>,
        //           stash: T::AccountId,
        //           num_slashing_spans: u32,
        //       ) -> DispatchResult {
        //           ensure_root(origin)?;
        //
        //           // Remove all staking-related information and lock.
        //           Self::kill_stash(&stash, num_slashing_spans)?;
        //
        //           Ok(())
        //       }
        //
        //       /// Force there to be a new era at the end of sessions indefinitely.
        //       ///
        //       /// The dispatch origin must be Root.
        //       ///
        //       /// # Warning
        //       ///
        //       /// The election process starts multiple blocks before the end of the era.
        //       /// If this is called just before a new era is triggered, the election process may not
        //       /// have enough blocks to get a result.
        //       #[pallet::call_index(16)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::force_new_era_always())]
        //       pub fn force_new_era_always(origin: OriginFor<T>) -> DispatchResult {
        //           ensure_root(origin)?;
        //           Self::set_force_era(Forcing::ForceAlways);
        //           Ok(())
        //       }
        //
        //       /// Cancel enactment of a deferred slash.
        //       ///
        //       /// Can be called by the `T::AdminOrigin`.
        //       ///
        //       /// Parameters: era and indices of the slashes for that era to kill.
        //       #[pallet::call_index(17)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::cancel_deferred_slash(slash_indices.len() as u32))]
        //       pub fn cancel_deferred_slash(
        //           origin: OriginFor<T>,
        //           era: EraIndex,
        //           slash_indices: Vec<u32>,
        //       ) -> DispatchResult {
        //           T::AdminOrigin::ensure_origin(origin)?;
        //
        //           ensure!(!slash_indices.is_empty(), Error::<T>::EmptyTargets);
        //           ensure!(is_sorted_and_unique(&slash_indices), Error::<T>::NotSortedAndUnique);
        //
        //           let mut unapplied = UnappliedSlashes::<T>::get(&era);
        //           let last_item = slash_indices[slash_indices.len() - 1];
        //           ensure!((last_item as usize) < unapplied.len(), Error::<T>::InvalidSlashIndex);
        //
        //           for (removed, index) in slash_indices.into_iter().enumerate() {
        //               let index = (index as usize) - removed;
        //               unapplied.remove(index);
        //           }
        //
        //           UnappliedSlashes::<T>::insert(&era, &unapplied);
        //           Ok(())
        //       }
        //
        //       /// Pay out next page of the stakers behind a validator for the given era.
        //       ///
        //       /// - `validator_stash` is the stash account of the validator.
        //       /// - `era` may be any era between `[current_era - history_depth; current_era]`.
        //       ///
        //       /// The origin of this call must be _Signed_. Any account can call this function, even if
        //       /// it is not one of the stakers.
        //       ///
        //       /// The reward payout could be paged in case there are too many nominators backing the
        //       /// `validator_stash`. This call will payout unpaid pages in an ascending order. To claim a
        //       /// specific page, use `payout_stakers_by_page`.`
        //       ///
        //       /// If all pages are claimed, it returns an error `InvalidPage`.
        //       #[pallet::call_index(18)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::payout_stakers_alive_staked(T::MaxExposurePageSize::get()))]
        //       pub fn payout_stakers(
        //           origin: OriginFor<T>,
        //           validator_stash: T::AccountId,
        //           era: EraIndex,
        //       ) -> DispatchResultWithPostInfo {
        //           ensure_signed(origin)?;
        //           Self::do_payout_stakers(validator_stash, era)
        //       }
        //
        //       /// Rebond a portion of the stash scheduled to be unlocked.
        //       ///
        //       /// The dispatch origin must be signed by the controller.
        //       ///
        //       /// ## Complexity
        //       /// - Time complexity: O(L), where L is unlocking chunks
        //       /// - Bounded by `MaxUnlockingChunks`.
        //       #[pallet::call_index(19)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::rebond(T::MaxUnlockingChunks::get() as u32))]
        //       pub fn rebond(
        //           origin: OriginFor<T>,
        //           #[pallet::compact] value: BalanceOf<T>,
        //       ) -> DispatchResultWithPostInfo {
        //           let controller = ensure_signed(origin)?;
        //           let ledger = Self::ledger(Controller(controller))?;
        //           ensure!(!ledger.unlocking.is_empty(), Error::<T>::NoUnlockChunk);
        //
        //           let initial_unlocking = ledger.unlocking.len() as u32;
        //           let (ledger, rebonded_value) = ledger.rebond(value);
        //           // Last check: the new active amount of ledger must be more than ED.
        //           ensure!(ledger.active >= T::Currency::minimum_balance(), Error::<T>::InsufficientBond);
        //
        //           Self::deposit_event(Event::<T>::Bonded {
        //               stash: ledger.stash.clone(),
        //               amount: rebonded_value,
        //           });
        //
        //           let stash = ledger.stash.clone();
        //           let final_unlocking = ledger.unlocking.len();
        //
        //           // NOTE: ledger must be updated prior to calling `Self::weight_of`.
        //           ledger.update()?;
        //           if T::VoterList::contains(&stash) {
        //               let _ = T::VoterList::on_update(&stash, Self::weight_of(&stash)).defensive();
        //           }
        //
        //           let removed_chunks = 1u32 // for the case where the last iterated chunk is not removed
        //               .saturating_add(initial_unlocking)
        //               .saturating_sub(final_unlocking as u32);
        //           Ok(Some(<T as pallet_staking::Config>::WeightInfo::rebond(removed_chunks)).into())
        //       }
        //
        //       /// Remove all data structures concerning a staker/stash once it is at a state where it can
        //       /// be considered `dust` in the staking system. The requirements are:
        //       ///
        //       /// 1. the `total_balance` of the stash is below existential deposit.
        //       /// 2. or, the `ledger.total` of the stash is below existential deposit.
        //       ///
        //       /// The former can happen in cases like a slash; the latter when a fully unbonded account
        //       /// is still receiving staking rewards in `RewardDestination::Staked`.
        //       ///
        //       /// It can be called by anyone, as long as `stash` meets the above requirements.
        //       ///
        //       /// Refunds the transaction fees upon successful execution.
        //       ///
        //       /// ## Parameters
        //       ///
        //       /// - `num_slashing_spans`: Refer to comments on [`Call::withdraw_unbonded`] for more
        //       /// details.
        //       #[pallet::call_index(20)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::reap_stash(*num_slashing_spans))]
        //       pub fn reap_stash(
        //           origin: OriginFor<T>,
        //           stash: T::AccountId,
        //           num_slashing_spans: u32,
        //       ) -> DispatchResultWithPostInfo {
        //           let _ = ensure_signed(origin)?;
        //
        //           let ed = T::Currency::minimum_balance();
        //           let reapable = T::Currency::total_balance(&stash) < ed
        //               || Self::ledger(Stash(stash.clone())).map(|l| l.total).unwrap_or_default() < ed;
        //           ensure!(reapable, Error::<T>::FundedTarget);
        //
        //           // Remove all staking-related information and lock.
        //           Self::kill_stash(&stash, num_slashing_spans)?;
        //
        //           Ok(Pays::No.into())
        //       }
        //
        //       /// Remove the given nominations from the calling validator.
        //       ///
        //       /// Effects will be felt at the beginning of the next era.
        //       ///
        //       /// The dispatch origin for this call must be _Signed_ by the controller, not the stash.
        //       ///
        //       /// - `who`: A list of nominator stash accounts who are nominating this validator which
        //       ///   should no longer be nominating this validator.
        //       ///
        //       /// Note: Making this call only makes sense if you first set the validator preferences to
        //       /// block any further nominations.
        //       #[pallet::call_index(21)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::kick(who.len() as u32))]
        //       pub fn kick(origin: OriginFor<T>, who: Vec<AccountIdLookupOf<T>>) -> DispatchResult {
        //           let controller = ensure_signed(origin)?;
        //           let ledger = Self::ledger(Controller(controller))?;
        //           let stash = &ledger.stash;
        //
        //           for nom_stash in who
        //               .into_iter()
        //               .map(T::Lookup::lookup)
        //               .collect::<Result<Vec<T::AccountId>, _>>()?
        //               .into_iter()
        //           {
        //               Nominators::<T>::mutate(&nom_stash, |maybe_nom| {
        //                   if let Some(ref mut nom) = maybe_nom {
        //                       if let Some(pos) = nom.targets.iter().position(|v| v == stash) {
        //                           nom.targets.swap_remove(pos);
        //                           Self::deposit_event(Event::<T>::Kicked {
        //                               nominator: nom_stash.clone(),
        //                               stash: stash.clone(),
        //                           });
        //                       }
        //                   }
        //               });
        //           }
        //
        //           Ok(())
        //       }
        //
        //       /// Update the various staking configurations .
        //       ///
        //       /// * `min_nominator_bond`: The minimum active bond needed to be a nominator.
        //       /// * `min_validator_bond`: The minimum active bond needed to be a validator.
        //       /// * `max_nominator_count`: The max number of users who can be a nominator at once. When
        //       ///   set to `None`, no limit is enforced.
        //       /// * `max_validator_count`: The max number of users who can be a validator at once. When
        //       ///   set to `None`, no limit is enforced.
        //       /// * `chill_threshold`: The ratio of `max_nominator_count` or `max_validator_count` which
        //       ///   should be filled in order for the `chill_other` transaction to work.
        //       /// * `min_commission`: The minimum amount of commission that each validators must maintain.
        //       ///   This is checked only upon calling `validate`. Existing validators are not affected.
        //       ///
        //       /// RuntimeOrigin must be Root to call this function.
        //       ///
        //       /// NOTE: Existing nominators and validators will not be affected by this update.
        //       /// to kick people under the new limits, `chill_other` should be called.
        //       // We assume the worst case for this call is either: all items are set or all items are
        //       // removed.
        //       #[pallet::call_index(22)]
        //       #[pallet::weight(
        // 	<T as pallet_staking::Config>::WeightInfo::set_staking_configs_all_set()
        // 		.max(<T as pallet_staking::Config>::WeightInfo::set_staking_configs_all_remove())
        // )]
        //       pub fn set_staking_configs(
        //           origin: OriginFor<T>,
        //           min_nominator_bond: ConfigOp<BalanceOf<T>>,
        //           min_validator_bond: ConfigOp<BalanceOf<T>>,
        //           max_nominator_count: ConfigOp<u32>,
        //           max_validator_count: ConfigOp<u32>,
        //           chill_threshold: ConfigOp<Percent>,
        //           min_commission: ConfigOp<Perbill>,
        //       ) -> DispatchResult {
        //           ensure_root(origin)?;
        //
        //           macro_rules! config_op_exp {
        //               ($storage:ty, $op:ident) => {
        //                   match $op {
        //                       ConfigOp::Noop => (),
        //                       ConfigOp::Set(v) => <$storage>::put(v),
        //                       ConfigOp::Remove => <$storage>::kill(),
        //                   }
        //               };
        //           }
        //
        //           config_op_exp!(MinNominatorBond<T>, min_nominator_bond);
        //           config_op_exp!(MinValidatorBond<T>, min_validator_bond);
        //           config_op_exp!(MaxNominatorsCount<T>, max_nominator_count);
        //           config_op_exp!(MaxValidatorsCount<T>, max_validator_count);
        //           config_op_exp!(ChillThreshold<T>, chill_threshold);
        //           config_op_exp!(MinCommission<T>, min_commission);
        //           Ok(())
        //       }
        //       /// Declare a `controller` to stop participating as either a validator or nominator.
        //       ///
        //       /// Effects will be felt at the beginning of the next era.
        //       ///
        //       /// The dispatch origin for this call must be _Signed_, but can be called by anyone.
        //       ///
        //       /// If the caller is the same as the controller being targeted, then no further checks are
        //       /// enforced, and this function behaves just like `chill`.
        //       ///
        //       /// If the caller is different than the controller being targeted, the following conditions
        //       /// must be met:
        //       ///
        //       /// * `controller` must belong to a nominator who has become non-decodable,
        //       ///
        //       /// Or:
        //       ///
        //       /// * A `ChillThreshold` must be set and checked which defines how close to the max
        //       ///   nominators or validators we must reach before users can start chilling one-another.
        //       /// * A `MaxNominatorCount` and `MaxValidatorCount` must be set which is used to determine
        //       ///   how close we are to the threshold.
        //       /// * A `MinNominatorBond` and `MinValidatorBond` must be set and checked, which determines
        //       ///   if this is a person that should be chilled because they have not met the threshold
        //       ///   bond required.
        //       ///
        //       /// This can be helpful if bond requirements are updated, and we need to remove old users
        //       /// who do not satisfy these requirements.
        //       #[pallet::call_index(23)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::chill_other())]
        //       pub fn chill_other(origin: OriginFor<T>, stash: T::AccountId) -> DispatchResult {
        //           // Anyone can call this function.
        //           let caller = ensure_signed(origin)?;
        //           let ledger = Self::ledger(Stash(stash.clone()))?;
        //           let controller = ledger
        // 		.controller()
        // 		.defensive_proof(
        // 			"Ledger's controller field didn't exist. The controller should have been fetched using StakingLedger.",
        // 		)
        // 		.ok_or(Error::<T>::NotController)?;
        //
        //           // In order for one user to chill another user, the following conditions must be met:
        //           //
        //           // * `controller` belongs to a nominator who has become non-decodable,
        //           //
        //           // Or
        //           //
        //           // * A `ChillThreshold` is set which defines how close to the max nominators or
        //           //   validators we must reach before users can start chilling one-another.
        //           // * A `MaxNominatorCount` and `MaxValidatorCount` which is used to determine how close
        //           //   we are to the threshold.
        //           // * A `MinNominatorBond` and `MinValidatorBond` which is the final condition checked to
        //           //   determine this is a person that should be chilled because they have not met the
        //           //   threshold bond required.
        //           //
        //           // Otherwise, if caller is the same as the controller, this is just like `chill`.
        //
        //           if Nominators::<T>::contains_key(&stash) && Nominators::<T>::get(&stash).is_none() {
        //               Self::chill_stash(&stash);
        //               return Ok(());
        //           }
        //
        //           if caller != controller {
        //               let threshold = ChillThreshold::<T>::get().ok_or(Error::<T>::CannotChillOther)?;
        //               let min_active_bond = if Nominators::<T>::contains_key(&stash) {
        //                   let max_nominator_count =
        //                       MaxNominatorsCount::<T>::get().ok_or(Error::<T>::CannotChillOther)?;
        //                   let current_nominator_count = Nominators::<T>::count();
        //                   ensure!(
        //                       threshold * max_nominator_count < current_nominator_count,
        //                       Error::<T>::CannotChillOther
        //                   );
        //                   MinNominatorBond::<T>::get()
        //               } else if Validators::<T>::contains_key(&stash) {
        //                   let max_validator_count =
        //                       MaxValidatorsCount::<T>::get().ok_or(Error::<T>::CannotChillOther)?;
        //                   let current_validator_count = Validators::<T>::count();
        //                   ensure!(
        //                       threshold * max_validator_count < current_validator_count,
        //                       Error::<T>::CannotChillOther
        //                   );
        //                   MinValidatorBond::<T>::get()
        //               } else {
        //                   Zero::zero()
        //               };
        //
        //               ensure!(ledger.active < min_active_bond, Error::<T>::CannotChillOther);
        //           }
        //
        //           Self::chill_stash(&stash);
        //           Ok(())
        //       }
        //
        //       /// Force a validator to have at least the minimum commission. This will not affect a
        //       /// validator who already has a commission greater than or equal to the minimum. Any account
        //       /// can call this.
        //       #[pallet::call_index(24)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::force_apply_min_commission())]
        //       pub fn force_apply_min_commission(
        //           origin: OriginFor<T>,
        //           validator_stash: T::AccountId,
        //       ) -> DispatchResult {
        //           ensure_signed(origin)?;
        //           let min_commission = MinCommission::<T>::get();
        //           Validators::<T>::try_mutate_exists(validator_stash, |maybe_prefs| {
        //               maybe_prefs
        //                   .as_mut()
        //                   .map(|prefs| {
        //                       (prefs.commission < min_commission)
        //                           .then(|| prefs.commission = min_commission)
        //                   })
        //                   .ok_or(Error::<T>::NotStash)
        //           })?;
        //           Ok(())
        //       }
        //
        //       /// Sets the minimum amount of commission that each validators must maintain.
        //       ///
        //       /// This call has lower privilege requirements than `set_staking_config` and can be called
        //       /// by the `T::AdminOrigin`. Root can always call this.
        //       #[pallet::call_index(25)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::set_min_commission())]
        //       pub fn set_min_commission(origin: OriginFor<T>, new: Perbill) -> DispatchResult {
        //           T::AdminOrigin::ensure_origin(origin)?;
        //           MinCommission::<T>::put(new);
        //           Ok(())
        //       }
        //
        //       /// Pay out a page of the stakers behind a validator for the given era and page.
        //       ///
        //       /// - `validator_stash` is the stash account of the validator.
        //       /// - `era` may be any era between `[current_era - history_depth; current_era]`.
        //       /// - `page` is the page index of nominators to pay out with value between 0 and
        //       ///   `num_nominators / T::MaxExposurePageSize`.
        //       ///
        //       /// The origin of this call must be _Signed_. Any account can call this function, even if
        //       /// it is not one of the stakers.
        //       ///
        //       /// If a validator has more than [`Config::MaxExposurePageSize`] nominators backing
        //       /// them, then the list of nominators is paged, with each page being capped at
        //       /// [`Config::MaxExposurePageSize`.] If a validator has more than one page of nominators,
        //       /// the call needs to be made for each page separately in order for all the nominators
        //       /// backing a validator to receive the reward. The nominators are not sorted across pages
        //       /// and so it should not be assumed the highest staker would be on the topmost page and vice
        //       /// versa. If rewards are not claimed in [`Config::HistoryDepth`] eras, they are lost.
        //       #[pallet::call_index(26)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::payout_stakers_alive_staked(T::MaxExposurePageSize::get()))]
        //       pub fn payout_stakers_by_page(
        //           origin: OriginFor<T>,
        //           validator_stash: T::AccountId,
        //           era: EraIndex,
        //           page: Page,
        //       ) -> DispatchResultWithPostInfo {
        //           ensure_signed(origin)?;
        //           Self::do_payout_stakers_by_page(validator_stash, era, page)
        //       }
        //
        //       /// Migrates an account's `RewardDestination::Controller` to
        //       /// `RewardDestination::Account(controller)`.
        //       ///
        //       /// Effects will be felt instantly (as soon as this function is completed successfully).
        //       ///
        //       /// This will waive the transaction fee if the `payee` is successfully migrated.
        //       #[pallet::call_index(27)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::update_payee())]
        //       pub fn update_payee(
        //           origin: OriginFor<T>,
        //           controller: T::AccountId,
        //       ) -> DispatchResultWithPostInfo {
        //           let _ = ensure_signed(origin)?;
        //           let ledger = Self::ledger(StakingAccount::Controller(controller.clone()))?;
        //
        //           ensure!(
        //               (Payee::<T>::get(&ledger.stash) == {
        //                   #[allow(deprecated)]
        //                   RewardDestination::Controller
        //               }),
        //               Error::<T>::NotController
        //           );
        //
        //           let _ = ledger
        //               .set_payee(RewardDestination::Account(controller))
        //               .defensive_proof("ledger should have been previously retrieved from storage.")?;
        //
        //           Ok(Pays::No.into())
        //       }
        //
        //       /// Updates a batch of controller accounts to their corresponding stash account if they are
        //       /// not the same. Ignores any controller accounts that do not exist, and does not operate if
        //       /// the stash and controller are already the same.
        //       ///
        //       /// Effects will be felt instantly (as soon as this function is completed successfully).
        //       ///
        //       /// The dispatch origin must be `T::AdminOrigin`.
        //       #[pallet::call_index(28)]
        //       #[pallet::weight(<T as pallet_staking::Config>::WeightInfo::deprecate_controller_batch(controllers.len() as u32))]
        //       pub fn deprecate_controller_batch(
        //           origin: OriginFor<T>,
        //           controllers: BoundedVec<T::AccountId, T::MaxControllersInDeprecationBatch>,
        //       ) -> DispatchResultWithPostInfo {
        //           T::AdminOrigin::ensure_origin(origin)?;
        //
        //           // Ignore controllers that do not exist or are already the same as stash.
        //           let filtered_batch_with_ledger: Vec<_> = controllers
        //               .iter()
        //               .filter_map(|controller| {
        //                   let ledger = Self::ledger(StakingAccount::Controller(controller.clone()));
        //                   ledger.ok().map_or(None, |ledger| {
        //                       // If the controller `RewardDestination` is still the deprecated
        //                       // `Controller` variant, skip deprecating this account.
        //                       let payee_deprecated = Payee::<T>::get(&ledger.stash) == {
        //                           #[allow(deprecated)]
        //                           RewardDestination::Controller
        //                       };
        //
        //                       if ledger.stash != *controller && !payee_deprecated {
        //                           Some((controller.clone(), ledger))
        //                       } else {
        //                           None
        //                       }
        //                   })
        //               })
        //               .collect();
        //
        //           // Update unique pairs.
        //           for (controller, ledger) in filtered_batch_with_ledger {
        //               let stash = ledger.stash.clone();
        //
        //               <Bonded<T>>::insert(&stash, &stash);
        //               <Ledger<T>>::remove(controller);
        //               <Ledger<T>>::insert(stash, ledger);
        //           }
        //           Ok(Some(<T as pallet_staking::Config>::WeightInfo::deprecate_controller_batch(controllers.len() as u32)).into())
        //       }
    }
}
