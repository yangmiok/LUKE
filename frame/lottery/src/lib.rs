// This file is part of Substrate.

// Copyright (C) 2021-2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A lottery pallet that uses participation in the network to purchase tickets.
//!
//! With this pallet, you can configure a lottery, which is a pot of money that
//! users contribute to, and that is reallocated to a single user at the end of
//! the lottery period. Just like a normal lottery system, to participate, you
//! need to "buy a ticket", which is used to fund the pot.
//!
//! The unique feature of this lottery system is that tickets can only be
//! purchased by making a "valid call" dispatched through this pallet.
//! By configuring certain calls to be valid for the lottery, you can encourage
//! users to make those calls on your network. An example of how this could be
//! used is to set validator nominations as a valid lottery call. If the lottery
//! is set to repeat every month, then users would be encouraged to re-nominate
//! validators every month. A user can only purchase one ticket per valid call
//! per lottery.
//!
//! This pallet can be configured to use dynamically set calls or statically set
//! calls. Call validation happens through the `ValidateCall` implementation.
//! This pallet provides one implementation of this using the `CallIndices`
//! storage item. You can also make your own implementation at the runtime level
//! which can contain much more complex logic, such as validation of the
//! parameters, which this pallet alone cannot do.
//!
//! This pallet uses the modulus operator to pick a random winner. It is known
//! that this might introduce a bias if the random number chosen in a range that
//! is not perfectly divisible by the total number of participants. The
//! `MaxGenerateRandom` configuration can help mitigate this by generating new
//! numbers until we hit the limit or we find a "fair" number. This is best
//! effort only.

#![cfg_attr(not(feature = "std"), no_std)]

mod benchmarking;
#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use codec::{Decode, Encode};
use frame_support::{
    dispatch::{DispatchResult, Dispatchable, GetDispatchInfo},
    ensure,
    pallet_prelude::MaxEncodedLen,
    storage::bounded_vec::BoundedVec,
    traits::{Currency, ExistenceRequirement::KeepAlive, Get, Randomness, ReservableCurrency},
    PalletId, RuntimeDebug,
};
pub use pallet::*;
use sp_runtime::{
    traits::{AccountIdConversion, Saturating, Zero},
    ArithmeticError, DispatchError,
};
use sp_std::prelude::*;
pub use weights::WeightInfo;

type BalanceOf<T> =
<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

// Any runtime call can be encoded into two bytes which represent the pallet and call index.
// We use this to uniquely match someone's incoming call with the calls configured for the lottery.
type CallIndex = (u8, u8);

#[derive(
Encode, Decode, Default, Eq, PartialEq, RuntimeDebug, scale_info::TypeInfo, MaxEncodedLen,
)]
pub struct LotteryConfig<BlockNumber, Balance> {
    /// Price per entry.
    price: Balance,
    /// Starting block of the lottery.
    start: BlockNumber,
    /// Length of the lottery (start + length = end).
    length: BlockNumber,
    /// Delay for choosing the winner of the lottery. (start + length + delay = payout).
    /// Randomness in the "payout" block will be used to determine the winner.
    delay: BlockNumber,
    /// Whether this lottery will repeat after it completes.
    repeat: bool,
}

pub trait ValidateCall<T: Config> {
    fn validate_call(call: &<T as Config>::Call) -> bool;
}

impl<T: Config> ValidateCall<T> for () {
    fn validate_call(_: &<T as Config>::Call) -> bool {
        false
    }
}

impl<T: Config> ValidateCall<T> for Pallet<T> {
    fn validate_call(call: &<T as Config>::Call) -> bool {
        let valid_calls = CallIndices::<T>::get();
        let call_index = match Self::call_to_index(&call) {
            Ok(call_index) => call_index,
            Err(_) => return false,
        };
        valid_calls.iter().any(|c| call_index == *c)
    }
}

#[frame_support::pallet]
pub mod pallet {
    use frame_support::log::info;
    use super::*;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use sp_runtime::traits::CheckedAdd;

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    /// The pallet's config trait.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// The Lottery's pallet id
        #[pallet::constant]
        type PalletId: Get<PalletId>;

        /// A dispatchable call.
        type Call: Parameter
        + Dispatchable<Origin=Self::Origin>
        + GetDispatchInfo
        + From<frame_system::Call<Self>>;

        /// The currency trait.
        type Currency: ReservableCurrency<Self::AccountId>;

        /// Something that provides randomness in the runtime.
        type Randomness: Randomness<Self::Hash, Self::BlockNumber>;

        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// The manager origin.
        type ManagerOrigin: EnsureOrigin<Self::Origin>;

        /// The max number of calls available in a single lottery.
        #[pallet::constant]
        type MaxCalls: Get<u32>;

        /// Used to determine if a call would be valid for purchasing a ticket.
        ///
        /// Be conscious of the implementation used here. We assume at worst that
        /// a vector of `MaxCalls` indices are queried for any call validation.
        /// You may need to provide a custom benchmark if this assumption is broken.
        type ValidateCall: ValidateCall<Self>;

        /// Number of time we should try to generate a random number that has no modulo bias.
        /// The larger this number, the more potential computation is used for picking the winner,
        /// but also the more likely that the chosen winner is done fairly.
        #[pallet::constant]
        type MaxGenerateRandom: Get<u32>;

        /// Weight information for extrinsics in this pallet.
        type WeightInfo: WeightInfo;
        type Hundred: Get<BalanceOf<Self>>;
        type Ninety: Get<BalanceOf<Self>>;
        type Eighty: Get<BalanceOf<Self>>;
        type Seventy: Get<BalanceOf<Self>>;
        type Sixty: Get<BalanceOf<Self>>;
        type Fifty: Get<BalanceOf<Self>>;
        type Forty: Get<BalanceOf<Self>>;
        type Thirty: Get<BalanceOf<Self>>;
        type Twenty: Get<BalanceOf<Self>>;
        type Ten: Get<BalanceOf<Self>>;
        type One: Get<BalanceOf<Self>>;
        type Two: Get<BalanceOf<Self>>;
        type Three: Get<BalanceOf<Self>>;
        type Four: Get<BalanceOf<Self>>;
        type Five: Get<BalanceOf<Self>>;
        type BaseAmount: Get<BalanceOf<Self>>;
        type TotalAmount: Get<BalanceOf<Self>>;
        type FeeAmount: Get<BalanceOf<Self>>;
        type FiveHunHMill: Get<BalanceOf<Self>>;
        type FixFeeAmount: Get<BalanceOf<Self>>;
        type OnePFouSixbil: Get<BalanceOf<Self>>;
        type Distance: Get<BalanceOf<Self>>;
        type MaxAmount: Get<BalanceOf<Self>>;
        type LLCAccount: Get<<Self as frame_system::Config>::AccountId>;
    }

    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// issue token!
        IssueToken,
        /// A lottery has been started!
        LotteryStarted,
        /// A new set of calls have been set!
        CallsUpdated,
        /// A winner has been chosen!
        Winner { winner: T::AccountId, lottery_balance: BalanceOf<T> },
        /// A ticket has been bought!
        TicketBought { who: T::AccountId, call_index: CallIndex },
    }

    #[pallet::error]
    pub enum Error<T> {
        /// A lottery has not been configured.
        NotConfigured,
        /// A lottery is already in progress.
        InProgress,
        /// A lottery has already ended.
        AlreadyEnded,
        /// The call is not valid for an open lottery.
        InvalidCall,
        /// You are already participating in the lottery with this call.
        AlreadyParticipating,
        /// Too many calls for a single lottery.
        TooManyCalls,
        /// Failed to encode calls
        EncodingFailed,

        NoneValue,

        StorageOverflow,
    }

    #[pallet::storage]
    pub(crate) type LotteryIndex<T> = StorageValue<_, u32, ValueQuery>;

    /// The configuration for the current lottery.
    #[pallet::storage]
    pub(crate) type Lottery<T: Config> =
    StorageValue<_, LotteryConfig<T::BlockNumber, BalanceOf<T>>>;

    /// Users who have purchased a ticket. (Lottery Index, Tickets Purchased)
    #[pallet::storage]
    pub(crate) type Participants<T: Config> = StorageMap<
        _,
        Twox64Concat,
        T::AccountId,
        (u32, BoundedVec<CallIndex, T::MaxCalls>),
        ValueQuery,
    >;

    /// Total number of tickets sold.
    #[pallet::storage]
    pub(crate) type TicketsCount<T> = StorageValue<_, u32, ValueQuery>;


    /// DistanceCount.
    #[pallet::storage]
    pub type DistanceCount<T> = StorageValue<_, BalanceOf<T>>;


    /// fee.
    #[pallet::storage]
    pub type Fee<T> = StorageValue<_, BalanceOf<T>>;


    ///
    /// May have residual storage from previous lotteries. Use `TicketsCount` to see which ones
    /// are actually valid ticket mappings.
    #[pallet::storage]
    pub(crate) type Tickets<T: Config> = StorageMap<_, Twox64Concat, u32, T::AccountId>;

    /// The calls stored in this pallet to be used in an active lottery if configured
    /// by `Config::ValidateCall`.
    #[pallet::storage]
    pub(crate) type CallIndices<T: Config> =
    StorageValue<_, BoundedVec<CallIndex, T::MaxCalls>, ValueQuery>;

    #[pallet::genesis_config]
    pub struct GenesisConfig<T: Config> {
        pub single_value: BalanceOf<T>,
        pub fee_value: BalanceOf<T>,
    }

    #[cfg(feature = "std")]
    impl<T: Config> Default for GenesisConfig<T> {
        fn default() -> Self {
            Self { single_value: Default::default(), fee_value: Default::default() }
        }
    }

    #[pallet::genesis_build]
    impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
        fn build(&self) {
            <DistanceCount<T>>::put(T::Distance::get());
            <Fee<T>>::put(T::FeeAmount::get());
        }
    }


    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_initialize(n: T::BlockNumber) -> Weight {
            Lottery::<T>::mutate(|mut lottery| -> Weight {
                if let Some(config) = &mut lottery {
                    let payout_block =
                        config.start.saturating_add(config.length).saturating_add(config.delay);
                    if payout_block <= n {
                        let (lottery_account, lottery_balance) = Self::pot();

                        let winner = Self::choose_account().unwrap_or(lottery_account);
                        // Not much we can do if this fails...
                        let res = T::Currency::transfer(
                            &Self::account_id(),
                            &winner,
                            lottery_balance,
                            KeepAlive,
                        );
                        debug_assert!(res.is_ok());

                        Self::deposit_event(Event::<T>::Winner { winner, lottery_balance });

                        TicketsCount::<T>::kill();

                        if config.repeat {
                            // If lottery should repeat, increment index by 1.
                            LotteryIndex::<T>::mutate(|index| *index = index.saturating_add(1));
                            // Set a new start with the current block.
                            config.start = n;
                            return T::WeightInfo::on_initialize_repeat();
                        } else {
                            // Else, kill the lottery storage.
                            *lottery = None;
                            return T::WeightInfo::on_initialize_end();
                        }
                        // We choose not need to kill Participants and Tickets to avoid a large
                        // number of writes at one time. Instead, data persists between lotteries,
                        // but is not used if it is not relevant.
                    }
                }
                return T::DbWeight::get().reads(1);
            })
        }
    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Buy a ticket to enter the lottery.
        ///
        /// This extrinsic acts as a passthrough function for `call`. In all
        /// situations where `call` alone would succeed, this extrinsic should
        /// succeed.
        ///
        /// If `call` is successful, then we will attempt to purchase a ticket,
        /// which may fail silently. To detect success of a ticket purchase, you
        /// should listen for the `TicketBought` event.
        ///
        /// This extrinsic must be called by a signed origin.
        #[pallet::weight(
        T::WeightInfo::buy_ticket()
        .saturating_add(call.get_dispatch_info().weight)
        )]
        pub fn buy_ticket(origin: OriginFor<T>, call: Box<<T as Config>::Call>) -> DispatchResult {
            let caller = ensure_signed(origin.clone())?;
            call.clone().dispatch(origin).map_err(|e| e.error)?;

            let _ = Self::do_buy_ticket(&caller, &call);
            Ok(())
        }

        /// Set calls in storage which can be used to purchase a lottery ticket.
        ///
        /// This function only matters if you use the `ValidateCall` implementation
        /// provided by this pallet, which uses storage to determine the valid calls.
        ///
        /// This extrinsic must be called by the Manager origin.
        #[pallet::weight(T::WeightInfo::set_calls(calls.len() as u32))]
        pub fn set_calls(origin: OriginFor<T>, calls: Vec<<T as Config>::Call>) -> DispatchResult {
            T::ManagerOrigin::ensure_origin(origin)?;
            ensure!(calls.len() <= T::MaxCalls::get() as usize, Error::<T>::TooManyCalls);
            if calls.is_empty() {
                CallIndices::<T>::kill();
            } else {
                let indices = Self::calls_to_indices(&calls)?;
                CallIndices::<T>::put(indices);
            }
            Self::deposit_event(Event::<T>::CallsUpdated);
            Ok(())
        }

        /// Start a lottery using the provided configuration.
        ///
        /// This extrinsic must be called by the `ManagerOrigin`.
        ///
        /// Parameters:
        ///
        /// * `price`: The cost of a single ticket.
        /// * `length`: How long the lottery should run for starting at the current block.
        /// * `delay`: How long after the lottery end we should wait before picking a winner.
        /// * `repeat`: If the lottery should repeat when completed.
        #[pallet::weight(T::WeightInfo::start_lottery())]
        pub fn start_lottery(
            origin: OriginFor<T>,
            price: BalanceOf<T>,
            length: T::BlockNumber,
            delay: T::BlockNumber,
            repeat: bool,
        ) -> DispatchResult {
            T::ManagerOrigin::ensure_origin(origin)?;
            Lottery::<T>::try_mutate(|lottery| -> DispatchResult {
                ensure!(lottery.is_none(), Error::<T>::InProgress);
                let index = LotteryIndex::<T>::get();
                let new_index = index.checked_add(1).ok_or(ArithmeticError::Overflow)?;
                let start = frame_system::Pallet::<T>::block_number();
                // Use new_index to more easily track everything with the current state.
                *lottery = Some(LotteryConfig { price, start, length, delay, repeat });
                LotteryIndex::<T>::put(new_index);
                Ok(())
            })?;
            // Make sure pot exists.
            let lottery_account = Self::account_id();
            if T::Currency::total_balance(&lottery_account).is_zero() {
                T::Currency::deposit_creating(&lottery_account, T::Currency::minimum_balance());
            }
            Self::deposit_event(Event::<T>::LotteryStarted);
            Ok(())
        }

        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn issue(
            origin: OriginFor<T>,
            dest: T::AccountId,
            user_amount: BalanceOf<T>,
        ) -> DispatchResult {
            // ensure_root(origin)?;
			info!("user_amount: {:?}", user_amount);

            let temtotal = (T::Currency::total_issuance() + user_amount) / T::BaseAmount::get();
            info!( "TotalAmount: {:?}", T::TotalAmount::get());
            info!( "total_issuance: {:?}", T::Currency::total_issuance()/T::BaseAmount::get());
            info!("temtotal: {:?}", temtotal);

            if temtotal.ge(&T::TotalAmount::get()) {
                info!("*** total_balance is >= 100_0000_0000");
                T::Currency::deposit_creating(&dest, T::TotalAmount::get() - T::Currency::total_issuance());
            } else {
                info!("*** total_balance is < 100_0000_0000");
                T::Currency::deposit_creating(&dest, user_amount);
            }


			let new_user_amount = user_amount/ T::BaseAmount::get();
            let account_llc = T::LLCAccount::get();
            let llc_total_amount = (T::Currency::total_balance(&account_llc)) / T::BaseAmount::get();
            info!("llc_total_amount:{:?}",llc_total_amount);
            let remain = T::TotalAmount::get() -  T::Currency::total_issuance() / T::BaseAmount::get();
            info!("remain:{:?}",remain);
            let consumer = T::Currency::total_issuance() / T::BaseAmount::get() - llc_total_amount;
            info!("consumer:{:?}",consumer);
            info!("Distance:{:?}",T::Distance::get());
            //发行n，(剩下总量-n)*n/1000000000
            // let llcamount = remain/T::FeeAmount::get();

            let dis = <DistanceCount<T>>::get();
            info!("dis:{:?}",dis);


            if consumer.lt(&T::OnePFouSixbil::get()) {
                if consumer.ge(&T::FiveHunHMill::get()) {
					info!("FiveHunHMill:{:?}",T::FiveHunHMill::get());
                    let llcamount = ((remain - new_user_amount) * new_user_amount /T::FeeAmount::get())*T::FixFeeAmount::get()/T::FeeAmount::get();
                    T::Currency::deposit_creating(&account_llc, llcamount * T::BaseAmount::get());
                } else {
                    match <DistanceCount<T>>::get() {
                        // Return an error if the value has not been set.
                        None => Err(Error::<T>::NoneValue)?,
                        Some(old) => {
                            if consumer.ge(&old) {
                                match <Fee<T>>::get() {
                                    None => Err(Error::<T>::NoneValue)?,
                                    Some(fee) => {
                                        let before_amount = new_user_amount - consumer % old;
                                        let after_amount = consumer % old;
                                        info!("before:{:?}",new_user_amount - consumer % old);
                                        info!("after:{:?}",consumer % old);
                                        info!("consumer >=:{:?}",T::Distance::get());
                                        let before_fee = fee;
                                        info!("--before-fee:{:?}",before_fee);
                                        let before_llcamount = ((remain - before_amount) * before_amount /T::FeeAmount::get())*fee/T::FeeAmount::get();
                                        info!("before_llcamount:{:?}",before_llcamount);
                                        T::Currency::deposit_creating(&account_llc, before_llcamount * T::BaseAmount::get());

                                        let after_fee = fee*T::Ninety::get()/T::Hundred::get();
                                        info!("--after-fee:{:?}",after_fee);
                                        let after_llcamount = ((remain - before_llcamount - after_amount) * after_amount /T::FeeAmount::get())*after_fee/T::FeeAmount::get();
                                        info!("after_llcamount:{:?}",after_llcamount);
                                        T::Currency::deposit_creating(&account_llc, after_llcamount * T::BaseAmount::get());

                                        // let llcamount = ((remain - new_user_amount) * new_user_amount /T::FeeAmount::get())*fee/T::FeeAmount::get();
                                        // T::Currency::deposit_creating(&account_llc, llcamount * T::BaseAmount::get());

                                        // Increment the value read from storage; will error in the event of overflow.
                                        let new = old.checked_add(&T::Distance::get()).ok_or(Error::<T>::StorageOverflow)?;
                                        // Update the value in storage with the incremented result.
                                        <DistanceCount<T>>::put(new);
                                        let dis = <DistanceCount<T>>::get();
                                        info!("--dis:{:?}",dis);

                                        <Fee<T>>::put(fee * T::Ninety::get() / T::Hundred::get());
                                        let dis = <Fee<T>>::get();
                                        info!("-new-Fee:{:?}",dis);

                                        let llc_total_amount = (T::Currency::total_balance(&account_llc)) / T::BaseAmount::get();
                                        info!("after_llc_total_amount:{:?}",llc_total_amount);
                                    }
                                }
                            } else {
                                match <Fee<T>>::get() {
                                    None => Err(Error::<T>::NoneValue)?,
                                    Some(fee) => {

                                        info!("**fee:{:?}",fee);
                                        let llcamount = ((remain - new_user_amount) * new_user_amount /T::FeeAmount::get())*fee/T::FeeAmount::get();
                                        T::Currency::deposit_creating(&account_llc, llcamount * T::BaseAmount::get());
                                        let dis = <DistanceCount<T>>::get();
                                        info!("**dis:{:?}",dis);

                                        let llc_total_amount = (T::Currency::total_balance(&account_llc)) / T::BaseAmount::get();
                                        info!("**_llc_total_amount:{:?}",llc_total_amount);


                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Read a value from storage.

            Self::deposit_event(Event::<T>::IssueToken);
            Ok(())
        }


        /// If a lottery is repeating, you can use this to stop the repeat.
        /// The lottery will continue to run to completion.
        ///
        /// This extrinsic must be called by the `ManagerOrigin`.
        #[pallet::weight(T::WeightInfo::stop_repeat())]
        pub fn stop_repeat(origin: OriginFor<T>) -> DispatchResult {
            T::ManagerOrigin::ensure_origin(origin)?;
            Lottery::<T>::mutate(|mut lottery| {
                if let Some(config) = &mut lottery {
                    config.repeat = false
                }
            });
            Ok(())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// The account ID of the lottery pot.
    ///
    /// This actually does computation. If you need to keep using it, then make sure you cache the
    /// value and only call this once.
    pub fn account_id() -> T::AccountId {
        T::PalletId::get().into_account()
    }

    /// Return the pot account and amount of money in the pot.
    /// The existential deposit is not part of the pot so lottery account never gets deleted.
    fn pot() -> (T::AccountId, BalanceOf<T>) {
        let account_id = Self::account_id();
        let balance =
            T::Currency::free_balance(&account_id).saturating_sub(T::Currency::minimum_balance());

        (account_id, balance)
    }

    /// Converts a vector of calls into a vector of call indices.
    fn calls_to_indices(
        calls: &[<T as Config>::Call],
    ) -> Result<BoundedVec<CallIndex, T::MaxCalls>, DispatchError> {
        let mut indices = BoundedVec::<CallIndex, T::MaxCalls>::with_bounded_capacity(calls.len());
        for c in calls.iter() {
            let index = Self::call_to_index(c)?;
            indices.try_push(index).map_err(|_| Error::<T>::TooManyCalls)?;
        }
        Ok(indices)
    }

    /// Convert a call to it's call index by encoding the call and taking the first two bytes.
    fn call_to_index(call: &<T as Config>::Call) -> Result<CallIndex, DispatchError> {
        let encoded_call = call.encode();
        if encoded_call.len() < 2 {
            Err(Error::<T>::EncodingFailed)?
        }
        return Ok((encoded_call[0], encoded_call[1]));
    }

    /// Logic for buying a ticket.
    fn do_buy_ticket(caller: &T::AccountId, call: &<T as Config>::Call) -> DispatchResult {
        // Check the call is valid lottery
        let config = Lottery::<T>::get().ok_or(Error::<T>::NotConfigured)?;
        let block_number = frame_system::Pallet::<T>::block_number();
        ensure!(
			block_number < config.start.saturating_add(config.length),
			Error::<T>::AlreadyEnded
		);
        ensure!(T::ValidateCall::validate_call(call), Error::<T>::InvalidCall);
        let call_index = Self::call_to_index(call)?;
        let ticket_count = TicketsCount::<T>::get();
        let new_ticket_count = ticket_count.checked_add(1).ok_or(ArithmeticError::Overflow)?;
        // Try to update the participant status
        Participants::<T>::try_mutate(
            &caller,
            |(lottery_index, participating_calls)| -> DispatchResult {
                let index = LotteryIndex::<T>::get();
                // If lottery index doesn't match, then reset participating calls and index.
                if *lottery_index != index {
                    *participating_calls = Default::default();
                    *lottery_index = index;
                } else {
                    // Check that user is not already participating under this call.
                    ensure!(
						!participating_calls.iter().any(|c| call_index == *c),
						Error::<T>::AlreadyParticipating
					);
                }
                participating_calls.try_push(call_index).map_err(|_| Error::<T>::TooManyCalls)?;
                // Check user has enough funds and send it to the Lottery account.
                T::Currency::transfer(caller, &Self::account_id(), config.price, KeepAlive)?;
                // Create a new ticket.
                TicketsCount::<T>::put(new_ticket_count);
                Tickets::<T>::insert(ticket_count, caller.clone());
                Ok(())
            },
        )?;

        Self::deposit_event(Event::<T>::TicketBought { who: caller.clone(), call_index });

        Ok(())
    }

    /// Randomly choose a winning ticket and return the account that purchased it.
    /// The more tickets an account bought, the higher are its chances of winning.
    /// Returns `None` if there is no winner.
    fn choose_account() -> Option<T::AccountId> {
        match Self::choose_ticket(TicketsCount::<T>::get()) {
            None => None,
            Some(ticket) => Tickets::<T>::get(ticket),
        }
    }

    /// Randomly choose a winning ticket from among the total number of tickets.
    /// Returns `None` if there are no tickets.
    fn choose_ticket(total: u32) -> Option<u32> {
        if total == 0 {
            return None;
        }
        let mut random_number = Self::generate_random_number(0);

        // Best effort attempt to remove bias from modulus operator.
        for i in 1..T::MaxGenerateRandom::get() {
            if random_number < u32::MAX - u32::MAX % total {
                break;
            }

            random_number = Self::generate_random_number(i);
        }

        Some(random_number % total)
    }

    /// Generate a random number from a given seed.
    /// Note that there is potential bias introduced by using modulus operator.
    /// You should call this function with different seed values until the random
    /// number lies within `u32::MAX - u32::MAX % n`.
    /// TODO: deal with randomness freshness
    /// https://github.com/paritytech/substrate/issues/8311
    fn generate_random_number(seed: u32) -> u32 {
        let (random_seed, _) = T::Randomness::random(&(T::PalletId::get(), seed).encode());
        let random_number = <u32>::decode(&mut random_seed.as_ref())
            .expect("secure hashes should always be bigger than u32; qed");
        random_number
    }
}
