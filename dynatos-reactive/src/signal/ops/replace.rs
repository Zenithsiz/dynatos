//! [`SignalReplace`]

/// Signal replace
pub trait SignalReplace<Value> {
	/// Replaces the signal value, returning the previous value
	fn replace(&self, new_value: Value) -> Value;
}
