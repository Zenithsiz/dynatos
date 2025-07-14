//! [`SignalBorrowMut`]

// Imports
use crate::effect;

/// Signal borrow
pub trait SignalBorrowMut {
	/// Mutable reference type
	type RefMut<'a>
	where
		Self: 'a;

	/// Borrows the signal value mutably
	#[track_caller]
	fn borrow_mut(&self) -> Self::RefMut<'_>;

	/// Borrows the signal value mutably without updating dependencies
	// TODO: Better name than `_raw`?
	// TODO: Allow using a different reference than `Self::RefMut`?
	#[track_caller]
	fn borrow_mut_raw(&self) -> Self::RefMut<'_> {
		effect::with_raw(|| self.borrow_mut())
	}
}
