//! [`SignalGet`]

// Imports
use crate::SignalWith;

/// Types which may be copied by [`SignalGet`]
pub trait SignalGetCopy<T>: Sized {
	fn copy_value(self) -> T;
}

impl<T: Copy> SignalGetCopy<T> for &'_ T {
	fn copy_value(self) -> T {
		*self
	}
}
impl<T: Copy> SignalGetCopy<Option<T>> for Option<&'_ T> {
	fn copy_value(self) -> Option<T> {
		self.copied()
	}
}

/// Signal get
pub trait SignalGet<T> {
	/// Gets the signal value, by copying it.
	fn get(&self) -> T;
}

impl<S, T> SignalGet<T> for S
where
	S: SignalWith,
	for<'a> S::Value<'a>: SignalGetCopy<T>,
{
	#[track_caller]
	fn get(&self) -> T {
		self.with(|value| value.copy_value())
	}
}
