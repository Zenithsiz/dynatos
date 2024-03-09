//! [`SignalWith`]

/// Signal with
pub trait SignalWith {
	/// Value type
	type Value<'a>: ?Sized;

	/// Uses the signal value
	fn with<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O;
}
