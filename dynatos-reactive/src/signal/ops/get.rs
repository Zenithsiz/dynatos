//! [`SignalGet`]

use crate::{SignalWith, effect};

/// Auto trait implemented for all signals that want a default implementation of [`SignalGet`]
///
/// If you are writing a signal type with type parameters, you should manually implement
/// this auto trait, since those type parameters might disable it
pub auto trait SignalGetDefaultImpl {}

/// Types which may be copied by [`SignalGet`]
pub trait SignalGetCopy: Sized {
	type Value: 'static;
	fn copy_value(self) -> Self::Value;
}

impl<T: Copy + 'static> SignalGetCopy for &'_ T {
	type Value = T;

	fn copy_value(self) -> Self::Value {
		*self
	}
}
impl<T: Copy + 'static> SignalGetCopy for Option<&'_ T> {
	type Value = Option<T>;

	fn copy_value(self) -> Self::Value {
		self.copied()
	}
}

/// Signal get
pub trait SignalGet {
	/// Value type
	type Value;

	/// Gets the signal value, by copying it.
	#[track_caller]
	fn get(&self) -> Self::Value;

	/// Gets the signal value, by copying it without gathering dependencies.
	#[track_caller]
	fn get_no_dep(&self) -> Self::Value {
		effect::with_no_dep(|| self.get())
	}
}

impl<S, T> SignalGet for S
where
	S: for<'a> SignalWith<Value<'a>: SignalGetCopy<Value = T>> + 'static + SignalGetDefaultImpl,
{
	type Value = T;

	fn get(&self) -> Self::Value {
		self.with(|value| value.copy_value())
	}
}
