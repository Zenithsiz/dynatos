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

/// Signal set
pub trait SignalSet<Value> {
	/// Sets the signal value
	fn set(&self, new_value: Value);
}

impl<S, T> SignalSet<T> for S
where
	S: SignalUpdate,
	for<'a> S::Value<'a>: SignalSetWith<T>,
{
	#[track_caller]
	fn set(&self, new_value: T) {
		self.update(|value| SignalSetWith::set_value(value, new_value));
	}
}
