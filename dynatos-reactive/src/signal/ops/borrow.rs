//! [`SignalBorrow`]

// Imports
use crate::effect;

/// Signal borrow
pub trait SignalBorrow {
	/// Reference type
	type Ref<'a>
	where
		Self: 'a;

	/// Borrows the signal value
	#[track_caller]
	fn borrow(&self) -> Self::Ref<'_>;

	/// Borrows the signal value without gathering dependencies
	// TODO: Allow using a different reference than `Self::Ref`?
	#[track_caller]
	fn borrow_no_dep(&self) -> Self::Ref<'_> {
		effect::with_no_dep(|| self.borrow())
	}
}
