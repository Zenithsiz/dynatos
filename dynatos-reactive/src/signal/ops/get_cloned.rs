//! [`SignalGetCloned`]

// Imports
use crate::SignalWith;

/// Types which may be cloned by [`SignalGetCloned`]
pub trait SignalGetClone<T>: Sized {
	fn clone_value(self) -> T;
}

impl<T: Clone> SignalGetClone<T> for &'_ T {
	fn clone_value(self) -> T {
		self.clone()
	}
}
impl<T: Clone> SignalGetClone<Option<T>> for Option<&'_ T> {
	fn clone_value(self) -> Option<T> {
		self.cloned()
	}
}

/// Signal cloned
pub trait SignalGetCloned<T> {
	/// Gets the signal value, by cloning it.
	fn get_cloned(&self) -> T;
}

impl<S, T> SignalGetCloned<T> for S
where
	S: SignalWith,
	for<'a> S::Value<'a>: SignalGetClone<T>,
{
	#[track_caller]
	fn get_cloned(&self) -> T {
		self.with(|value| value.clone_value())
	}
}
