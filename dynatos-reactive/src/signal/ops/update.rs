//! [`SignalUpdate`]

// Imports
use {super::SignalBorrowMut, core::ops::DerefMut};

/// Auto trait implemented for all signals that want a default implementation of `SignalUpdate`
///
/// If you are writing a signal type with type parameters, you should manually implement
/// this auto trait, since those type parameters might disable it
pub auto trait SignalUpdateDefaultImpl {}

/// Signal update
pub trait SignalUpdate {
	/// Value type
	type Value<'a>;

	/// Updates the signal value
	#[track_caller]
	fn update<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O;

	/// Updates the signal value without updating dependencies
	#[track_caller]
	fn update_raw<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O;
}

impl<S, T> SignalUpdate for S
where
	S: for<'a> SignalBorrowMut<RefMut<'a>: DerefMut<Target = T>> + 'static + SignalUpdateDefaultImpl,
	T: ?Sized + 'static,
{
	type Value<'a> = &'a mut T;

	fn update<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let mut borrow = self.borrow_mut();
		f(&mut borrow)
	}

	fn update_raw<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let mut borrow = self.borrow_mut_raw();
		f(&mut borrow)
	}
}
