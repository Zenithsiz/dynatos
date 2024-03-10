//! [`SignalBorrowMut`]

/// Signal borrow
pub trait SignalBorrowMut {
	/// Mutable reference type
	type RefMut<'a>
	where
		Self: 'a;

	/// Borrows the signal value mutably
	fn borrow_mut(&self) -> Self::RefMut<'_>;
}
