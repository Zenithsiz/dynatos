//! [`SignalReplace`]

/// Signal replace
pub trait SignalReplace<T> {
	type Value;

	/// Replaces the signal value, returning the previous value
	fn replace(&self, new_value: T) -> Self::Value;

	/// Replaces the signal value, returning the previous value without triggering any dependencies.
	fn replace_raw(&self, new_value: T) -> Self::Value;
}
