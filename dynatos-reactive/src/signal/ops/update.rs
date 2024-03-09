//! [`SignalUpdate`]

/// Signal update
pub trait SignalUpdate {
	/// Value type
	type Value<'a>: ?Sized;

	/// Updates the signal value
	fn update<F, O>(&self, f: F) -> O
	where
		F: for<'a> FnOnce(Self::Value<'a>) -> O;
}
