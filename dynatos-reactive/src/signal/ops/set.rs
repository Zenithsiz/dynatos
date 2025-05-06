//! [`SignalSet`]

// Imports
use crate::SignalUpdate;

/// Types which may be set by [`SignalSet`]
pub trait SignalSetWith<T>: Sized {
	fn set_value(self, new_value: T);
}

impl<T> SignalSetWith<T> for &'_ mut T {
	fn set_value(self, new_value: T) {
		*self = new_value;
	}
}
impl<T> SignalSetWith<T> for &'_ mut Option<T> {
	fn set_value(self, new_value: T) {
		*self = Some(new_value);
	}
}

/// Auto trait implemented for all signals that want a default implementation of `SignalSet`
///
/// If you are writing a signal type with type parameters, you should manually implement
/// this auto trait, since those type parameters might disable it (although this only mattering for signals)
pub auto trait SignalSetDefaultImpl {}

/// Signal set
pub trait SignalSet<Value> {
	/// Sets the signal value
	fn set(&self, new_value: Value);
}

impl<S, T> SignalSet<T> for S
where
	S: for<'a> SignalUpdate<Value<'a>: SignalSetWith<T>> + SignalSetDefaultImpl,
{
	#[track_caller]
	fn set(&self, new_value: T) {
		self.update(|value| SignalSetWith::set_value(value, new_value));
	}
}
