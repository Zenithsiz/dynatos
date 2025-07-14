//! [`SignalWith`]

// Imports
use {super::SignalBorrow, crate::effect, core::ops::Deref};

/// Auto trait implemented for all signals that want a default implementation of `SignalWith`
///
/// If you are writing a signal type with type parameters, you should manually implement
/// this auto trait, since those type parameters might disable it
pub auto trait SignalWithDefaultImpl {}

/// Signal with
pub trait SignalWith {
	/// Value type
	type Value<'a>;

	/// Uses the signal value
	#[track_caller]
	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O;

	/// Uses the signal value without gathering dependencies
	#[track_caller]
	fn with_raw<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		effect::with_raw(|| self.with(f))
	}
}

impl<S, T> SignalWith for S
where
	S: for<'a> SignalBorrow<Ref<'a>: Deref<Target = T>> + 'static + SignalWithDefaultImpl,
	T: ?Sized + 'static,
{
	type Value<'a> = &'a T;

	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O,
	{
		let borrow = self.borrow();
		f(&borrow)
	}
}
