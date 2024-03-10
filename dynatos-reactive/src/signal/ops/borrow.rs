//! [`SignalBorrow`]

/// Signal borrow
pub trait SignalBorrow {
	/// Reference type
	type Ref<'a>
	where
		Self: 'a;

	/// Borrows the signal value
	fn borrow(&self) -> Self::Ref<'_>;
}
