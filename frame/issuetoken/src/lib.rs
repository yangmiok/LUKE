#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/v3/runtime/frame>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::log::info;
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use frame_support::traits::Currency;

    // use sp_runtime::traits::{AccountIdConversion, SaturatedConversion};
    pub type BalanceOf<T> = <<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
    /// The balance type of this pallet.
    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Currency: Currency<Self::AccountId>;
        type Hundred: Get<BalanceOf<Self>>;
        type Eighty: Get<BalanceOf<Self>>;
        type Seventy: Get<BalanceOf<Self>>;
        type Fifty: Get<BalanceOf<Self>>;
        type Forty: Get<BalanceOf<Self>>;
        type Thirty: Get<BalanceOf<Self>>;
        type Twenty: Get<BalanceOf<Self>>;
        type Ten: Get<BalanceOf<Self>>;
        type BaseAmount: Get<BalanceOf<Self>>;
        type MaxAmount: Get<BalanceOf<Self>>;
        type FirstPhase: Get<BalanceOf<Self>>;
        type SecendPhase: Get<BalanceOf<Self>>;
        type ThirdPhase: Get<BalanceOf<Self>>;
        type ForthPhase: Get<BalanceOf<Self>>;
        type LLCAccount: Get<<Self as frame_system::Config>::AccountId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub (super) trait Store)]
    pub struct Pallet<T>(_);

    // The pallet's runtime storage items.
    // https://docs.substrate.io/v3/runtime/storage


    // Pallets use events to inform users when important changes are made.
    // https://docs.substrate.io/v3/runtime/events-and-errors
    #[pallet::event]
    #[pallet::generate_deposit(pub (super) fn deposit_event)]
    pub enum Event<T: Config> {
        /// Event documentation should end with an array that provides descriptive names for event
        /// parameters. [something, who]
        SomethingStored(BalanceOf<T>, T::AccountId),

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
        /// An example dispatchable that takes a singles value as a parameter, writes the value to
        /// storage and emits an event. This function must be dispatched by a signed extrinsic.
        #[pallet::weight(10_000 + T::DbWeight::get().writes(1))]
        pub fn issue(origin: OriginFor<T>,
                     dest: T::AccountId,
                     user_amount: BalanceOf<T>,
        ) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://docs.substrate.io/v3/runtime/origins
            ensure_root(origin)?;

            // let who = ensure_signed(origin)?;
            let account_llc = T::LLCAccount::get();
            info!("alice_acc from runtime: {:#?}", account_llc.clone());
            info!( "user_amount:  {:?}", user_amount);
            info!( "before total_issuance: {:?}", T::Currency::total_issuance());

            // T::Currency::deposit_creating(&dest, user_amount * T::BaseAmount::get());
            T::Currency::deposit_creating(&dest, user_amount * T::BaseAmount::get());
            info!( "llc total_balance:  {:?}", T::Currency::total_balance(&account_llc));
            info!( "---- 1.0:  {:?}", user_amount * T::BaseAmount::get());
            if T::Currency::total_balance(&account_llc).lt(&T::MaxAmount::get()) {
                info!("llc total_balance is <= 25_000_000_000");
                // T::Currency::deposit_creating(&account_llc, user_amount * T::BaseAmount::get());
                if T::Currency::total_issuance().lt(&T::FirstPhase::get()) {
                    info!("------FirstPhase is <= 20_000_000_000");
                    info!( "---- 0.8:  {:?}", user_amount*T::Eighty::get()/T::Hundred::get()* T::BaseAmount::get());

                    let newamount = T::Currency::total_balance(&account_llc)+user_amount*T::Eighty::get()/T::Hundred::get()* T::BaseAmount::get();
                    info!( "---- newamount:  {:?}", newamount);

                    if newamount.ge(&T::MaxAmount::get()){
                        info!( "---- ge MaxAmount:  {:?}", newamount);
                        T::Currency::deposit_creating(&account_llc, T::MaxAmount::get()-T::Currency::total_balance(&account_llc));

                    }else{
                        info!( "----lt MaxAmount:  {:?}", newamount);
                        T::Currency::deposit_creating(&account_llc, user_amount*T::Eighty::get()/T::Hundred::get()* T::BaseAmount::get());

                    }

                    
                } else if T::Currency::total_issuance().lt(&T::SecendPhase::get()) {
                    info!("------SecendPhase is <= 40_000_000_000");
                    info!( "---- 0.5:  {:?}", user_amount*T::Fifty::get()/T::Hundred::get()* T::BaseAmount::get());

                    let newamount = T::Currency::total_balance(&account_llc)+user_amount*T::Fifty::get()/T::Hundred::get()* T::BaseAmount::get();
                    info!( "---- newamount:  {:?}", newamount);

                    if newamount.ge(&T::MaxAmount::get()){
                        info!( "---- ge MaxAmount:  {:?}", newamount);
                        T::Currency::deposit_creating(&account_llc, T::MaxAmount::get()-T::Currency::total_balance(&account_llc));

                    }else{
                        info!( "----lt MaxAmount:  {:?}", newamount);
                        T::Currency::deposit_creating(&account_llc, user_amount*T::Fifty::get()/T::Hundred::get()* T::BaseAmount::get());

                    }


                } else if T::Currency::total_issuance().lt(&T::ThirdPhase::get()) {
                    info!("------ThirdPhase is <= 60_000_000_000");
                    info!( "---- 0.2:  {:?}", user_amount*T::Twenty::get()/T::Hundred::get()* T::BaseAmount::get());
                    let newamount = T::Currency::total_balance(&account_llc)+user_amount*T::Twenty::get()/T::Hundred::get()* T::BaseAmount::get();
                    info!( "---- newamount:  {:?}", newamount);

                    if newamount.ge(&T::MaxAmount::get()){
                        info!( "---- ge MaxAmount:  {:?}", newamount);
                        T::Currency::deposit_creating(&account_llc, T::MaxAmount::get()-T::Currency::total_balance(&account_llc));

                    }else{
                        info!( "----lt MaxAmount:  {:?}", newamount);
                        T::Currency::deposit_creating(&account_llc, user_amount*T::Twenty::get()/T::Hundred::get()* T::BaseAmount::get());

                    }

                } else {
                    info!("------ForthPhase is <= 80_000_000_000");
                    info!( "---- 0.1:  {:?}", user_amount*T::Ten::get()/T::Hundred::get()* T::BaseAmount::get());
                    let newamount = T::Currency::total_balance(&account_llc)+user_amount*T::Ten::get()/T::Hundred::get()* T::BaseAmount::get();
                    info!( "---- newamount:  {:?}", newamount);

                    if newamount.ge(&T::MaxAmount::get()){
                        info!( "---- ge MaxAmount:  {:?}", newamount);
                        T::Currency::deposit_creating(&account_llc, T::MaxAmount::get()-T::Currency::total_balance(&account_llc));

                    }else{
                        info!( "----lt MaxAmount:  {:?}", newamount);
                        T::Currency::deposit_creating(&account_llc, user_amount*T::Ten::get()/T::Hundred::get()* T::BaseAmount::get());

                    }                    

                }
            } else {
                info!("llc total_balance is > 25_000_000_000");
            }
            info!( "end total_issuance: {:?}", T::Currency::total_issuance());
            Self::deposit_event(Event::SomethingStored(user_amount, account_llc));
            // Return a successful DispatchResultWithPostInfo
            Ok(())
        }
    }
}