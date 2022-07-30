#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
		traits::{Currency, ReservableCurrency},
		BoundedVec,
	};
	use frame_system::pallet_prelude::*;

	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub type BalanceOf<T> = <<T as Config>::Token as Currency<AccountIdOf<T>>>::Balance;

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type VecBound: Get<u32>;
		type Token: ReservableCurrency<Self::AccountId>;
		type ReserveAmount: Get<BalanceOf<Self>>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ClaimCreated(T::AccountId, BoundedVec<u8, T::VecBound>),
		ClaimRevoked(T::AccountId, BoundedVec<u8, T::VecBound>),
	}

	#[pallet::error]
	pub enum Error<T> {
		ProofAlreadyClaimed,
		NoSuchProof,
		NotProofOwner,
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::storage]
	pub(super) type Proofs<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		BoundedVec<u8, T::VecBound>,
		(T::AccountId, T::BlockNumber),
	>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(1_000)]
		pub fn create_claim(
			origin: OriginFor<T>,
			proof: BoundedVec<u8, T::VecBound>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(!Proofs::<T>::contains_key(&proof), Error::<T>::ProofAlreadyClaimed);

			T::Token::reserve(&sender, 1_000u32.into())?;

			let current_block = <frame_system::Pallet<T>>::block_number();

			Proofs::<T>::insert(&proof, (&sender, current_block));

			Self::deposit_event(Event::ClaimCreated(sender, proof));

			Ok(())
		}

		#[pallet::weight(1_000)]
		pub fn revoke_claim(
			origin: OriginFor<T>,
			proof: BoundedVec<u8, T::VecBound>,
		) -> DispatchResult {
			let sender = ensure_signed(origin)?;

			ensure!(Proofs::<T>::contains_key(&proof), Error::<T>::NoSuchProof);

			let (owner, _) = Proofs::<T>::get(&proof)
				.expect("Already checked that value exists; so it is safe to unwrap; qed");

			ensure!(sender == owner, Error::<T>::NotProofOwner);

			// Attempt to unreserve the funds from the user. We expect that they should
			// have at least this much reserved because we reserved it earlier.
			// If for some reason there isn't enough reserved, it's the user's problem :)
			let _ = T::Token::unreserve(&sender, 1_000u32.into());

			Proofs::<T>::remove(&proof);

			Self::deposit_event(Event::ClaimRevoked(sender, proof));

			Ok(())
		}
	}
}
