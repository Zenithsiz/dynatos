//! [`SignalBorrow`]

/// Signal borrow
pub trait SignalBorrow {
	/// Reference type
	type Ref<'a>
	where
		Self: 'a;

	/// Borrows the signal value
	fn borrow(&self) -> Self::Ref<'_>;

	/// Borrows the signal value without adding a dependency
	// TODO: Better name than `_raw`?
	// TODO: Allow using a different reference than `Self::Ref`?
	fn borrow_raw(&self) -> Self::Ref<'_>;
}
