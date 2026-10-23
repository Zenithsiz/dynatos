//! [`SignalGetCloned`]

use crate::{SignalWith, effect};

/// Auto trait implemented for all signals that want a default implementation of [`SignalGetCloned`]
///
/// If you are writing a signal type with type parameters, you should manually implement
/// this auto trait, since those type parameters might disable it
pub auto trait SignalGetClonedDefaultImpl {}

/// Types which may be cloned by [`SignalGetCloned`]
pub trait SignalGetClone: Sized {
	type Value: 'static;
	fn clone_value(self) -> Self::Value;
}

impl<T: Clone + 'static> SignalGetClone for &'_ T {
	type Value = T;

	fn clone_value(self) -> Self::Value {
		self.clone()
	}
}
impl<T: Clone + 'static> SignalGetClone for Option<&'_ T> {
	type Value = Option<T>;

	fn clone_value(self) -> Self::Value {
		self.cloned()
	}
}

/// Signal cloned
pub trait SignalGetCloned {
	/// Value type
	type Value;

	/// Gets the signal value, by cloning it.
	#[track_caller]
	fn get_cloned(&self) -> Self::Value;

	/// Gets the signal value, by cloning it without gathering dependencies.
	#[track_caller]
	fn get_cloned_no_dep(&self) -> Self::Value {
		effect::with_no_dep(|| self.get_cloned())
	}
}

impl<S, T> SignalGetCloned for S
where
	S: for<'a> SignalWith<Value<'a>: SignalGetClone<Value = T>> + 'static + SignalGetClonedDefaultImpl,
{
	type Value = T;

	fn get_cloned(&self) -> Self::Value {
		self.with(|value| value.clone_value())
	}
}
