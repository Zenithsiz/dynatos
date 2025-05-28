//! [`SignalReplace`]

/// Signal replace
pub trait SignalReplace<T> {
	type Value;

	/// Replaces the signal value, returning the previous value
	#[track_caller]
	fn replace(&self, new_value: T) -> Self::Value;

	/// Replaces the signal value, returning the previous value without triggering any dependencies.
	#[track_caller]
	fn replace_raw(&self, new_value: T) -> Self::Value;
}
